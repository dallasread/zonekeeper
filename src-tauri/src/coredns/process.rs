use std::fs;
use std::io::{self, BufRead, BufReader};
use std::net::UdpSocket;
use std::process::{Child, Command, Stdio};
use tauri::{AppHandle, Emitter};
use crate::paths;
use super::corefile;

pub struct CoreDnsProcess {
    child: Child,
    identity: String,
    port: u16,
}

fn process_log_line(line: &str, handle: &AppHandle, evt: &str, identity: &str) {
    let is_error = line.contains("[ERROR]");
    let is_warn = line.contains("[WARNING]");

    if line.contains("Transferred:") {
        if let Some(zone) = extract_zone_after(line, "Transferred: ") {
            let from = extract_after_marker(line, "from ").unwrap_or_default();
            let msg = if from.is_empty() {
                format!("Transferred {}", zone)
            } else {
                format!("Transferred {} from {}", zone, from)
            };
            let _ = handle.emit(evt, msg);
            let _ = handle.emit(&format!("sync-changed-{}", identity), zone);
        }
    } else if line.contains("Failed to transfer") {
        if let Some(zone) = extract_zone_after(line, "Failed to transfer ") {
            let reason = extract_after_zone(line, &zone).unwrap_or_default();
            let _ = handle.emit(evt, format!("[ERROR] Transfer failed for {}{}", zone, reason));
        }
    } else if line.contains("masters failed to transfer") {
        if let Some(zone) = extract_zone_before(line, "masters") {
            let reason = line.find("transfer")
                .and_then(|i| {
                    let after = line[i + "transfer".len()..].trim().trim_start_matches(':').trim();
                    if after.is_empty() { None } else { Some(format!(": {}", after)) }
                })
                .unwrap_or_default();
            let _ = handle.emit(evt, format!("[ERROR] Transfer failed for {}{}", zone, reason));
        }
    } else if line.contains("Dropping notify") {
        if let Some(zone) = extract_zone_after(line, "for ") {
            let src = extract_between(line, "from ", " for").unwrap_or_default();
            let _ = handle.emit(evt, format!("Received NOTIFY for {} from {}", zone, src));
        }
    } else if line.contains("Outgoing transfer") {
        // "Outgoing transfer of 4 records of zone "zone.com." to 127.0.0.1 for 1 SOA serial"
        if let Some(zone) = extract_zone_after(line, "of zone ") {
            let to = extract_after_marker(line, "to ").unwrap_or_default();
            // Skip self-referential AXFR (app reading from its own CoreDNS)
            if to.starts_with("127.0.0.1") { return; }
            let msg = if to.is_empty() {
                format!("Serving {} via AXFR", zone)
            } else {
                format!("Serving {} via AXFR to {}", zone, to)
            };
            let _ = handle.emit(evt, msg);
        }
    } else if is_error || is_warn {
        // Pass through other errors/warnings with clean prefix
        let cleaned = line.replace("[ERROR] ", "").replace("[WARNING] ", "");
        let cleaned = cleaned.trim();
        // Strip plugin prefix
        let cleaned = if let Some(idx) = cleaned.find("plugin/") {
            let after = &cleaned[idx..];
            after.find(": ").map(|i| after[i + 2..].trim()).unwrap_or(cleaned)
        } else {
            cleaned
        };
        if !cleaned.is_empty() {
            let prefix = if is_error { "[ERROR] " } else { "[WARNING] " };
            let _ = handle.emit(evt, format!("{}{}", prefix, cleaned));
        }
    }
    // All other CoreDNS lines (startup noise, query logs, reload notices) are silently dropped
}

fn extract_after_marker(line: &str, marker: &str) -> Option<String> {
    let idx = line.find(marker)?;
    let after = &line[idx + marker.len()..];
    let word = after.split_whitespace().next()?.trim_end_matches('.').trim_matches('"');
    if word.is_empty() { None } else { Some(word.to_string()) }
}

fn extract_zone_before(line: &str, marker: &str) -> Option<String> {
    let idx = line.find(marker)?;
    let before = line[..idx].trim().trim_end_matches('-').trim();
    let fqdn = before.rsplit_once(' ').map(|(_, w)| w).unwrap_or(before);
    let zone = fqdn.trim_matches('\'').trim_matches('`').trim_end_matches('.').to_lowercase();
    if zone.contains('.') { Some(zone) } else { None }
}

fn extract_zone_after(line: &str, marker: &str) -> Option<String> {
    let idx = line.find(marker)?;
    let after = &line[idx + marker.len()..];
    let zone = after.split_whitespace().next()?.trim_end_matches('.').to_lowercase();
    if zone.contains('.') { Some(zone) } else { None }
}

