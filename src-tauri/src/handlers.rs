use notify::{EventKind, RecursiveMode, Watcher};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::env;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex as StdMutex};
use tauri::{Emitter, Manager};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;
use tauri_plugin_updater::{Update, UpdaterExt};
use tokio::io::{AsyncReadExt, BufReader as AsyncBufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::Mutex;

// MCP module imports
use crate::mcp::{
    McpManager, McpPromptGetRequest, McpResourceReadRequest, McpServerConfig, McpToolCallRequest,
};

// Struct to hold the shell process with its event receiver
struct ShellProcess {
    _receiver: tauri::async_runtime::Receiver<CommandEvent>,
    child: tauri_plugin_shell::process::CommandChild,
}

// Global state to manage agent processes and callbacks
static AGENT_PROCESSES: std::sync::LazyLock<Arc<Mutex<HashMap<String, ShellProcess>>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

static LISTENING_TASKS: std::sync::LazyLock<
    Arc<Mutex<HashMap<String, tokio::task::JoinHandle<()>>>>,
> = std::sync::LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

const APP_UPDATE_PROGRESS_EVENT: &str = "app_update_progress";
const RAVEN_ACP_ENTRY_RELATIVE_PATH: &str = "../packages/raven-acp/index.ts";
const RAVEN_ACP_PACKAGE_NAME: &str = "@raven/raven-acp";

fn get_raven_acp_dev_entry_path() -> Result<PathBuf, serde_json::Value> {
    let cwd = env::current_dir().map_err(|e| {
        serde_json::json!({
            "code": 21,
            "message": format!("Failed to resolve current working directory: {}", e)
        })
    })?;

    let entry_path = cwd.join(RAVEN_ACP_ENTRY_RELATIVE_PATH);

    if entry_path.exists() {
        return Ok(entry_path);
    }

    Err(serde_json::json!({
        "code": 22,
        "message": format!("Failed to locate dev raven-acp entrypoint at {}", RAVEN_ACP_ENTRY_RELATIVE_PATH)
    }))
}

#[derive(Default)]
pub struct PendingAppUpdate(pub StdMutex<Option<Update>>);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateMetadata {
    version: String,
    current_version: String,
    notes: Option<String>,
    pub_date: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppUpdateProgressPayload {
    event: String,
    content_length: Option<u64>,
    chunk_length: Option<usize>,
    downloaded: Option<u64>,
}

fn to_update_metadata(update: &Update) -> AppUpdateMetadata {
    AppUpdateMetadata {
        version: update.version.clone(),
        current_version: update.current_version.clone(),
        notes: update.body.clone(),
        pub_date: update.date.map(|date| date.to_string()),
    }
}

fn emit_app_update_progress(
    window: &tauri::Window,
    event: &str,
    content_length: Option<u64>,
    chunk_length: Option<usize>,
    downloaded: Option<u64>,
) {
    let _ = window.emit(
        APP_UPDATE_PROGRESS_EVENT,
        AppUpdateProgressPayload {
            event: event.to_string(),
            content_length,
            chunk_length,
            downloaded,
        },
    );
}

#[tauri::command]
pub async fn check_app_update(
    app: tauri::AppHandle,
    pending_update: tauri::State<'_, PendingAppUpdate>,
) -> Result<Option<AppUpdateMetadata>, String> {
    let update = app
        .updater()
        .map_err(|e| format!("Failed to create updater: {}", e))?
        .check()
        .await
        .map_err(|e| format!("Failed to check for updates: {}", e))?;
    let metadata = update.as_ref().map(to_update_metadata);

    {
        let mut pending = pending_update
            .0
            .lock()
            .map_err(|_| "Failed to access pending update state".to_string())?;
        *pending = update;
    }

    Ok(metadata)
}

#[tauri::command]
pub async fn install_app_update(
    app: tauri::AppHandle,
    window: tauri::Window,
    pending_update: tauri::State<'_, PendingAppUpdate>,
) -> Result<(), String> {
    let update = {
        let mut pending = pending_update
            .0
            .lock()
            .map_err(|_| "Failed to access pending update state".to_string())?;
        pending
            .take()
            .ok_or_else(|| "There is no pending app update to install".to_string())?
    };

    let mut started = false;
    let downloaded = Arc::new(StdMutex::new(0u64));
    let progress_downloaded = downloaded.clone();
    let finished_downloaded = downloaded.clone();
    let result = update
        .download_and_install(
            |chunk_length, content_length| {
                let mut downloaded = progress_downloaded
                    .lock()
                    .expect("download progress mutex poisoned");
                *downloaded += chunk_length as u64;

                if !started {
                    emit_app_update_progress(
                        &window,
                        "started",
                        content_length,
                        None,
                        Some(*downloaded),
                    );
                    started = true;
                }

                emit_app_update_progress(
                    &window,
                    "progress",
                    content_length,
                    Some(chunk_length),
                    Some(*downloaded),
                );
            },
            || {
                let downloaded = finished_downloaded
                    .lock()
                    .expect("download progress mutex poisoned");
                emit_app_update_progress(&window, "finished", None, None, Some(*downloaded));
            },
        )
        .await;

    match result {
        Ok(()) => {
            {
                let mut pending = pending_update
                    .0
                    .lock()
                    .map_err(|_| "Failed to access pending update state".to_string())?;
                *pending = None;
            }

            let downloaded = downloaded
                .lock()
                .map_err(|_| "Failed to access download progress state".to_string())?;
            emit_app_update_progress(&window, "installed", None, None, Some(*downloaded));

            if !cfg!(target_os = "windows") {
                app.restart();
            }

            Ok(())
        }
        Err(error) => {
            let mut pending = pending_update
                .0
                .lock()
                .map_err(|_| "Failed to access pending update state".to_string())?;
            *pending = Some(update);
            Err(format!("Failed to install update: {}", error))
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct ModelSettings {
    #[serde(alias = "model")]
    model: String,
    #[serde(alias = "baseUrl")]
    base_url: String,
    #[serde(alias = "apiKey")]
    api_key: String,
}

#[tauri::command]
pub async fn acp_initialize(
    agent: &str,
    settings: Option<ModelSettings>,
    app: tauri::AppHandle,
    window: tauri::Window,
) -> Result<serde_json::Value, serde_json::Value> {
    println!(
        "acp_initialize called with agent: {}, settings: {:?}",
        agent, settings
    );
    let agent_parts: Vec<&str> = agent.split("::").collect();
    let agent_name = agent_parts.first().unwrap_or(&agent);

    let mut args = vec!["x".into()];
    // Create environment variables map based on agent type
    let mut env_vars: HashMap<String, String> = HashMap::new();

    let app_config_dir = app.path().app_data_dir().unwrap();

    let package_name = match *agent_name {
        "qwen" => {
            args.push("@qwen-code/qwen-code".into());
            if let Some(ms) = settings {
                args.extend(["--auth-type".into(), "openai".into()]);
                env_vars.insert("OPENAI_API_KEY".into(), ms.api_key);
                env_vars.insert("OPENAI_BASE_URL".into(), ms.base_url);
                env_vars.insert("OPENAI_MODEL".into(), ms.model);
            }
            args.push("--acp".into());
            "@qwen-code/qwen-code"
        }
        "codex" => {
            args.push("@zed-industries/codex-acp".into());
            if let Some(ms) = settings {
                args.extend([
                    "-c".into(),
                    "model_provider=x".into(),
                    "-c".into(),
                    "model_providers.x.name=x".into(),
                    "-c".into(),
                    format!("model_providers.x.base_url={}", ms.base_url),
                    "-c".into(),
                    "model_providers.x.env_key=X_API_KEY".into(),
                    "-c".into(),
                    "model_providers.x.wire_api=chat".into(),
                    "-c".into(),
                    "model_providers.x.requires_openai_auth=false".into(),
                    "-c".into(),
                    "model_provider=x".into(),
                    "-c".into(),
                    format!("model={}", ms.model),
                ]);
                env_vars.insert("X_API_KEY".into(), ms.api_key);
            }
            "@zed-industries/codex-acp"
        }
        "claude" => {
            args.push("@zed-industries/claude-code-acp".into());
            if let Some(ms) = settings {
                env_vars.insert("ANTHROPIC_BASE_URL".into(), ms.base_url);
                env_vars.insert("ANTHROPIC_MODEL".into(), ms.model);
                env_vars.insert("ANTHROPIC_AUTH_TOKEN".into(), ms.api_key);
                env_vars.insert("API_TIMEOUT_MS".into(), "600000".into());
                env_vars.insert(
                    "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".into(),
                    "1".into(),
                );
            }
            "@zed-industries/claude-code-acp"
        }
        "gemini" => {
            args.push("@google/gemini-cli".into());
            if let Some(ms) = settings {
                env_vars.insert("GEMINI_MODEL".into(), ms.model);
                env_vars.insert("GOOGLE_GEMINI_BASE_URL".into(), ms.base_url);
                env_vars.insert("GEMINI_API_KEY".into(), ms.api_key);
            }
            args.push("--acp".into());
            "@google/gemini-cli"
        }
        "opencode" => {
            args.push("opencode-ai".into());
            args.push("acp".into());
            if let Some(ms) = settings {
                let model_name = ms.model.clone();
                let config = serde_json::json!({
                    "$schema": "https://opencode.ai/config.json",
                    "model": format!("local/{}", ms.model),
                    "provider": {
                        "local": {
                            "models": {
                                (model_name): {}
                            },
                            "options": {
                                "apiKey": ms.api_key,
                                "baseURL": ms.base_url,
                            }
                        }
                    }
                });
                env_vars.insert("OPENCODE_CONFIG_CONTENT".into(), config.to_string());
            }
            "opencode-ai"
        }
        "raven" => {
            if tauri::is_dev() {
                let entry_path = get_raven_acp_dev_entry_path()?;
                args = vec!["run".into(), entry_path.to_string_lossy().to_string()];
            } else {
                args.push(RAVEN_ACP_PACKAGE_NAME.into());
            }
            args.extend(["--transport".into(), "acp".into()]);

            if let Some(ms) = settings {
                if !ms.api_key.is_empty() {
                    env_vars.insert("RAVEN_ACP_API_KEY".into(), ms.api_key.clone());
                }
                if !ms.base_url.is_empty() {
                    env_vars.insert("RAVEN_ACP_BASE_URL".into(), ms.base_url);
                }
                if !ms.model.is_empty() {
                    env_vars.insert("RAVEN_ACP_MODEL".into(), ms.model);
                }
            }

            RAVEN_ACP_PACKAGE_NAME
        }
        _ => {
            return Err(serde_json::json!({
                "code": 9,
                "message": format!("Unknown agent type: {}", agent_name)
            }));
        }
    };

    // println!("Command args: {:?}", args);
    // println!("Environment vars: {:?}", env_vars);

    // Check if package is already installed
    let is_installed = check_package_installed(package_name, &app_config_dir, &app)
        .await
        .unwrap_or(false);

    if is_installed {
        println!("Package {} is already installed", package_name);

        // Get the installed version
        let installed_version =
            match get_installed_version(package_name, &app_config_dir, &app).await {
                Ok(Some(v)) => v,
                Ok(None) => "unknown".to_string(),
                Err(e) => {
                    println!("Failed to get installed version: {:?}", e);
                    "unknown".to_string()
                }
            };

        // Emit already_installed status without checking for updates
        // The frontend will call check_for_updates separately when mounted
        let event_name = format!("package_install_status::{}", agent_name);
        let _ = window.emit(
            &event_name,
            serde_json::json!({
                "package": package_name,
                "status": "already_installed",
                "message": format!("{} v{} is installed", package_name, installed_version),
                "current_version": installed_version
            }),
        );
    } else {
        // Run bun install -g for the package before starting the agent
        println!("Installing package: {}", package_name);

        let event_name = format!("package_install_status::{}", agent_name);
        let _ = window.emit(
            &event_name,
            serde_json::json!({
                "package": package_name,
                "status": "installing",
                "message": format!("Installing {}...", package_name)
            }),
        );

        let install_command = app.shell().sidecar("bun").map_err(|e| {
            let _ = window.emit(
                "package_install_status",
                serde_json::json!({
                    "package": package_name,
                    "status": "error",
                    "message": format!("Failed to load bun sidecar: {}", e)
                }),
            );
            serde_json::json!({
                "code": 10,
                "message": format!("Failed to load bun sidecar for install: {}", e)
            })
        })?;

        let (mut install_rx, _install_child) = install_command
            .args(["install", "-g", package_name])
            .env("BUN_INSTALL", app_config_dir.clone())
            .spawn()
            .map_err(|e| {
                let _ = window.emit(
                    &event_name,
                    serde_json::json!({
                        "package": package_name,
                        "status": "error",
                        "message": format!("Failed to run bun install: {}", e)
                    }),
                );
                serde_json::json!({
                    "code": 11,
                    "message": format!("Failed to run bun install: {}", e)
                })
            })?;

        // Wait for the installation process to complete
        let mut install_failed = false;
        while let Some(event) = install_rx.recv().await {
            match event {
                CommandEvent::Stderr(line) => {
                    let text = String::from_utf8_lossy(&line);
                    println!("Install stderr: {}", text);
                    if text.contains("error") || text.contains("failed") {
                        install_failed = true;
                    }
                }
                CommandEvent::Terminated(payload) => {
                    if let Some(code) = payload.code {
                        if code != 0 {
                            install_failed = true;
                        }
                    }
                    break;
                }
                _ => {}
            }
        }

        if install_failed {
            let _ = window.emit(
                &event_name,
                serde_json::json!({
                    "package": package_name,
                    "status": "error",
                    "message": format!("Failed to install {}", package_name)
                }),
            );
            return Err(serde_json::json!({
                "code": 20,
                "message": format!("Failed to install {}", package_name)
            }));
        }

        println!("Installation command completed for {}", package_name);

        // Get the installed version
        let installed_version = get_installed_version(package_name, &app_config_dir, &app)
            .await
            .ok()
            .flatten();

        let message = if let Some(ref v) = installed_version {
            format!("Successfully installed {} v{}", package_name, v)
        } else {
            format!("Successfully installed {}", package_name)
        };

        // Emit success event
        let _ = window.emit(
            &event_name,
            serde_json::json!({
                "package": package_name,
                "status": "success",
                "message": message,
                "current_version": installed_version
            }),
        );
    }

    // Create the sidecar command with environment variables
    let command = app.shell().sidecar("bun").map_err(|e| {
        serde_json::json!({
            "code": 10,
            "message": format!("Failed to load bun sidecar: {}", e)
        })
    })?;

    // Set environment variables and config (chain the env calls)
    let mut command = command.env("BUN_INSTALL", app_config_dir);
    for (key, value) in &env_vars {
        command = command.env(key, value);
    }

    let (receiver, child) = command.args(&args).spawn().map_err(|e| {
        serde_json::json!({
            "code": 1,
            "message": format!("Failed to start {} process: {}", agent_name, e)
        })
    })?;

    // Store the process for later communication
    {
        let mut processes = AGENT_PROCESSES.lock().await;
        processes.insert(
            agent.to_string(),
            ShellProcess {
                _receiver: receiver,
                child,
            },
        );
    }

    println!("Agent {} initialized successfully", agent);
    Ok(serde_json::json!({
        "code": 0,
    }))
}

#[tauri::command]
pub async fn acp_send_message(
    agent: &str,
    message: serde_json::Value,
) -> Result<serde_json::Value, serde_json::Value> {
    let mut processes = AGENT_PROCESSES.lock().await;
    let shell_process = processes.get_mut(agent).ok_or_else(|| {
        serde_json::json!({
            "code": 2,
            "message": format!("Agent {} not found", agent)
        })
    })?;

    let json_str = message.to_string();
    let mut data = json_str.as_bytes().to_vec();
    data.push(b'\n');

    shell_process.child.write(&data).map_err(|e| {
        serde_json::json!({
            "code": 3,
            "message": format!("Failed to write to stdin: {}", e)
        })
    })?;

    println!("Message sent to agent {} {}", agent, json_str);

    Ok(serde_json::json!({
        "code": 0,
    }))
}

#[tauri::command]
pub async fn acp_start_listening(
    agent: &str,
    window: tauri::Window,
) -> Result<serde_json::Value, serde_json::Value> {
    let agent_name = agent.to_string();
    let window_clone = window.clone();
    let agent_name_clone = agent_name.clone();

    // Take the receiver from the shell process
    let receiver_opt = {
        let mut processes = AGENT_PROCESSES.lock().await;
        if let Some(shell_process) = processes.get_mut(&agent_name_clone) {
            Some(std::mem::replace(
                &mut shell_process._receiver,
                tauri::async_runtime::channel(1).1,
            ))
        } else {
            None
        }
    };

    let mut receiver = receiver_opt.ok_or_else(|| {
        serde_json::json!({
            "code": 2,
            "message": format!("Agent {} not found", agent)
        })
    })?;

    let handle = tokio::spawn(async move {
        let evt = "acp_message::".to_string() + &agent_name_clone;

        while let Some(event) = receiver.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    let text = String::from_utf8_lossy(&line).to_string();
                    println!("Received stdout: {}", text.trim());

                    let event_data = serde_json::json!({
                        "type": "message",
                        "message": text.trim().to_string()
                    });

                    let _ = window_clone.emit(evt.as_str(), event_data);
                }
                CommandEvent::Stderr(line) => {
                    let text = String::from_utf8_lossy(&line).to_string();
                    println!("Received stderr: {}", text.trim());

                    let event_data = serde_json::json!({
                        "type": "error",
                        "error": text.trim().to_string()
                    });

                    let _ = window_clone.emit(evt.as_str(), event_data);
                }
                CommandEvent::Terminated(payload) => {
                    println!("Process terminated: {:?}", payload);
                    let _ = window_clone.emit(
                        evt.as_str(),
                        serde_json::json!({
                            "type": "disconnect",
                            "code": payload.code,
                            "signal": payload.signal,
                        }),
                    );
                    break;
                }
                _ => {}
            }
        }
    });

    // Store the task handle
    {
        let mut tasks = LISTENING_TASKS.lock().await;
        tasks.insert(agent_name, handle);
    }

    println!("Started listening to agent {}", agent);
    Ok(serde_json::json!({
        "code": 0,
    }))
}

#[tauri::command]
pub async fn acp_stop_listening(agent: &str) -> Result<serde_json::Value, serde_json::Value> {
    let mut tasks = LISTENING_TASKS.lock().await;

    if let Some(handle) = tasks.remove(agent) {
        handle.abort();
        println!("Stopped listening to agent {}", agent);
        Ok(serde_json::json!({
            "code": 0,
        }))
    } else {
        Err(serde_json::json!({
            "code": 6,
            "message": format!("No active listener for agent {}", agent)
        }))
    }
}

#[tauri::command]
pub async fn acp_dispose(agent: &str) -> Result<serde_json::Value, serde_json::Value> {
    println!("acp_dispose {}", agent);

    // First stop listening if active
    {
        let mut tasks = LISTENING_TASKS.lock().await;
        if let Some(handle) = tasks.remove(agent) {
            handle.abort();
        }
    }

    // Then kill and remove the process
    {
        let mut processes = AGENT_PROCESSES.lock().await;
        if let Some(shell_process) = processes.remove(agent) {
            // Try to kill the process
            match shell_process.child.kill() {
                Ok(_) => {
                    println!("Agent {} process killed successfully", agent);
                }
                Err(e) => {
                    println!("Error killing agent {} process: {}", agent, e);
                    return Err(serde_json::json!({
                        "code": 7,
                        "message": format!("Failed to kill agent process: {}", e)
                    }));
                }
            }
        } else {
            return Err(serde_json::json!({
                "code": 8,
                "message": format!("Agent {} not found", agent)
            }));
        }
    }

    println!("Agent {} disposed successfully", agent);
    Ok(serde_json::json!({
        "code": 0,
    }))
}

/// Returns true if the file/directory name should be ignored (well-known types)
fn should_ignore_file(name: &str) -> bool {
    // macOS system files
    if name == ".DS_Store"
        || name == ".Spotlight-V100"
        || name == ".Trashes"
        || name == ".fseventsd"
        || name == ".localized"
    {
        return true;
    }

    // Windows system files
    if name == "Thumbs.db" || name == "desktop.ini" {
        return true;
    }

    // Version control
    if name == ".git" || name == ".svn" || name == ".hg" {
        return true;
    }

    false
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileNode {
    name: String,
    path: String,
    is_file: bool,
    is_dir: bool,
    children: Option<Vec<FileNode>>,
    size: Option<u64>,
}

#[tauri::command]
pub async fn read_directory(path: &str) -> Result<FileNode, String> {
    let path_buf = Path::new(path);

    if !path_buf.exists() {
        return Err("Directory does not exist".to_string());
    }

    let metadata = fs::metadata(path_buf).map_err(|e| format!("Failed to read metadata: {}", e))?;

    let name = path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    if metadata.is_dir() {
        let mut children = Vec::new();
        let entries =
            fs::read_dir(path_buf).map_err(|e| format!("Failed to read directory: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let child_path = entry.path();
            let child_metadata = entry
                .metadata()
                .map_err(|e| format!("Failed to read child metadata: {}", e))?;

            let child_name = child_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Skip well-known system files and directories
            if should_ignore_file(&child_name) {
                continue;
            }

            let child_node = if child_metadata.is_dir() {
                FileNode {
                    name: child_name,
                    path: child_path.to_string_lossy().to_string(),
                    is_file: false,
                    is_dir: true,
                    children: None, // Lazy load children
                    size: None,
                }
            } else {
                FileNode {
                    name: child_name,
                    path: child_path.to_string_lossy().to_string(),
                    is_file: true,
                    is_dir: false,
                    children: None,
                    size: Some(child_metadata.len()),
                }
            };

            children.push(child_node);
        }

        // Sort directories first, then files, both alphabetically
        children.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });

        Ok(FileNode {
            name,
            path: path_buf.to_string_lossy().to_string(),
            is_file: false,
            is_dir: true,
            children: Some(children),
            size: None,
        })
    } else {
        Ok(FileNode {
            name,
            path: path_buf.to_string_lossy().to_string(),
            is_file: true,
            is_dir: false,
            children: None,
            size: Some(metadata.len()),
        })
    }
}

#[tauri::command]
pub async fn read_file(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))
}

