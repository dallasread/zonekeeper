use std::fs;
use std::path::PathBuf;
use crate::paths;

pub fn generate_corefile(identity: &str, zone_names: &[String], port: u16, accept_transfers: bool, transfer_from: &str) -> String {
    let zones_dir = paths::zones_dir(identity);
    let mut blocks = Vec::new();

    for name in zone_names {
        let zone_file = zones_dir.join(format!("{}.zone", name));

        let block = if accept_transfers && !transfer_from.is_empty() {
            let source = if transfer_from.contains(':') {
                transfer_from.to_string()
            } else {
                format!("{}:53", transfer_from)
            };
            format!(
                "{name}:{port} {{\n    secondary {{\n        transfer from {source}\n    }}\n    transfer {{\n        to *\n    }}\n    log\n}}",
                name = name, port = port, source = source
            )
        } else {
            format!(
                "{name}:{port} {{\n    file \"{path}\"\n    transfer {{\n        to *\n    }}\n    log\n}}",
                name = name, port = port, path = zone_file.to_string_lossy()
            )
        };

        blocks.push(block);
    }

    blocks.join("\n\n")
}

pub fn corefile_path(identity: &str, port: u16) -> PathBuf {
    paths::identity_dir(identity).join(format!("Corefile.{}", port))
}

pub fn write_corefile(identity: &str, zone_names: &[String], port: u16, accept_transfers: bool, transfer_from: &str) -> std::io::Result<()> {
    let content = generate_corefile(identity, zone_names, port, accept_transfers, transfer_from);
    fs::write(corefile_path(identity, port), content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_corefile_uses_given_port() {
        let zones = vec!["test.local".to_string()];
        let result = generate_corefile("default", &zones, 5353, false, "");
        assert!(result.contains("test.local:5353"));
        assert!(!result.contains(":1053"));
    }

    #[test]
    fn generate_corefile_default_port() {
        let zones = vec!["example.com".to_string()];
        let result = generate_corefile("default", &zones, 1053, false, "");
        assert!(result.contains("example.com:1053"));
    }

    #[test]
    fn generate_corefile_multiple_zones() {
        let zones = vec!["a.local".to_string(), "b.local".to_string()];
        let result = generate_corefile("default", &zones, 8053, false, "");
        assert!(result.contains("a.local:8053"));
        assert!(result.contains("b.local:8053"));
        assert!(result.contains("\n\n"));
    }

    #[test]
    fn generate_corefile_empty_zones() {
        let zones: Vec<String> = vec![];
        let result = generate_corefile("default", &zones, 1053, false, "");
        assert_eq!(result, "");
    }

    #[test]
    fn generate_corefile_accept_transfers() {
        let zones = vec!["test.local".to_string()];
        let result = generate_corefile("default", &zones, 1054, true, "127.0.0.1:1053");
        assert!(result.contains("secondary"));
        assert!(result.contains("transfer from 127.0.0.1:1053"));
        assert!(result.contains("transfer {\n        to *\n    }"));
        assert!(!result.contains("file"));
        assert_eq!(result.matches("transfer").count(), 2); // "transfer from" + "transfer { to * }"
    }

    #[test]
    fn generate_corefile_primary_has_transfer_block() {
        let zones = vec!["test.local".to_string()];
        let result = generate_corefile("default", &zones, 1053, false, "");
        assert!(result.contains("file \""));
        assert!(result.contains("transfer {\n        to *\n    }"));
    }

    #[test]
    fn generate_corefile_accept_transfers_no_source() {
        let zones = vec!["test.local".to_string()];
        let result = generate_corefile("default", &zones, 1054, true, "");
        assert!(result.contains("file"));
        assert!(!result.contains("secondary"));
    }

    #[test]
    fn generate_corefile_no_accept_transfers() {
        let zones = vec!["test.local".to_string()];
        let result = generate_corefile("default", &zones, 1053, false, "");
        assert!(result.contains("file"));
        assert!(!result.contains("secondary"));
    }

    #[test]
    fn corefile_path_scoped_to_identity() {
        let path = corefile_path("secondary", 1054);
        assert!(path.to_string_lossy().contains("identities/secondary/Corefile.1054"));
    }
}
