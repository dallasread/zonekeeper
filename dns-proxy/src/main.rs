use std::env;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::thread;
use std::time::Duration;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        eprintln!("Usage: dns_proxy NOTIFY_LISTEN NOTIFY_TARGET DNS_LISTEN DNS_TARGET");
        eprintln!("  e.g. dns_proxy 5300 172.19.0.3:53 6055 host.docker.internal:6055");
        std::process::exit(1);
    }

    let notify_listen: u16 = args[1].parse().expect("NOTIFY_LISTEN must be a port number");
    let notify_target = args[2].clone();
    let dns_listen: u16 = args[3].parse().expect("DNS_LISTEN must be a port number");
    let dns_target = args[4].clone();

    let dns_target_tcp = dns_target.clone();

    // UDP: forward NOTIFY packets (skip if listen port is 0)
    let t1 = if notify_listen > 0 {
        let target = notify_target;
        Some(thread::spawn(move || {
            udp_proxy(notify_listen, &target, "notify");
        }))
    } else {
        eprintln!("[notify] disabled");
        None
    };

    // UDP: forward SOA queries from secondary to host CoreDNS
    let t2 = thread::spawn(move || {
        udp_proxy(dns_listen, &dns_target, "dns-udp");
    });

    // TCP: forward AXFR connections from secondary to host CoreDNS
    let t3 = thread::spawn(move || {
        tcp_proxy(dns_listen, &dns_target_tcp, "dns-tcp");
    });

    if let Some(t) = t1 { let _ = t.join(); }
    let _ = t2.join();
    let _ = t3.join();
}

fn udp_proxy(listen_port: u16, target: &str, label: &str) {
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", listen_port))
        .unwrap_or_else(|e| panic!("[{}] bind failed on port {}: {}", label, listen_port, e));

    eprintln!("[{}] listening on UDP :{}", label, listen_port);

    let mut buf = [0u8; 4096];
    loop {
        let (len, src) = match socket.recv_from(&mut buf) {
            Ok(r) => r,
            Err(e) => { eprintln!("[{}] recv error: {}", label, e); continue; }
        };

        // Create a new socket per request to forward and receive response
        let fwd = match UdpSocket::bind("0.0.0.0:0") {
            Ok(s) => s,
            Err(e) => { eprintln!("[{}] fwd bind error: {}", label, e); continue; }
        };
        let _ = fwd.set_read_timeout(Some(Duration::from_secs(5)));

        if let Err(e) = fwd.send_to(&buf[..len], target) {
            eprintln!("[{}] fwd send error: {}", label, e);
            continue;
        }

        // Wait for response and relay back
        match fwd.recv_from(&mut buf) {
            Ok((rlen, _)) => {
                let _ = socket.send_to(&buf[..rlen], src);
            }
            Err(_) => {
                // No response (timeout) — that's ok for NOTIFY
            }
        }
    }
}

fn tcp_proxy(listen_port: u16, target: &str, label: &str) {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", listen_port))
        .unwrap_or_else(|e| panic!("[{}] bind failed on port {}: {}", label, listen_port, e));

    eprintln!("[{}] listening on TCP :{}", label, listen_port);

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(e) => { eprintln!("[{}] accept error: {}", label, e); continue; }
        };

        let target = target.to_string();
        let label = label.to_string();
        thread::spawn(move || {
            if let Err(e) = relay_tcp(stream, &target) {
                eprintln!("[{}] relay error: {}", label, e);
            }
        });
    }
}

fn relay_tcp(mut client: TcpStream, target: &str) -> std::io::Result<()> {
    let mut upstream = TcpStream::connect(target)?;
    let _ = client.set_read_timeout(Some(Duration::from_secs(30)));
    let _ = upstream.set_read_timeout(Some(Duration::from_secs(30)));

    let mut client_clone = client.try_clone()?;
    let mut upstream_clone = upstream.try_clone()?;

    let t1 = thread::spawn(move || {
        let mut buf = [0u8; 65535];
        loop {
            match client.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => { if upstream.write_all(&buf[..n]).is_err() { break; } }
            }
        }
        let _ = upstream.shutdown(std::net::Shutdown::Write);
    });

    let t2 = thread::spawn(move || {
        let mut buf = [0u8; 65535];
        loop {
            match upstream_clone.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => { if client_clone.write_all(&buf[..n]).is_err() { break; } }
            }
        }
        let _ = client_clone.shutdown(std::net::Shutdown::Write);
    });

    let _ = t1.join();
    let _ = t2.join();
    Ok(())
}