#[tauri::command]
pub async fn write_file(path: &str, content: &str) -> Result<(), String> {
    let path_buf = Path::new(path);

    // Check if the parent directory exists, create it if it doesn't
    if let Some(parent) = path_buf.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create parent directory: {}", e))?;
        }
    }

    fs::write(path, content).map_err(|e| format!("Failed to write file: {}", e))
}

#[tauri::command]
pub async fn read_binary_file(path: &str) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("Failed to read file: {}", e))
}

#[tauri::command]
pub async fn write_binary_file(path: &str, content: Vec<u8>) -> Result<(), String> {
    let path_buf = Path::new(path);

    // Check if the parent directory exists, create it if it doesn't
    if let Some(parent) = path_buf.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create parent directory: {}", e))?;
        }
    }

    fs::write(path, content).map_err(|e| format!("Failed to write file: {}", e))
}

#[tauri::command]
pub async fn read_file_by_range(path: &str, start: usize, end: usize) -> Result<String, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let lines: Vec<&str> = content.lines().collect();

    // Ensure start and end are within bounds
    let start = start.min(lines.len());
    let end = end.min(lines.len());

    if start >= end {
        return Ok(String::new());
    }

    // Extract the line range (start is inclusive, end is exclusive)
    let range_lines = &lines[start..end];
    Ok(range_lines.join("\n"))
}

