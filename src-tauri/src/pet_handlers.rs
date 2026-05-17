use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use tauri::Manager;
use tauri_plugin_store::StoreExt;

use crate::pet_window;

#[derive(Serialize, Debug)]
pub struct PetInfo {
    slug: String,
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Deserialize, Debug)]
pub struct PetState {
    pub slug: Option<String>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub enabled: Option<bool>,
}

fn find_spritesheet(pet_dir: &PathBuf) -> Option<PathBuf> {
    for ext in &["webp", "png"] {
        let path = pet_dir.join(format!("spritesheet.{}", ext));
        if path.exists() {
            return Some(path);
        }
    }
    None
}

/// All directories where pets may live, codex CLI dir first.
fn pets_roots(app: &tauri::AppHandle) -> Vec<PathBuf> {
    let mut roots = vec![];

    // Primary: CLI-installed location
    if let Ok(home) = std::env::var("HOME") {
        let codex_dir = PathBuf::from(&home).join(".codex").join("pets");
        if codex_dir.is_dir() {
            roots.push(codex_dir);
        }
    }

    // Secondary: app data dir
    if let Ok(data_dir) = app.path().app_data_dir() {
        let pets_dir = data_dir.join("pets");
        let _ = fs::create_dir_all(&pets_dir);
        roots.push(pets_dir);
    }

    roots
}

#[tauri::command]
pub fn list_pets(app: tauri::AppHandle) -> Result<Value, Value> {
    let mut seen = std::collections::HashSet::new();
    let mut pets: Vec<PetInfo> = vec![];
    for root in pets_roots(&app) {
        if let Ok(entries) = fs::read_dir(&root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                if find_spritesheet(&path).is_none() {
                    continue;
                }
                let slug = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if !seen.insert(slug.clone()) {
                    continue;
                }
                let pet_json = path.join("pet.json");
                let display_name = if let Ok(data) = fs::read_to_string(&pet_json) {
                    if let Ok(v) = serde_json::from_str::<Value>(&data) {
                        v.get("displayName")
                            .and_then(|n| n.as_str())
                            .unwrap_or(&slug)
                            .to_string()
                    } else {
                        slug.clone()
                    }
                } else {
                    slug.clone()
                };
                pets.push(PetInfo {
                    slug,
                    display_name,
                });
            }
        }
    }
    pets.sort_by(|a, b| a.slug.cmp(&b.slug));
    serde_json::to_value(pets).map_err(|e| {
        serde_json::json!({
            "code": "list_pets_error",
            "message": e.to_string()
        })
    })
}

#[tauri::command]
pub fn read_pet_spritesheet(slug: String, app: tauri::AppHandle) -> Result<Value, Value> {
    for root in pets_roots(&app) {
        let pet_dir = root.join(&slug);
        if let Some(spritesheet) = find_spritesheet(&pet_dir) {
            let bytes = fs::read(&spritesheet).map_err(|e| {
                serde_json::json!({
                    "code": "read_spritesheet_error",
                    "message": e.to_string()
                })
            })?;
            let b64 = STANDARD.encode(&bytes);
            let ext = spritesheet
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("webp");
            let mime = if ext == "png" { "image/png" } else { "image/webp" };
            return Ok(serde_json::json!({
                "data": format!("data:{};base64,{}", mime, b64),
                "slug": slug,
            }));
        }
    }
    Err(serde_json::json!({
        "code": "pet_not_found",
        "message": format!("Pet '{}' not found in any pets root", slug)
    }))
}

#[tauri::command]
pub async fn fetch_pet_manifest() -> Result<Value, Value> {
    let resp = reqwest::get("https://petdex.crafter.run/api/manifest")
        .await
        .map_err(|e| {
            serde_json::json!({
                "code": "fetch_manifest_error",
                "message": e.to_string()
            })
        })?;
    let data: Value = resp.json().await.map_err(|e| {
        serde_json::json!({
            "code": "parse_manifest_error",
            "message": e.to_string()
        })
    })?;
    Ok(data)
}

