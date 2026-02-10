use std::net::UdpSocket;
use std::time::Duration;
use tauri::{AppHandle, Manager, WebviewWindowBuilder, WebviewUrl};
use crate::{config, docker_proxy, paths};

fn encode_name(name: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    for label in name.trim_end_matches('.').split('.') {
        buf.push(label.len() as u8);
        buf.extend_from_slice(label.as_bytes());
    }
    buf.push(0);
    buf
}

fn build_notify_packet(zone: &str) -> Vec<u8> {
    let id: u16 = rand_id();
    let mut pkt = Vec::with_capacity(64);

    // Header
    pkt.extend_from_slice(&id.to_be_bytes());
    pkt.extend_from_slice(&[0x24, 0x00]); // QR=0, OPCODE=4 (NOTIFY), AA=1
    pkt.extend_from_slice(&[0x00, 0x01]); // QDCOUNT=1
    pkt.extend_from_slice(&[0x00, 0x00]); // ANCOUNT=0
    pkt.extend_from_slice(&[0x00, 0x00]); // NSCOUNT=0
    pkt.extend_from_slice(&[0x00, 0x00]); // ARCOUNT=0

    // Question
    pkt.extend_from_slice(&encode_name(zone));
    pkt.extend_from_slice(&[0x00, 0x06]); // QTYPE=SOA
    pkt.extend_from_slice(&[0x00, 0x01]); // QCLASS=IN

    pkt
}

fn rand_id() -> u16 {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (t & 0xFFFF) as u16
}

fn is_loopback_target(addr: &std::net::SocketAddr) -> bool {
    match addr {
        std::net::SocketAddr::V4(a) => a.ip().is_loopback(),
        std::net::SocketAddr::V6(a) => a.ip().is_loopback(),
    }
}

fn send_udp_notify(pkt: &[u8], addr: &std::net::SocketAddr) -> Result<String, String> {
    let bind_addr = match addr {
        std::net::SocketAddr::V6(_) => "[::]:0",
        std::net::SocketAddr::V4(_) => "0.0.0.0:0",
    };
    let socket = UdpSocket::bind(bind_addr).map_err(|e| format!("Bind failed: {}", e))?;
    socket.set_read_timeout(Some(Duration::from_secs(3))).ok();
    socket.send_to(pkt, addr).map_err(|e| format!("Send failed: {}", e))?;

    let mut buf = [0u8; 512];
    match socket.recv_from(&mut buf) {
        Ok((len, _)) if len >= 4 => {
            let rcode = buf[3] & 0x0F;
            match rcode {
                0 => Ok(format!("NOTIFY accepted by {}", addr)),
                _ => Err(format!("NOTIFY refused by {} (rcode={})", addr, rcode)),
            }
        }
        Ok(_) => Err(format!("Invalid response from {}", addr)),
        Err(_) => Ok(format!("NOTIFY sent to {} (no response)", addr)),
    }
}

#[tauri::command]
pub async fn send_notify(zone: String, target: String, identity: String, coredns_port: u16) -> Result<String, String> {
    let zone = zone.to_lowercase();
    let target = if !target.contains(':') {
        format!("{}:53", target)
    } else {
        target
    };

    let addr: std::net::SocketAddr = target.parse().map_err(|e| format!("Invalid target: {}", e))?;
    let pkt = build_notify_packet(&zone);

    if !is_loopback_target(&addr) {
        return send_udp_notify(&pkt, &addr);
    }

    // Loopback target — try Docker proxy, fall back to direct send
    let proxy = match docker_proxy::status(&identity) {
        Some(info) => Some(info),
        None => {
            match docker_proxy::detect_target(addr.port()) {
                Ok(docker_target) => Some(docker_proxy::start(
                    &identity,
                    &docker_target.network,
                    &docker_target.container_ip,
                    docker_target.internal_port,
                    coredns_port,
                ).map_err(|e| format!("Docker proxy failed to start: {}", e))?),
                Err(_) => None,
            }
        }
    };

    if proxy.is_none() {
        return send_udp_notify(&pkt, &addr);
    }
    let proxy = proxy.unwrap();

    let proxy_addr: std::net::SocketAddr = format!("127.0.0.1:{}", proxy.mapped_port)
        .parse().map_err(|e| format!("Invalid proxy address: {}", e))?;

    let result = send_udp_notify(&pkt, &proxy_addr)?;
    Ok(format!("{}. Master IP: {}:{}", result, proxy.bridge_ip, coredns_port))
}