#[tauri::command]
pub async fn create_file(path: &str) -> Result<(), String> {
    fs::write(path, "").map_err(|e| format!("Failed to create file: {}", e))
}

#[tauri::command]
pub async fn create_directory(path: &str) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| format!("Failed to create directory: {}", e))
}

#[tauri::command]
pub async fn rename_file(old_path: &str, new_path: &str) -> Result<(), String> {
    fs::rename(old_path, new_path).map_err(|e| format!("Failed to rename: {}", e))
}

#[tauri::command]
pub async fn delete_file(path: &str) -> Result<(), String> {
    let path_buf = Path::new(path);

    if path_buf.is_dir() {
        fs::remove_dir_all(path_buf).map_err(|e| format!("Failed to delete directory: {}", e))?;
    } else {
        fs::remove_file(path_buf).map_err(|e| format!("Failed to delete file: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn move_file(from_path: &str, to_path: &str) -> Result<(), String> {
    // Ensure the destination directory exists
    if let Some(parent) = Path::new(to_path).parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create destination directory: {}", e))?;
    }

    fs::rename(from_path, to_path).map_err(|e| format!("Failed to move file: {}", e))
}

struct TerminalSession {
    writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
}

static TERMINAL_SESSIONS: std::sync::LazyLock<Arc<Mutex<HashMap<String, TerminalSession>>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

#[tauri::command]
pub async fn terminal_create_session(
    terminal_id: String,
    cwd: Option<String>,
    cols: Option<u16>,
    rows: Option<u16>,
    window: tauri::Window,
) -> Result<serde_json::Value, String> {
    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows: rows.unwrap_or(24),
            cols: cols.unwrap_or(80),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("Failed to open pty: {}", e))?;

    let shell = if cfg!(target_os = "windows") {
        "cmd.exe".to_string()
    } else if cfg!(target_os = "macos") {
        "/bin/zsh".to_string()
    } else {
        "/bin/bash".to_string()
    };

    let mut cmd = CommandBuilder::new(&shell);
    cmd.env("TERM", "xterm-256color");

    if let Some(working_dir) = cwd {
        cmd.cwd(working_dir);
    }

    let _child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("Failed to spawn shell: {}", e))?;

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("Failed to clone reader: {}", e))?;

    let writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("Failed to take writer: {}", e))?;

    let terminal_id_clone = terminal_id.clone();
    let window_clone = window;

    tokio::task::spawn_blocking(move || {
        let event_name = format!("terminal_output::{}", terminal_id_clone);
        let mut buffer = vec![0u8; 8192];

        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buffer[..n]).to_string();
                    let _ = window_clone.emit(
                        &event_name,
                        serde_json::json!({
                            "type": "stdout",
                            "data": data
                        }),
                    );
                }
                Err(e) => {
                    let _ = window_clone.emit(
                        &event_name,
                        serde_json::json!({
                            "type": "error",
                            "data": format!("Error reading output: {}", e)
                        }),
                    );
                    break;
                }
            }
        }
    });

    let session = TerminalSession {
        writer,
        master: pair.master,
    };

    let mut sessions = TERMINAL_SESSIONS.lock().await;
    sessions.insert(terminal_id, session);

    Ok(serde_json::json!({
        "success": true
    }))
}

