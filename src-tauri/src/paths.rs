use serde::Serialize;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Clone)]
pub struct IdentityInfo {
    pub id: String,
    pub name: String,
}

fn app_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Library/Application Support/zonekeeper")
}

pub fn identity_dir(identity: &str) -> PathBuf {
    app_dir().join("identities").join(identity)
}

pub fn ensure_identity_dir(identity: &str) -> PathBuf {
    let dir = identity_dir(identity);
    fs::create_dir_all(&dir).ok();
    dir
}

pub fn zones_dir(identity: &str) -> PathBuf {
    let dir = identity_dir(identity).join("zones");
    fs::create_dir_all(&dir).ok();
    dir
}

pub fn config_path(identity: &str) -> PathBuf {
    identity_dir(identity).join("config.json")
}

pub fn app_config_path() -> PathBuf {
    app_dir().join("app_config.json")
}

pub fn create_identity(name: &str) -> IdentityInfo {
    let id = uuid::Uuid::now_v7().to_string();
    let dir = app_dir().join("identities").join(&id);
    fs::create_dir_all(&dir).ok();
    IdentityInfo { id, name: name.to_string() }
}

pub fn list_identities() -> Vec<IdentityInfo> {
    let dir = app_dir().join("identities");
    let mut identities = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if !entry.path().is_dir() { continue; }
            let id = entry.file_name().to_string_lossy().to_string();
            let config_path = entry.path().join("config.json");
            let name = fs::read_to_string(&config_path)
                .ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .and_then(|v| v["name"].as_str().map(|s| s.to_string()))
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| id.clone());
            identities.push(IdentityInfo { id, name });
        }
    }
    identities.sort_by(|a, b| a.name.cmp(&b.name));
    identities
}

pub fn migrate_legacy_layout() {
    let app = app_dir();
    let old_zones = app.join("zones");
    let old_config = app.join("config.json");

    if old_zones.exists() || old_config.exists() {
        let default_dir = app.join("identities").join("default");
        fs::create_dir_all(&default_dir).ok();

        if old_zones.exists() && !default_dir.join("zones").exists() {
            let _ = fs::rename(&old_zones, default_dir.join("zones"));
        }

        if old_config.exists() && !default_dir.join("config.json").exists() {
            let _ = fs::rename(&old_config, default_dir.join("config.json"));
        }

        // Clean up old per-port Corefiles and PIDs at root level
        if let Ok(entries) = fs::read_dir(&app) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with("Corefile.") || name.starts_with("coredns.") {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
    }

    // Migrate named directories to UUID7
    let identities_dir = app.join("identities");
    if !identities_dir.exists() { return; }

    if let Ok(entries) = fs::read_dir(&identities_dir) {
        for entry in entries.flatten() {
            if !entry.path().is_dir() { continue; }
            let dir_name = entry.file_name().to_string_lossy().to_string();
            if uuid::Uuid::parse_str(&dir_name).is_ok() { continue; }

            let id = uuid::Uuid::now_v7().to_string();
            let new_dir = identities_dir.join(&id);
            if fs::rename(entry.path(), &new_dir).is_err() { continue; }

            // Inject name into config.json
            let cfg_path = new_dir.join("config.json");
            let mut config: serde_json::Value = fs::read_to_string(&cfg_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or(serde_json::json!({}));

            config["name"] = serde_json::json!(dir_name);
            if let Ok(json) = serde_json::to_string_pretty(&config) {
                let _ = fs::write(&cfg_path, json);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_dir_contains_id() {
        let dir = identity_dir("abc-123");
        assert!(dir.to_string_lossy().contains("identities/abc-123"));
    }

    #[test]
    fn zones_dir_is_under_identity() {
        let dir = zones_dir("myid");
        assert!(dir.to_string_lossy().contains("identities/myid/zones"));
    }

    #[test]
    fn config_path_is_under_identity() {
        let path = config_path("some-uuid");
        assert!(path.to_string_lossy().contains("identities/some-uuid/config.json"));
    }

    #[test]
    fn create_identity_returns_uuid() {
        let info = create_identity("test-server");
        assert!(uuid::Uuid::parse_str(&info.id).is_ok());
        assert_eq!(info.name, "test-server");
    }
}
