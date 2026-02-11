use crate::config;

#[tauri::command]
pub async fn get_config(identity: String) -> Result<config::Config, String> {
    Ok(config::read_config(&identity))
}

#[tauri::command]
pub async fn set_config(identity: String, port: u16, notify_target: String, accept_transfers: bool, transfer_from: String, auto_bump_serial: bool, auto_format: bool, notify_servers: Vec<String>) -> Result<(), String> {
    let existing = config::read_config(&identity);
    let cfg = config::Config { name: existing.name, port, font_size: existing.font_size, notify_target, accept_transfers, transfer_from, auto_bump_serial, auto_format, notify_servers, zone_notify: existing.zone_notify };
    config::save_config(&identity, &cfg).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_zone_notify(identity: String, zone: String, targets: Vec<String>) -> Result<(), String> {
    let mut cfg = config::read_config(&identity);
    if targets.is_empty() {
        cfg.zone_notify.remove(&zone);
    } else {
        cfg.zone_notify.insert(zone, targets);
    }
    config::save_config(&identity, &cfg).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_app_config() -> Result<config::AppConfig, String> {
    Ok(config::read_app_config())
}

#[tauri::command]
pub async fn set_app_config(font_size: u8) -> Result<(), String> {
    let cfg = config::AppConfig { font_size };
    config::save_app_config(&cfg).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn nuke_app_data(state: tauri::State<'_, crate::commands::server::ServerState>) -> Result<(), String> {
    // Stop all running servers first
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    for (_, mut process) in guard.drain() {
        let _ = process.stop();
    }
    drop(guard);

    let app_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("Library/Application Support/zonekeeper");

    if app_dir.exists() {
        std::fs::remove_dir_all(&app_dir).map_err(|e| e.to_string())?;
    }

    Ok(())
}