#[tauri::command]
pub async fn terminal_send_input(
    terminal_id: String,
    input: String,
) -> Result<serde_json::Value, String> {
    let mut sessions = TERMINAL_SESSIONS.lock().await;

    let session = sessions
        .get_mut(&terminal_id)
        .ok_or_else(|| format!("Terminal session {} not found", terminal_id))?;

    session
        .writer
        .write_all(input.as_bytes())
        .map_err(|e| format!("Failed to write to pty: {}", e))?;

    session
        .writer
        .flush()
        .map_err(|e| format!("Failed to flush pty: {}", e))?;

    Ok(serde_json::json!({
        "success": true
    }))
}

#[tauri::command]
pub async fn terminal_resize(
    terminal_id: String,
    cols: u16,
    rows: u16,
) -> Result<serde_json::Value, String> {
    let sessions = TERMINAL_SESSIONS.lock().await;

    let session = sessions
        .get(&terminal_id)
        .ok_or_else(|| format!("Terminal session {} not found", terminal_id))?;

    session
        .master
        .resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("Failed to resize pty: {}", e))?;

    Ok(serde_json::json!({
        "success": true
    }))
}

#[tauri::command]
pub async fn terminal_kill_session(terminal_id: String) -> Result<(), String> {
    let mut sessions = TERMINAL_SESSIONS.lock().await;
    sessions.remove(&terminal_id);
    Ok(())
}

// ACP Terminal management structures
struct ACPTerminal {
    child: tokio::process::Child,
    output: Arc<Mutex<String>>,
    truncated: bool,
    exit_status: Arc<Mutex<Option<(Option<i32>, Option<String>)>>>,
    output_byte_limit: usize,
}

static ACP_TERMINALS: std::sync::LazyLock<Arc<Mutex<HashMap<String, ACPTerminal>>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

