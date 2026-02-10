use std::process::Command;

const CONTAINER_PREFIX: &str = "zk-dns-proxy-";
const NOTIFY_PORT: u16 = 5300;

#[derive(Debug, Clone)]
pub struct ProxyInfo {
    pub container_id: String,
    pub bridge_ip: String,
    pub mapped_port: u16,
}

#[derive(Debug, Clone)]
pub struct DockerTarget {
    pub container_name: String,
    pub container_ip: String,
    pub network: String,
    pub internal_port: u16,
}

fn container_name(identity: &str) -> String {
    let short = &identity[..8.min(identity.len())];
    format!("{}{}", CONTAINER_PREFIX, short)
}

fn proxy_binary_path() -> std::path::PathBuf {
    // In dev: look next to the executable in resources/
    // In production: Tauri resolves resources relative to the app bundle
    let exe = std::env::current_exe().unwrap_or_default();
    let dir = exe.parent().unwrap_or(std::path::Path::new("."));

    // Dev mode: src-tauri/target/debug/ → src-tauri/resources/
    let dev_path = dir.join("../../resources/dns-proxy-linux-arm64");
    if dev_path.exists() {
        return dev_path.canonicalize().unwrap_or(dev_path);
    }

    // Tauri bundled: next to the executable
    let bundled = dir.join("resources/dns-proxy-linux-arm64");
    if bundled.exists() {
        return bundled.canonicalize().unwrap_or(bundled);
    }

    // macOS .app bundle
    let macos_bundle = dir.join("../Resources/resources/dns-proxy-linux-arm64");
    if macos_bundle.exists() {
        return macos_bundle.canonicalize().unwrap_or(macos_bundle);
    }

    dev_path
}

fn run_docker(args: &[&str]) -> Result<String, String> {
    let output = Command::new("docker")
        .args(args)
        .output()
        .map_err(|e| format!("docker not available: {}", e))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(stderr)
    }
}

pub fn detect_target(port: u16) -> Result<DockerTarget, String> {
    let output = run_docker(&[
        "ps", "--filter", &format!("publish={}", port),
        "--format", "{{.ID}}"
    ])?;
    let container_id = output.lines().next()
        .ok_or_else(|| format!("No Docker container found with port {} mapped", port))?;

    let inspect_json = run_docker(&["inspect", container_id])?;
    let parsed: serde_json::Value = serde_json::from_str(&inspect_json)
        .map_err(|e| format!("Failed to parse docker inspect: {}", e))?;

    let container = &parsed[0];
    let container_name = container["Name"].as_str().unwrap_or("").trim_start_matches('/').to_string();

    let networks = container["NetworkSettings"]["Networks"].as_object()
        .ok_or("No networks found on container")?;

    let (network, net_info) = networks.iter().next()
        .ok_or("Container has no network")?;
    let container_ip = net_info["IPAddress"].as_str().unwrap_or("").to_string();

    let internal_port = container["NetworkSettings"]["Ports"].as_object()
        .and_then(|ports| {
            ports.keys()
                .find(|k| k.contains("udp"))
                .and_then(|k| k.split('/').next()?.parse::<u16>().ok())
        })
        .unwrap_or(53);

    Ok(DockerTarget { container_name, container_ip, network: network.clone(), internal_port })
}

pub fn start(identity: &str, network: &str, target_ip: &str, target_port: u16, coredns_port: u16) -> Result<ProxyInfo, String> {
    let name = container_name(identity);
    let _ = stop(identity);

    let binary = proxy_binary_path();
    if !binary.exists() {
        return Err(format!("dns-proxy binary not found at {:?}", binary));
    }

    let binary_str = binary.to_string_lossy();
    let notify_target = format!("{}:{}", target_ip, target_port);
    let dns_target = format!("host.docker.internal:{}", coredns_port);
    let port_map = format!("127.0.0.1:0:{}/udp", NOTIFY_PORT);

    let container_id = run_docker(&[
        "run", "-d",
        "--name", &name,
        "--network", network,
        "--add-host", "host.docker.internal:host-gateway",
        "-p", &port_map,
        "-v", &format!("{}:/dns-proxy:ro", binary_str),
        "busybox:musl",
        "/dns-proxy",
        &NOTIFY_PORT.to_string(),
        &notify_target,
        &coredns_port.to_string(),
        &dns_target,
    ])?;

    std::thread::sleep(std::time::Duration::from_millis(500));

    let bridge_ip = inspect_ip(&name)?;
    let mapped_port = inspect_mapped_port(&name)?;

    Ok(ProxyInfo { container_id, bridge_ip, mapped_port })
}

pub fn stop(identity: &str) -> Result<(), String> {
    let name = container_name(identity);
    let _ = run_docker(&["rm", "-f", &name]);
    Ok(())
}

pub fn status(identity: &str) -> Option<ProxyInfo> {
    let name = container_name(identity);
    let running = run_docker(&[
        "inspect", "--format", "{{.State.Running}}", &name
    ]).ok()?;
    if running != "true" {
        return None;
    }
    let bridge_ip = inspect_ip(&name).ok()?;
    let mapped_port = inspect_mapped_port(&name).ok()?;
    Some(ProxyInfo { container_id: name, bridge_ip, mapped_port })
}

pub fn stop_all() {
    if let Ok(output) = run_docker(&[
        "ps", "-q", "--filter", &format!("name={}", CONTAINER_PREFIX)
    ]) {
        for id in output.lines() {
            if !id.is_empty() {
                let _ = run_docker(&["rm", "-f", id]);
            }
        }
    }
}

fn inspect_ip(name: &str) -> Result<String, String> {
    run_docker(&[
        "inspect", "--format",
        "{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}",
        name,
    ]).and_then(|ip| {
        if ip.is_empty() { Err("Proxy has no IP address".to_string()) } else { Ok(ip) }
    })
}

fn inspect_mapped_port(name: &str) -> Result<u16, String> {
    let output = run_docker(&[
        "inspect", "--format",
        &format!("{{{{(index (index .NetworkSettings.Ports \"{}/udp\") 0).HostPort}}}}", NOTIFY_PORT),
        name,
    ])?;
    output.parse::<u16>().map_err(|e| format!("Failed to parse mapped port: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_name_uses_identity_prefix() {
        assert_eq!(container_name("abcdef12-3456-7890"), "zk-dns-proxy-abcdef12");
    }

    #[test]
    fn container_name_handles_short_identity() {
        assert_eq!(container_name("abc"), "zk-dns-proxy-abc");
    }
}