#[tauri::command]
pub async fn install_pet(
    slug: String,
    spritesheet_url: String,
    pet_json_url: String,
    app: tauri::AppHandle,
) -> Result<Value, Value> {
    let data_dir = app.path().app_data_dir().map_err(|e| {
        serde_json::json!({"code": "path_error", "message": e.to_string()})
    })?;
    let pet_dir = data_dir.join("pets").join(&slug);
    fs::create_dir_all(&pet_dir).map_err(|e| {
        serde_json::json!({"code": "create_dir_error", "message": e.to_string()})
    })?;

    let client = reqwest::Client::new();
    let (json_resp, sprite_resp) = tokio::join!(
        client.get(&pet_json_url).send(),
        client.get(&spritesheet_url).send()
    );

    let json_bytes = json_resp
        .map_err(|e| serde_json::json!({"code": "download_error", "message": format!("pet.json: {}", e)}))?
        .bytes()
        .await
        .map_err(|e| serde_json::json!({"code": "download_error", "message": format!("pet.json body: {}", e)}))?;
    let sprite_bytes = sprite_resp
        .map_err(|e| serde_json::json!({"code": "download_error", "message": format!("spritesheet: {}", e)}))?
        .bytes()
        .await
        .map_err(|e| serde_json::json!({"code": "download_error", "message": format!("spritesheet body: {}", e)}))?;

    let ext = if spritesheet_url.ends_with(".png") { "png" } else { "webp" };

    fs::write(pet_dir.join("pet.json"), &json_bytes).map_err(|e| {
        serde_json::json!({"code": "write_error", "message": format!("pet.json: {}", e)})
    })?;
    fs::write(pet_dir.join(format!("spritesheet.{}", ext)), &sprite_bytes).map_err(|e| {
        serde_json::json!({"code": "write_error", "message": format!("spritesheet: {}", e)})
    })?;

    Ok(serde_json::json!({"ok": true, "slug": slug}))
}

#[tauri::command]
pub fn create_pet_cmd(app: tauri::AppHandle) -> Result<Value, Value> {
    pet_window::create_pet_window(&app)?;
    Ok(serde_json::json!({"ok": true}))
}

#[tauri::command]
pub fn destroy_pet_cmd(app: tauri::AppHandle) -> Result<Value, Value> {
    pet_window::destroy_pet_window(&app)?;
    Ok(serde_json::json!({"ok": true}))
}

#[tauri::command]
pub fn save_pet_state(state: PetState, app: tauri::AppHandle) -> Result<Value, Value> {
    let store = app
        .store("pet-state.json")
        .map_err(|e| serde_json::json!({"code": "store_error", "message": e.to_string()}))?;

    if let Some(slug) = &state.slug {
        store.set("activeSlug", serde_json::json!(slug));
    }
    if let Some(x) = state.x {
        store.set("x", serde_json::json!(x));
    }
    if let Some(y) = state.y {
        store.set("y", serde_json::json!(y));
    }
    if let Some(enabled) = state.enabled {
        store.set("enabled", serde_json::json!(enabled));
    }
    store.save().map_err(|e| {
        serde_json::json!({"code": "store_save_error", "message": e.to_string()})
    })?;
    Ok(serde_json::json!({"ok": true}))
}

#[tauri::command]
pub fn get_pet_state(app: tauri::AppHandle) -> Result<Value, Value> {
    let store = app
        .store("pet-state.json")
        .map_err(|e| serde_json::json!({"code": "store_error", "message": e.to_string()}))?;

    let slug = store.get("activeSlug");
    let x = store.get("x").and_then(|v| v.as_f64());
    let y = store.get("y").and_then(|v| v.as_f64());
    let enabled = store.get("enabled").and_then(|v| v.as_bool());

    Ok(serde_json::json!({
        "activeSlug": slug,
        "x": x,
        "y": y,
        "enabled": enabled,
    }))
}