// Helper function to check if a package is already installed
async fn check_package_installed(
    package_name: &str,
    app_config_dir: &PathBuf,
    app: &tauri::AppHandle,
) -> Result<bool, serde_json::Value> {
    if tauri::is_dev() && package_name == RAVEN_ACP_PACKAGE_NAME {
        return Ok(true);
    }
    let sidecar_command = app.shell().sidecar("bun").map_err(|e| {
        serde_json::json!({
            "code": 12,
            "message": format!("Failed to load bun sidecar: {}", e)
        })
    })?;

    let (mut rx, _child) = sidecar_command
        .args(["pm", "ls", "-g", package_name])
        .env("BUN_INSTALL", app_config_dir)
        .spawn()
        .map_err(|e| {
            serde_json::json!({
                "code": 12,
                "message": format!("Failed to check package installation: {}", e)
            })
        })?;

    let mut stdout_data = Vec::new();

    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(line) => stdout_data.extend_from_slice(&line),
            CommandEvent::Terminated(_) => break,
            _ => {}
        }
    }

    let stdout = String::from_utf8_lossy(&stdout_data);
    Ok(stdout.contains(package_name))
}

// Helper function to get the installed package version
async fn get_installed_version(
    package_name: &str,
    app_config_dir: &PathBuf,
    app: &tauri::AppHandle,
) -> Result<Option<String>, serde_json::Value> {
    if tauri::is_dev() && package_name == RAVEN_ACP_PACKAGE_NAME {
        return Ok(Some("0.0.0".to_string()));
    }

    let sidecar_command = app.shell().sidecar("bun").map_err(|e| {
        serde_json::json!({
            "code": 13,
            "message": format!("Failed to load bun sidecar: {}", e)
        })
    })?;

    let (mut rx, _child) = sidecar_command
        .args(["pm", "ls", "-g", package_name])
        .env("BUN_INSTALL", app_config_dir)
        .spawn()
        .map_err(|e| {
            serde_json::json!({
                "code": 13,
                "message": format!("Failed to get package version: {}", e)
            })
        })?;

    let mut stdout_data = Vec::new();

    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(line) => stdout_data.extend_from_slice(&line),
            CommandEvent::Terminated(_) => break,
            _ => {}
        }
    }

    let stdout = String::from_utf8_lossy(&stdout_data);

    // Parse the version from the output
    // The output format is typically: "package_name@version"
    // For scoped packages like @qwen-code/qwen-code@0.7.2, the version is the last part
    for line in stdout.lines() {
        if line.contains(package_name) && line.contains('@') {
            let parts: Vec<&str> = line.split('@').collect();
            if parts.len() >= 2 {
                let version = parts.last().unwrap().trim().to_string();
                return Ok(Some(version));
            }
        }
    }

    Ok(None)
}

// Helper function to get the latest version from npm registry
async fn get_latest_version(package_name: &str) -> Result<Option<String>, serde_json::Value> {
    if tauri::is_dev() && package_name == RAVEN_ACP_PACKAGE_NAME {
        return Ok(Some("0.0.0".to_string()));
    }

    let url = format!("https://registry.npmjs.org/{}", package_name);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| {
            serde_json::json!({
                "code": 14,
                "message": format!("Failed to create HTTP client: {}", e)
            })
        })?;

    let response = client.get(&url).send().await.map_err(|e| {
        serde_json::json!({
            "code": 15,
            "message": format!("Failed to fetch package info: {}", e)
        })
    })?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let json: serde_json::Value = response.json().await.map_err(|e| {
        serde_json::json!({
            "code": 16,
            "message": format!("Failed to parse response: {}", e)
        })
    })?;

    if let Some(latest) = json["dist-tags"]["latest"].as_str() {
        Ok(Some(latest.to_string()))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn check_for_updates(
    agent_name: String,
    package_name: String,
    app: tauri::AppHandle,
    window: tauri::Window,
) -> Result<serde_json::Value, serde_json::Value> {
    let app_config_dir = app.path().app_data_dir().unwrap();
    let event_name = format!("package_install_status::{}", agent_name);

    // Get the installed version
    let installed_version = match get_installed_version(&package_name, &app_config_dir, &app).await
    {
        Ok(Some(v)) => v,
        Ok(None) => "unknown".to_string(),
        Err(e) => {
            println!("Failed to get installed version: {:?}", e);
            "unknown".to_string()
        }
    };

    // Check for updates
    let update_available = match get_latest_version(&package_name).await {
        Ok(Some(latest)) => {
            if installed_version != "unknown" && installed_version != latest {
                Some(latest)
            } else {
                None
            }
        }
        Ok(None) => None,
        Err(e) => {
            println!("Failed to check for updates: {:?}", e);
            None
        }
    };

    let status = if update_available.is_some() {
        "update_available"
    } else {
        "already_installed"
    };

    let message = if let Some(ref latest) = update_available {
        format!(
            "{} v{} is installed (v{} available)",
            package_name, installed_version, latest
        )
    } else {
        format!(
            "{} v{} is installed (up to date)",
            package_name, installed_version
        )
    };

    let _ = window.emit(
        &event_name,
        serde_json::json!({
            "package": package_name,
            "status": status,
            "message": message,
            "current_version": installed_version,
            "latest_version": update_available
        }),
    );

    Ok(serde_json::json!({
        "code": 0,
        "status": status,
        "current_version": installed_version,
        "latest_version": update_available
    }))
}

// ACP Terminal RPC Methods

#[derive(Deserialize, Debug)]
pub struct EnvVar {
    name: String,
    value: String,
}

#[tauri::command]
pub async fn acp_terminal_create(
    session_id: String,
    command: String,
    args: Option<Vec<String>>,
    env: Option<Vec<EnvVar>>,
    cwd: Option<String>,
    output_byte_limit: Option<usize>,
) -> Result<serde_json::Value, serde_json::Value> {
    let terminal_id = format!("{}_{}", session_id, nanoid::nanoid!());
    println!("Creating terminal session: {} | command: {} | args: {:?} | env: {:?} | cwd: {:?} | output_byte_limit: {:?}", terminal_id, command, args, env, cwd, output_byte_limit);
    let byte_limit = output_byte_limit.unwrap_or(1024 * 1024); // Default 1MB

    let mut cmd = TokioCommand::new(&command);

    if let Some(arguments) = args {
        cmd.args(&arguments);
    }

    if let Some(env_vars) = env {
        for var in env_vars {
            cmd.env(&var.name, &var.value);
        }
    }

    if let Some(working_dir) = cwd {
        cmd.current_dir(working_dir);
    }

    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());

    let mut child = cmd.spawn().map_err(|e| {
        serde_json::json!({
            "code": -32000,
            "message": format!("Failed to spawn command: {}", e)
        })
    })?;

    let output = Arc::new(Mutex::new(String::new()));
    let exit_status = Arc::new(Mutex::new(None));

    // Capture stdout
    if let Some(stdout) = child.stdout.take() {
        let output_clone = output.clone();
        let limit = byte_limit;

        tokio::spawn(async move {
            let mut reader = AsyncBufReader::new(stdout);
            let mut buffer = Vec::new();

            match reader.read_to_end(&mut buffer).await {
                Ok(_) => {
                    let mut output_lock = output_clone.lock().await;
                    let text =
                        String::from_utf8_lossy(&buffer[..buffer.len().min(limit)]).to_string();
                    output_lock.push_str(&text);
                }
                Err(e) => {
                    eprintln!("Error reading stdout: {}", e);
                }
            }
        });
    }

    // Capture stderr
    if let Some(stderr) = child.stderr.take() {
        let output_clone = output.clone();
        let limit = byte_limit;

        tokio::spawn(async move {
            let mut reader = AsyncBufReader::new(stderr);
            let mut buffer = Vec::new();

            match reader.read_to_end(&mut buffer).await {
                Ok(_) => {
                    let mut output_lock = output_clone.lock().await;
                    let text =
                        String::from_utf8_lossy(&buffer[..buffer.len().min(limit)]).to_string();
                    output_lock.push_str(&text);
                }
                Err(e) => {
                    eprintln!("Error reading stderr: {}", e);
                }
            }
        });
    }

    // Monitor exit status
    let exit_status_clone = exit_status.clone();
    tokio::spawn(async move {
        match child.wait().await {
            Ok(status) => {
                let mut exit_lock = exit_status_clone.lock().await;
                // Signal is Unix-specific, so we don't use it for cross-platform compatibility
                *exit_lock = Some((status.code(), None));
            }
            Err(e) => {
                eprintln!("Error waiting for child: {}", e);
            }
        }
    });

    let terminal = ACPTerminal {
        child: TokioCommand::new("sleep").arg("0").spawn().unwrap(), // Placeholder
        output,
        truncated: false,
        exit_status,
        output_byte_limit: byte_limit,
    };

    let mut terminals = ACP_TERMINALS.lock().await;
    terminals.insert(terminal_id.clone(), terminal);

    Ok(serde_json::json!({
        "terminalId": terminal_id
    }))
}