#[tauri::command]
pub async fn start_docker_proxy(identity: String, target_port: u16, coredns_port: u16) -> Result<String, String> {
    let docker_target = docker_proxy::detect_target(target_port)?;
    let info = docker_proxy::start(
        &identity,
        &docker_target.network,
        &docker_target.container_ip,
        docker_target.internal_port,
        coredns_port,
    )?;
    Ok(format!("Proxy started. Master IP: {}:{}", info.bridge_ip, coredns_port))
}

#[tauri::command]
pub async fn stop_docker_proxy(identity: String) -> Result<(), String> {
    docker_proxy::stop(&identity)
}

#[tauri::command]
pub async fn docker_proxy_status(identity: String) -> Result<Option<String>, String> {
    Ok(docker_proxy::status(&identity).map(|info| info.bridge_ip))
}

#[tauri::command]
pub async fn open_window(app: AppHandle, identity: String, port: u16) -> Result<(), String> {
    let label = format!("zk-{}", &identity[..8.min(identity.len())]);
    if app.get_webview_window(&label).is_some() {
        return Err("Window already open".to_string());
    }
    let cfg = config::read_config(&identity);
    let title = format!("ZoneKeeper — {} :{}", cfg.name, port);
    WebviewWindowBuilder::new(&app, &label, WebviewUrl::App(format!("index.html?identity={}&port={}", identity, port).into()))
        .title(title)
        .inner_size(1200.0, 800.0)
        .center()
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn list_identities() -> Result<Vec<paths::IdentityInfo>, String> {
    Ok(paths::list_identities())
}

#[tauri::command]
pub async fn create_identity(name: String) -> Result<paths::IdentityInfo, String> {
    if name.is_empty() {
        return Err("Server name cannot be empty".to_string());
    }
    let info = paths::create_identity(&name);
    let mut cfg = config::Config::default();
    cfg.name = name;
    config::save_config(&info.id, &cfg).map_err(|e| e.to_string())?;
    Ok(info)
}

#[tauri::command]
pub async fn rename_identity(identity: String, new_name: String) -> Result<(), String> {
    if new_name.is_empty() {
        return Err("Server name cannot be empty".to_string());
    }
    let mut cfg = config::read_config(&identity);
    cfg.name = new_name;
    config::save_config(&identity, &cfg).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_identity(identity: String) -> Result<(), String> {
    let remaining = paths::list_identities().into_iter().filter(|i| i.id != identity).count();
    if remaining == 0 {
        return Err("Cannot delete the last server".to_string());
    }
    let dir = paths::identity_dir(&identity);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_name_works() {
        let encoded = encode_name("example.com");
        assert_eq!(encoded, vec![7, b'e', b'x', b'a', b'm', b'p', b'l', b'e', 3, b'c', b'o', b'm', 0]);
    }

    #[test]
    fn encode_name_strips_trailing_dot() {
        let encoded = encode_name("example.com.");
        assert_eq!(encoded, vec![7, b'e', b'x', b'a', b'm', b'p', b'l', b'e', 3, b'c', b'o', b'm', 0]);
    }

    #[test]
    fn notify_packet_has_correct_header() {
        let pkt = build_notify_packet("test.local");
        assert_eq!(pkt[2], 0x24); // OPCODE=4, AA=1
        assert_eq!(pkt[3], 0x00);
        assert_eq!(pkt[4..6], [0x00, 0x01]); // QDCOUNT=1
    }

    #[test]
    fn notify_packet_has_soa_question() {
        let pkt = build_notify_packet("test.local");
        // header(12) + 1+"test"(5) + 1+"local"(6) + null(1) = 24
        let qname_end = 12 + 5 + 6 + 1;
        assert_eq!(pkt[qname_end..qname_end + 2], [0x00, 0x06]); // SOA
        assert_eq!(pkt[qname_end + 2..qname_end + 4], [0x00, 0x01]); // IN
    }
}