fn extract_after_zone(line: &str, zone: &str) -> Option<String> {
    let fqdn = format!("{}.", zone);
    let idx = line.find(&fqdn)? + fqdn.len();
    let rest = line[idx..].trim().trim_start_matches(':').trim();
    if rest.is_empty() { None } else { Some(format!(": {}", rest)) }
}

fn extract_between(line: &str, start: &str, end: &str) -> Option<String> {
    let s = line.find(start)? + start.len();
    let e = line[s..].find(end)? + s;
    Some(line[s..e].trim().to_string())
}

impl CoreDnsProcess {
    pub fn start(app_handle: AppHandle, identity: &str, port: u16) -> io::Result<Self> {
        Self::kill_stale(identity, port);

        if UdpSocket::bind(("127.0.0.1", port)).is_err() {
            return Err(io::Error::new(
                io::ErrorKind::AddrInUse,
                format!("Port {} is already in use by another process", port),
            ));
        }
        // Socket drops here, freeing the port for CoreDNS

        let coredns_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("coredns")))
            .filter(|p| p.exists())
            .unwrap_or_else(|| "coredns".into());
        let mut child = Command::new(coredns_path)
            .arg("-conf")
            .arg(corefile::corefile_path(identity, port).to_string_lossy().to_string())
            .arg("-dns.port")
            .arg(port.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let _ = fs::write(Self::pid_path(identity, port), child.id().to_string());

        let event_name = format!("log-line-{}", identity);
        let name = crate::config::read_config(identity).name;
        let display = if name.is_empty() { identity.to_string() } else { name };
        let _ = app_handle.emit(&event_name, format!("Server '{}' starting on port {}", display, port));

        if let Some(stdout) = child.stdout.take() {
            let handle = app_handle.clone();
            let evt = event_name.clone();
            let id = identity.to_string();
            std::thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    match line {
                        Ok(line) => process_log_line(&line, &handle, &evt, &id),
                        Err(e) => { let _ = handle.emit(&evt, format!("stdout read error: {}", e)); break; }
                    }
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let handle = app_handle;
            let evt = event_name;
            let id = identity.to_string();
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    match line {
                        Ok(line) => process_log_line(&line, &handle, &evt, &id),
                        Err(e) => { let _ = handle.emit(&evt, format!("stderr read error: {}", e)); break; }
                    }
                }
                let _ = handle.emit(&format!("server-exited-{}", id), ());
            });
        }

        Ok(CoreDnsProcess { child, identity: identity.to_string(), port })
    }

    pub fn reload(&self) {
        let pid = self.child.id() as i32;
        unsafe { libc::kill(pid, libc::SIGUSR1); }
    }

    pub fn has_exited(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(Some(_)))
    }

    pub fn stop(&mut self) -> io::Result<()> {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_file(Self::pid_path(&self.identity, self.port));
        Ok(())
    }

    fn kill_stale(identity: &str, port: u16) {
        if let Ok(pid_str) = fs::read_to_string(Self::pid_path(identity, port)) {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                unsafe { libc::kill(pid, libc::SIGTERM); }
                std::thread::sleep(std::time::Duration::from_millis(200));
                unsafe { libc::kill(pid, libc::SIGKILL); }
            }
            let _ = fs::remove_file(Self::pid_path(identity, port));
        }
    }

    fn pid_path(identity: &str, port: u16) -> std::path::PathBuf {
        paths::identity_dir(identity).join(format!("coredns.{}.pid", port))
    }
}

impl Drop for CoreDnsProcess {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_zone_from_transferred() {
        assert_eq!(
            extract_zone_after("[INFO] plugin/file: Transferred: xmncvbcv.com. from 127.0.0.1:1055", "Transferred: "),
            Some("xmncvbcv.com".to_string())
        );
    }

    #[test]
    fn extract_zone_from_transfer_failed() {
        assert_eq!(
            extract_zone_before("[WARNING] plugin/secondary: All 'example.org.' masters failed to transfer", "masters"),
            Some("example.org".to_string())
        );
    }

    #[test]
    fn extract_zone_from_dropping_notify() {
        assert_eq!(
            extract_zone_after("[WARNING] Dropping notify from 192.168.1.1 for test.com.", "for "),
            Some("test.com".to_string())
        );
    }

    #[test]
    fn extract_zone_from_axfr_in() {
        assert_eq!(
            extract_zone_after("[INFO] 127.0.0.1:51740 - 24115 \"AXFR IN test.com. udp 28\"", "AXFR IN "),
            Some("test.com".to_string())
        );
    }

    #[test]
    fn extract_between_notify_source() {
        assert_eq!(
            extract_between("[WARNING] Dropping notify from 192.168.1.1 for test.com.", "from ", " for"),
            Some("192.168.1.1".to_string())
        );
    }
}