#[tauri::command]
pub async fn acp_terminal_output(
    _session_id: String,
    terminal_id: String,
) -> Result<serde_json::Value, serde_json::Value> {
    let terminals = ACP_TERMINALS.lock().await;

    let terminal = terminals.get(&terminal_id).ok_or_else(|| {
        serde_json::json!({
            "code": -32000,
            "message": format!("Terminal {} not found", terminal_id)
        })
    })?;

    let output = terminal.output.lock().await.clone();
    let exit_status_lock = terminal.exit_status.lock().await;
    let truncated = output.len() >= terminal.output_byte_limit;

    let mut result = serde_json::json!({
        "output": output,
        "truncated": truncated
    });

    if let Some((exit_code, signal)) = exit_status_lock.as_ref() {
        result["exitStatus"] = serde_json::json!({
            "exitCode": exit_code,
            "signal": signal
        });
    }

    Ok(result)
}

#[tauri::command]
pub async fn acp_terminal_wait_for_exit(
    _session_id: String,
    terminal_id: String,
) -> Result<serde_json::Value, serde_json::Value> {
    let terminals = ACP_TERMINALS.lock().await;

    let terminal = terminals.get(&terminal_id).ok_or_else(|| {
        serde_json::json!({
            "code": -32000,
            "message": format!("Terminal {} not found", terminal_id)
        })
    })?;

    let exit_status = terminal.exit_status.clone();
    drop(terminals);

    // Poll for exit status
    for _ in 0..300 {
        // Wait up to 30 seconds
        let status_lock = exit_status.lock().await;
        if let Some((exit_code, signal)) = status_lock.as_ref() {
            return Ok(serde_json::json!({
                "exitCode": exit_code,
                "signal": signal
            }));
        }
        drop(status_lock);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    // Timeout
    Ok(serde_json::json!({
        "exitCode": null,
        "signal": null
    }))
}

#[tauri::command]
pub async fn acp_terminal_kill(
    _session_id: String,
    terminal_id: String,
) -> Result<(), serde_json::Value> {
    let mut terminals = ACP_TERMINALS.lock().await;

    if let Some(terminal) = terminals.get_mut(&terminal_id) {
        let _ = terminal.child.kill().await;
        Ok(())
    } else {
        Err(serde_json::json!({
            "code": -32000,
            "message": format!("Terminal {} not found", terminal_id)
        }))
    }
}

#[tauri::command]
pub async fn acp_terminal_release(
    _session_id: String,
    terminal_id: String,
) -> Result<(), serde_json::Value> {
    let mut terminals = ACP_TERMINALS.lock().await;
    terminals.remove(&terminal_id);
    Ok(())
}

// File Watcher structures
struct FileWatcherSession {
    _watcher: notify::RecommendedWatcher,
    #[allow(dead_code)]
    base_path: PathBuf,
}

static FILE_WATCHERS: std::sync::LazyLock<Arc<Mutex<HashMap<String, FileWatcherSession>>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

#[derive(Deserialize)]
pub struct WatchFileArgs {
    session_id: String,
    #[allow(dead_code)]
    relative_path: String,
    base_path: String,
}

#[tauri::command]
pub async fn watch_file(args: WatchFileArgs, window: tauri::Window) -> Result<(), String> {
    let WatchFileArgs {
        session_id,
        relative_path: _,
        base_path,
    } = args;

    let base_path_buf = PathBuf::from(&base_path);

    // Validate the base path exists
    if !base_path_buf.exists() {
        return Err("Base path does not exist".to_string());
    }

    let mut watchers = FILE_WATCHERS.lock().await;

    // Only create a new watcher if one doesn't exist for this session
    if !watchers.contains_key(&session_id) {
        let window_clone = window.clone();
        let base_path_clone = base_path_buf.clone();

        // Create the watcher with the new notify API
        let mut watcher =
            notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
                match res {
                    Ok(event) => {
                        match &event.kind {
                            // Only handle actual data/content changes, not metadata
                            EventKind::Modify(notify::event::ModifyKind::Data(_)) => {
                                // Handle file content modifications
                                for path in &event.paths {
                                    if path.is_file() {
                                        let relative = path
                                            .strip_prefix(&base_path_clone)
                                            .ok()
                                            .and_then(|p: &Path| p.to_str())
                                            .unwrap_or("");

                                        let _ = window_clone.emit(
                                            "file_changed",
                                            serde_json::json!({
                                                "path": relative,
                                                "type": "modified"
                                            }),
                                        );
                                    }
                                }
                            }
                            EventKind::Create(_) => {
                                // Handle file/folder creation
                                for path in &event.paths {
                                    let relative = path
                                        .strip_prefix(&base_path_clone)
                                        .ok()
                                        .and_then(|p: &Path| p.to_str())
                                        .unwrap_or("");

                                    let _ = window_clone.emit(
                                        "file_tree_changed",
                                        serde_json::json!({
                                            "path": relative,
                                            "type": "created"
                                        }),
                                    );
                                }
                            }
                            EventKind::Remove(_) => {
                                // Handle file/folder removal
                                for path in &event.paths {
                                    let relative = path
                                        .strip_prefix(&base_path_clone)
                                        .ok()
                                        .and_then(|p: &Path| p.to_str())
                                        .unwrap_or("");

                                    let _ = window_clone.emit(
                                        "file_tree_changed",
                                        serde_json::json!({
                                            "path": relative,
                                            "type": "removed"
                                        }),
                                    );
                                }
                            }
                            // Handle rename/move events as file tree changes
                            EventKind::Modify(notify::event::ModifyKind::Name(_)) => {
                                for path in &event.paths {
                                    let relative = path
                                        .strip_prefix(&base_path_clone)
                                        .ok()
                                        .and_then(|p: &Path| p.to_str())
                                        .unwrap_or("");

                                    let _ = window_clone.emit(
                                        "file_tree_changed",
                                        serde_json::json!({
                                            "path": relative,
                                            "type": "renamed"
                                        }),
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        eprintln!("Watch error: {:?}", e);
                    }
                }
            })
            .expect("Failed to create file watcher");

        // Watch the base directory recursively
        watcher
            .watch(&base_path_buf, RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch directory: {}", e))?;

        watchers.insert(
            session_id.clone(),
            FileWatcherSession {
                _watcher: watcher,
                base_path: base_path_buf,
            },
        );
    }

    Ok(())
}

#[tauri::command]
pub async fn unwatch_file(
    _session_id: String,
    _relative_path: String,
    _base_path: String,
) -> Result<(), String> {
    // No-op since we now watch the entire base directory
    // Individual files don't need to be unwatched anymore
    Ok(())
}

#[tauri::command]
pub async fn stop_watching(session_id: String) -> Result<(), String> {
    let mut watchers = FILE_WATCHERS.lock().await;
    watchers.remove(&session_id);
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileSuggestion {
    name: String,
    path: String,
    is_dir: bool,
}

#[tauri::command]
pub async fn glob_files(
    directory: &str,
    pattern: Option<&str>,
) -> Result<Vec<FileSuggestion>, String> {
    use ignore::WalkBuilder;
    use std::path::Path;

    let dir_path = Path::new(directory);
    if !dir_path.exists() {
        return Err(format!("Directory does not exist: {}", directory));
    }

    let query = pattern.unwrap_or("").to_lowercase();

    let mut results = Vec::new();

    let walker = WalkBuilder::new(directory)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build();

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();

        // Skip the root directory itself
        if path == dir_path {
            continue;
        }

        let file_name = match path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => continue,
        };

        let relative_path = path
            .strip_prefix(directory)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        // Match query as substring in relative path (case-insensitive)
        if !query.is_empty() && !relative_path.to_lowercase().contains(&query) {
            continue;
        }

        let is_dir = path.is_dir();

        results.push(FileSuggestion {
            name: file_name,
            is_dir,
            path: relative_path,
        });
    }

    Ok(results)
}

#[tauri::command]
pub async fn upgrade_package(
    agent_name: String,
    package_name: String,
    app: tauri::AppHandle,
    window: tauri::Window,
) -> Result<serde_json::Value, serde_json::Value> {
    let app_config_dir = app.path().app_data_dir().unwrap();
    let event_name = format!("package_install_status::{}", agent_name);

    // Emit upgrading event
    let _ = window.emit(
        &event_name,
        serde_json::json!({
            "package": package_name,
            "status": "upgrading",
            "message": format!("Upgrading {}...", package_name)
        }),
    );

    // Run bun update -g for the package
    let update_command = app.shell().sidecar("bun").map_err(|e| {
        let _ = window.emit(
            &event_name,
            serde_json::json!({
                "package": package_name,
                "status": "error",
                "message": format!("Failed to load bun sidecar: {}", e)
            }),
        );
        serde_json::json!({
            "code": 17,
            "message": format!("Failed to load bun sidecar for upgrade: {}", e)
        })
    })?;

    let (mut update_rx, _update_child) = update_command
        .args(["update", "-g", &package_name, "--latest"])
        .env("BUN_INSTALL", &app_config_dir)
        .spawn()
        .map_err(|e| {
            let _ = window.emit(
                &event_name,
                serde_json::json!({
                    "package": package_name,
                    "status": "error",
                    "message": format!("Failed to run bun update: {}", e)
                }),
            );
            serde_json::json!({
                "code": 18,
                "message": format!("Failed to run bun update: {}", e)
            })
        })?;

    // Wait for the update process to complete
    let mut update_failed = false;
    while let Some(event) = update_rx.recv().await {
        match event {
            CommandEvent::Stderr(line) => {
                let text = String::from_utf8_lossy(&line);
                println!("Update stderr: {}", text);
                if text.contains("error") || text.contains("failed") {
                    update_failed = true;
                }
            }
            CommandEvent::Terminated(payload) => {
                if let Some(code) = payload.code {
                    if code != 0 {
                        update_failed = true;
                    }
                }
                break;
            }
            _ => {}
        }
    }

    if update_failed {
        let _ = window.emit(
            &event_name,
            serde_json::json!({
                "package": package_name,
                "status": "error",
                "message": format!("Failed to upgrade {}", package_name)
            }),
        );
        return Err(serde_json::json!({
            "code": 19,
            "message": format!("Failed to upgrade {}", package_name)
        }));
    }

    println!("Update command completed for {}", package_name);

    // Get the new version
    let new_version = get_installed_version(&package_name, &app_config_dir, &app)
        .await
        .ok()
        .flatten();

    let message = if let Some(ref v) = new_version {
        format!("Successfully upgraded {} to v{}", package_name, v)
    } else {
        format!("Successfully upgraded {}", package_name)
    };

    let _ = window.emit(
        &event_name,
        serde_json::json!({
            "package": package_name,
            "status": "upgrade_success",
            "message": message,
            "current_version": new_version
        }),
    );

    Ok(serde_json::json!({
        "code": 0,
        "version": new_version
    }))
}

#[tauri::command]
pub async fn is_git_repo(path: &str) -> Result<bool, String> {
    let path_buf = Path::new(path);
    git2::Repository::open(path_buf)
        .map(|_| true)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_git_diff_all(base_path: &str) -> Result<String, String> {
    let repo = git2::Repository::open(base_path)
        .map_err(|e| format!("Failed to open repository: {}", e))?;

    let head = repo
        .head()
        .map_err(|e| format!("Failed to get HEAD: {}", e))?;

    let head_commit = head
        .peel_to_commit()
        .map_err(|e| format!("Failed to peel to commit: {}", e))?;

    let head_tree = head_commit
        .tree()
        .map_err(|e| format!("Failed to get tree: {}", e))?;

    let index = repo
        .index()
        .map_err(|e| format!("Failed to get index: {}", e))?;

    let mut opts = git2::DiffOptions::new();
    let diff = repo
        .diff_tree_to_index(Some(&head_tree), Some(&index), Some(&mut opts))
        .map_err(|e| format!("Failed to create diff: {}", e))?;

    let mut diff_text = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        match line.origin() {
            'F' | 'H' => {
                // File headers (diff --git) and hunk headers (@@)
                // These don't have the origin prefix, just push the content
                diff_text.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
            }
            '+' | '-' | ' ' => {
                // Content lines - include the origin prefix
                diff_text.push(line.origin());
                diff_text.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
            }
            _ => {}
        }
        true
    })
    .map_err(|e| format!("Failed to print diff: {}", e))?;

    Ok(diff_text)
}

// MCP Server Management
static MCP_MANAGER: std::sync::LazyLock<Arc<Mutex<Option<McpManager>>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(None)));

