use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use tauri::Emitter;
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

fn pets_roots() -> Vec<PathBuf> {
    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return vec![],
    };

    let mut roots = vec![];
    for rel in &[".petdex/pets", ".codex/pets"] {
        let dir = home.join(rel);
        if dir.is_dir() {
            roots.push(dir);
        }
    }
    roots
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

#[tauri::command]
pub fn list_pets() -> Result<Value, Value> {
    let mut seen = std::collections::HashSet::new();
    let mut pets: Vec<PetInfo> = vec![];
    for root in pets_roots() {
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
pub fn read_pet_spritesheet(slug: String) -> Result<Value, Value> {
    for root in pets_roots() {
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

pub fn emit_pet_state(app: &tauri::AppHandle, state: &str, duration: Option<u64>) {
    let payload = serde_json::json!({
        "state": state,
        "duration": duration,
    });
    let _ = app.emit("pet:state-change", payload);
}

pub fn emit_pet_bubble(app: &tauri::AppHandle, text: &str, agent_source: Option<&str>) {
    let payload = serde_json::json!({
        "text": text,
        "agent_source": agent_source,
    });
    let _ = app.emit("pet:bubble", payload);
}
