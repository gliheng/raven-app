use tauri::{Emitter, Manager, RunEvent};
use tauri_plugin_sql::{Migration, MigrationKind};
mod handlers;
mod mcp;

fn get_migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            description: "create_initial_tables",
            sql: include_str!("../migrations/001_initial.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 2,
            description: "create_settings_table",
            sql: include_str!("../migrations/002_settings.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 3,
            description: "create_journal_table",
            sql: include_str!("../migrations/003_journal.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 4,
            description: "create_image_table",
            sql: include_str!("../migrations/004_image.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 5,
            description: "create_research_tables",
            sql: include_str!("../migrations/005_research.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 6,
            description: "add_journal_color",
            sql: include_str!("../migrations/006_journal_color.sql"),
            kind: MigrationKind::Up,
        },
    ]
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(
            tauri_plugin_sql::Builder::new()
                .add_migrations("sqlite:data.db", get_migrations())
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            app.manage(handlers::PendingAppUpdate::default());
            #[cfg(debug_assertions)] // only include this code on debug builds
            {
                use tauri::Manager;
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                // Prevent window from closing on Cmd+W and emit event to frontend
                api.prevent_close();
                let _ = window.emit("window-event", ());
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            // ACP commands
            handlers::acp_initialize,
            handlers::acp_send_message,
            handlers::acp_start_listening,
            handlers::acp_stop_listening,
            handlers::acp_dispose,
            handlers::read_file_by_range,
            handlers::terminal_create_session,
            handlers::terminal_send_input,
            handlers::terminal_resize,
            handlers::terminal_kill_session,
            handlers::acp_terminal_create,
            handlers::acp_terminal_output,
            handlers::acp_terminal_wait_for_exit,
            handlers::acp_terminal_kill,
            handlers::acp_terminal_release,
            // Editor
            handlers::create_file,
            handlers::create_directory,
            handlers::read_directory,
            handlers::read_file,
            handlers::write_file,
            handlers::read_binary_file,
            handlers::write_binary_file,
            handlers::move_file,
            handlers::rename_file,
            handlers::delete_file,
            handlers::watch_file,
            handlers::unwatch_file,
            handlers::stop_watching,
            handlers::glob_files,
            handlers::is_git_repo,
            handlers::get_git_diff_all,
            // Package management
            handlers::upgrade_package,
            handlers::check_for_updates,
            handlers::check_app_update,
            handlers::install_app_update,
            // MCP commands
            handlers::mcp_start_servers,
            handlers::mcp_stop_server,
            handlers::mcp_list_servers,
            handlers::mcp_get_server_logs,
            handlers::mcp_list_tools,
            handlers::mcp_call_tool,
            handlers::mcp_list_resources,
            handlers::mcp_list_prompts,
            handlers::mcp_read_resource,
            handlers::mcp_get_prompt,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app: &tauri::AppHandle, event: RunEvent| {
            match event {
                // 1. Prevent exit on close
                #[cfg(target_os = "macos")]
                RunEvent::ExitRequested { api, .. } => {
                    api.prevent_exit();
                }

                // 2. Listen for the "Reopen" event (Clicking Dock Icon)
                // This event specifically fires when the app is running but focused via Dock
                #[cfg(target_os = "macos")]
                RunEvent::Reopen {
                    has_visible_windows,
                    ..
                } => {
                    if !has_visible_windows {
                        if let Some(window) = app.get_webview_window("main") {
                            window.show().unwrap();
                            window.set_focus().unwrap();
                        }
                    }
                }
                _ => {}
            }
        });
}