#[tauri::command]
pub async fn mcp_start_servers(
    configs: Vec<(String, McpServerConfig)>,
    app: tauri::AppHandle,
) -> Result<serde_json::Value, serde_json::Value> {
    println!("Starting {} MCP servers: {:#?}", configs.len(), configs);

    // Initialize manager if needed
    {
        let mut manager = MCP_MANAGER.lock().await;
        if manager.is_none() {
            *manager = Some(crate::mcp::McpManager::new(app.clone()));
        }
    }

    let manager = MCP_MANAGER.lock().await;
    let manager_ref = manager.as_ref().expect("Manager should be initialized");

    manager_ref
        .start_servers(configs)
        .await
        .map(|results| {
            serde_json::to_value(results).map_err(|e| {
                serde_json::json!({
                    "code": -32000,
                    "message": format!("Failed to serialize results: {}", e)
                })
            })
        })
        .map_err(|e| {
            serde_json::json!({
                "code": -32000,
                "message": format!("Failed to start MCP servers: {}", e)
            })
        })?
}

#[tauri::command]
pub async fn mcp_list_tools() -> Result<serde_json::Value, serde_json::Value> {
    let manager = MCP_MANAGER.lock().await;
    let manager_ref = manager.as_ref().ok_or_else(|| {
        serde_json::json!({
            "code": -32000,
            "message": "MCP manager not initialized. Call mcp_start_servers first."
        })
    })?;

    manager_ref
        .list_tools()
        .await
        .map(|tools| {
            serde_json::to_value(tools).map_err(|e| {
                serde_json::json!({
                    "code": -32000,
                    "message": format!("Failed to serialize tools: {}", e)
                })
            })
        })
        .map_err(|e| {
            serde_json::json!({
                "code": -32000,
                "message": format!("Failed to list tools: {}", e)
            })
        })?
}

