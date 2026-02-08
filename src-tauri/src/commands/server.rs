use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};
use crate::config;
use crate::coredns::corefile;
use crate::coredns::process::CoreDnsProcess;
use super::zones;

pub struct ServerState(pub Mutex<HashMap<String, CoreDnsProcess>>);

#[tauri::command]
pub async fn start_server(identity: String, port: u16, app: AppHandle, state: State<'_, ServerState>) -> Result<String, String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(process) = guard.get_mut(&identity) {
        if process.has_exited() {
            guard.remove(&identity);
        } else {
            return Err(format!("Server already running for '{}'", identity));
        }
    }

    let zone_names = zones::zone_names(&identity).map_err(|e| e.to_string())?;
    let cfg = config::read_config(&identity);
    corefile::write_corefile(&identity, &zone_names, port, cfg.accept_transfers, &cfg.transfer_from).map_err(|e| e.to_string())?;

    let process = CoreDnsProcess::start(app, &identity, port).map_err(|e| e.to_string())?;
    guard.insert(identity, process);
    Ok("started".to_string())
}

#[tauri::command]
pub async fn stop_server(identity: String, app: AppHandle, state: State<'_, ServerState>) -> Result<String, String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(mut process) = guard.remove(&identity) {
        process.stop().map_err(|e| e.to_string())?;
        let name = config::read_config(&identity).name;
        let display = if name.is_empty() { identity.clone() } else { name };
        let _ = app.emit(&format!("log-line-{}", identity), format!("Server '{}' stopped", display));
    }
    Ok("stopped".to_string())
}

#[tauri::command]
pub async fn reload_server(identity: String, app: AppHandle, state: State<'_, ServerState>) -> Result<(), String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(process) = guard.get(&identity) {
        process.reload();
        let name = config::read_config(&identity).name;
        let display = if name.is_empty() { identity.clone() } else { name };
        let _ = app.emit(&format!("log-line-{}", identity), format!("Reloading zones for '{}'", display));
    }
    Ok(())
}

#[tauri::command]
pub async fn server_status(identity: String, state: State<'_, ServerState>) -> Result<bool, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    Ok(guard.contains_key(&identity))
}

#[tauri::command]
pub async fn stop_all_servers(state: State<'_, ServerState>) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    for (_, mut process) in guard.drain() {
        let _ = process.stop();
    }
    Ok(())
}
