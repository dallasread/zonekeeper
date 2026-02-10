pub mod commands;
pub mod config;
pub mod coredns;
pub mod docker_proxy;
pub mod paths;

use std::collections::HashMap;
use std::sync::Mutex;
use tauri::Manager;
use commands::config::{get_config, set_config, get_app_config, set_app_config, nuke_app_data};
use commands::notify::{create_identity, delete_identity, docker_proxy_status, list_identities, open_window, rename_identity, send_notify, start_docker_proxy, stop_docker_proxy};
use commands::server::{reload_server, server_status, start_server, stop_all_servers, stop_server, ServerState};
use commands::zones::{create_zone, delete_zone, list_zones, pull_zone, read_zone, save_zone};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    paths::migrate_legacy_layout();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(ServerState(Mutex::new(HashMap::new())))
        .invoke_handler(tauri::generate_handler![
            start_server,
            stop_server,
            stop_all_servers,
            reload_server,
            server_status,
            list_zones,
            create_zone,
            delete_zone,
            read_zone,
            save_zone,
            pull_zone,
            get_config,
            set_config,
            get_app_config,
            set_app_config,
            send_notify,
            open_window,
            list_identities,
            create_identity,
            rename_identity,
            delete_identity,
            nuke_app_data,
            start_docker_proxy,
            stop_docker_proxy,
            docker_proxy_status,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                let state = app_handle.state::<ServerState>();
                let mut guard = state.0.lock().unwrap();
                for (_, mut process) in guard.drain() {
                    let _ = process.stop();
                }
                docker_proxy::stop_all();
            }
        });
}