#[tauri::command]
pub async fn mcp_call_tool(
    request: McpToolCallRequest,
) -> Result<serde_json::Value, serde_json::Value> {
    println!(
        "Calling MCP tool: {} on server {}",
        request.tool_name, request.server_id
    );

    let manager = MCP_MANAGER.lock().await;
    let manager_ref = manager.as_ref().ok_or_else(|| {
        serde_json::json!({
            "code": -32000,
            "message": "MCP manager not initialized. Call mcp_start_servers first."
        })
    })?;

    manager_ref
        .call_tool(request)
        .await
        .map(|response| {
            serde_json::to_value(response).map_err(|e| {
                serde_json::json!({
                    "code": -32000,
                    "message": format!("Failed to serialize response: {}", e)
                })
            })
        })
        .map_err(|e| {
            serde_json::json!({
                "code": -32000,
                "message": format!("Tool call failed: {}", e)
            })
        })?
}

#[tauri::command]
pub async fn mcp_stop_server(server_id: String) -> Result<(), serde_json::Value> {
    println!("Stopping MCP server: {}", server_id);

    let manager = MCP_MANAGER.lock().await;
    let manager_ref = manager.as_ref().ok_or_else(|| {
        serde_json::json!({
            "code": -32000,
            "message": "MCP manager not initialized. Call mcp_start_servers first."
        })
    })?;

    manager_ref.stop_server(&server_id).await.map_err(|e| {
        serde_json::json!({
            "code": -32000,
            "message": format!("Failed to stop server: {}", e)
        })
    })
}

#[tauri::command]
pub async fn mcp_list_servers() -> Result<serde_json::Value, serde_json::Value> {
    let manager = MCP_MANAGER.lock().await;
    let manager_ref = manager.as_ref().ok_or_else(|| {
        serde_json::json!({
            "code": -32000,
            "message": "MCP manager not initialized. Call mcp_start_servers first."
        })
    })?;

    let servers = manager_ref.list_servers().await;
    serde_json::to_value(servers).map_err(|e| {
        serde_json::json!({
            "code": -32000,
            "message": format!("Failed to serialize server list: {}", e)
        })
    })
}

#[tauri::command]
pub async fn mcp_get_server_logs(
    server_id: String,
) -> Result<serde_json::Value, serde_json::Value> {
    let manager = MCP_MANAGER.lock().await;
    let manager_ref = manager.as_ref().ok_or_else(|| {
        serde_json::json!({
            "code": -32000,
            "message": "MCP manager not initialized. Call mcp_start_servers first."
        })
    })?;

    let logs = manager_ref.get_server_logs(&server_id).await.map_err(|e| {
        serde_json::json!({
            "code": -32000,
            "message": format!("Failed to get server logs: {}", e)
        })
    })?;

    serde_json::to_value(logs).map_err(|e| {
        serde_json::json!({
            "code": -32000,
            "message": format!("Failed to serialize logs: {}", e)
        })
    })
}

#[tauri::command]
pub async fn mcp_list_resources() -> Result<serde_json::Value, serde_json::Value> {
    let manager = MCP_MANAGER.lock().await;
    let manager_ref = manager.as_ref().ok_or_else(|| {
        serde_json::json!({
            "code": -32000,
            "message": "MCP manager not initialized. Call mcp_start_servers first."
        })
    })?;

    manager_ref
        .list_resources()
        .await
        .map(|resources| {
            serde_json::to_value(resources).map_err(|e| {
                serde_json::json!({
                    "code": -32000,
                    "message": format!("Failed to serialize resources: {}", e)
                })
            })
        })
        .map_err(|e| {
            serde_json::json!({
                "code": -32000,
                "message": format!("Failed to list resources: {}", e)
            })
        })?
}

#[tauri::command]
pub async fn mcp_list_prompts() -> Result<serde_json::Value, serde_json::Value> {
    let manager = MCP_MANAGER.lock().await;
    let manager_ref = manager.as_ref().ok_or_else(|| {
        serde_json::json!({
            "code": -32000,
            "message": "MCP manager not initialized. Call mcp_start_servers first."
        })
    })?;

    manager_ref
        .list_prompts()
        .await
        .map(|prompts| {
            serde_json::to_value(prompts).map_err(|e| {
                serde_json::json!({
                    "code": -32000,
                    "message": format!("Failed to serialize prompts: {}", e)
                })
            })
        })
        .map_err(|e| {
            serde_json::json!({
                "code": -32000,
                "message": format!("Failed to list prompts: {}", e)
            })
        })?
}

#[tauri::command]
pub async fn mcp_read_resource(
    request: McpResourceReadRequest,
) -> Result<serde_json::Value, serde_json::Value> {
    println!(
        "Reading MCP resource: {} from server {}",
        request.uri, request.server_id
    );

    let manager = MCP_MANAGER.lock().await;
    let manager_ref = manager.as_ref().ok_or_else(|| {
        serde_json::json!({
            "code": -32000,
            "message": "MCP manager not initialized. Call mcp_start_servers first."
        })
    })?;

    manager_ref
        .read_resource(request)
        .await
        .map(|response| {
            serde_json::to_value(response).map_err(|e| {
                serde_json::json!({
                    "code": -32000,
                    "message": format!("Failed to serialize response: {}", e)
                })
            })
        })
        .map_err(|e| {
            serde_json::json!({
                "code": -32000,
                "message": format!("Resource read failed: {}", e)
            })
        })?
}

#[tauri::command]
pub async fn mcp_get_prompt(
    request: McpPromptGetRequest,
) -> Result<serde_json::Value, serde_json::Value> {
    println!(
        "Getting MCP prompt: {} from server {}",
        request.name, request.server_id
    );

    let manager = MCP_MANAGER.lock().await;
    let manager_ref = manager.as_ref().ok_or_else(|| {
        serde_json::json!({
            "code": -32000,
            "message": "MCP manager not initialized. Call mcp_start_servers first."
        })
    })?;

    manager_ref
        .get_prompt(request)
        .await
        .map(|response| {
            serde_json::to_value(response).map_err(|e| {
                serde_json::json!({
                    "code": -32000,
                    "message": format!("Failed to serialize response: {}", e)
                })
            })
        })
        .map_err(|e| {
            serde_json::json!({
                "code": -32000,
                "message": format!("Prompt get failed: {}", e)
            })
        })?
}
