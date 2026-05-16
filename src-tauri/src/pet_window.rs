use serde_json::Value;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

const PET_WINDOW_LABEL: &str = "pet";

pub fn create_pet_window(app: &tauri::AppHandle) -> Result<(), Value> {
    if app.get_webview_window(PET_WINDOW_LABEL).is_some() {
        return Ok(());
    }

    WebviewWindowBuilder::new(
        app,
        PET_WINDOW_LABEL,
        WebviewUrl::App("index.html#/pet".into()),
    )
    .title("Raven Pet")
    .inner_size(140.0, 180.0)
    .min_inner_size(140.0, 140.0)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .visible_on_all_workspaces(true)
    .skip_taskbar(true)
    .hidden_title(true)
    .shadow(false)
    .build()
    .map_err(|e| {
        serde_json::json!({
            "code": "pet_window_create_failed",
            "message": e.to_string()
        })
    })?;

    Ok(())
}

pub fn destroy_pet_window(app: &tauri::AppHandle) -> Result<(), Value> {
    if let Some(window) = app.get_webview_window(PET_WINDOW_LABEL) {
        window.close().map_err(|e| {
            serde_json::json!({
                "code": "pet_window_close_failed",
                "message": e.to_string()
            })
        })?;
    }
    Ok(())
}
