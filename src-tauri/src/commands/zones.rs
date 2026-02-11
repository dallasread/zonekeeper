use serde::Serialize;
use std::fs;
use crate::config;
use crate::paths;
use crate::coredns::corefile;

pub fn zone_names(identity: &str) -> std::io::Result<Vec<String>> {
    let dir = paths::zones_dir(identity);
    let mut names = Vec::new();

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("zone") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let lower = stem.to_lowercase();
                    if lower != stem {
                        let new_path = dir.join(format!("{}.zone", lower));
                        let _ = fs::rename(&path, &new_path);
                    }
                    names.push(lower);
                }
            }
        }
    }

    names.sort();
    names.dedup();
    Ok(names)
}

fn regenerate_corefile(identity: &str, port: u16) {
    if let Ok(names) = zone_names(identity) {
        let cfg = config::read_config(identity);
        let _ = corefile::write_corefile(identity, &names, port, cfg.accept_transfers, &cfg.transfer_from);
    }
}

#[derive(Serialize, Clone)]
pub struct ZoneInfo {
    pub name: String,
    pub content: String,
}

#[tauri::command]
pub async fn list_zones(identity: String) -> Result<Vec<ZoneInfo>, String> {
    let dir = paths::zones_dir(&identity);
    let names = zone_names(&identity).map_err(|e| e.to_string())?;
    Ok(names.into_iter().map(|name| {
        let path = dir.join(format!("{}.zone", name));
        let content = fs::read_to_string(&path).unwrap_or_default();
        ZoneInfo { name, content }
    }).collect())
}

fn zone_template(name: &str) -> String {
    format!(
        "$TTL 3600
@  3600  IN  SOA  ns1.{name}. admin.{name}. (
    1       ; Serial
    3600    ; Refresh
    900     ; Retry
    604800  ; Expire
    86400   ; Minimum TTL
)

@       3600  IN  NS   ns1.{name}.
ns1     3600  IN  A    127.0.0.1
"
    )
}

#[tauri::command]
pub async fn create_zone(identity: String, name: String, port: u16) -> Result<ZoneInfo, String> {
    let name = name.to_lowercase();
    let path = paths::ensure_zones_dir(&identity).join(format!("{}.zone", name));

    if path.exists() {
        return Err(format!("Zone '{}' already exists", name));
    }

    let content = zone_template(&name);
    fs::write(&path, &content).map_err(|e| e.to_string())?;
    regenerate_corefile(&identity, port);

    Ok(ZoneInfo { name, content })
}

#[tauri::command]
pub async fn delete_zone(identity: String, name: String, port: u16) -> Result<(), String> {
    let name = name.to_lowercase();
    let path = paths::zones_dir(&identity).join(format!("{}.zone", name));
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    regenerate_corefile(&identity, port);
    Ok(())
}

#[tauri::command]
pub async fn read_zone(identity: String, name: String) -> Result<String, String> {
    let name = name.to_lowercase();
    let path = paths::zones_dir(&identity).join(format!("{}.zone", name));
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_zone(identity: String, name: String, content: String, port: u16) -> Result<String, String> {
    let name = name.to_lowercase();
    let path = paths::ensure_zones_dir(&identity).join(format!("{}.zone", name));
    let cfg = config::read_config(&identity);
    let content = if cfg.auto_bump_serial { bump_serial(&content) } else { content };
    let content = if cfg.auto_format { format_zone(&content) } else { content };
    fs::write(&path, &content).map_err(|e| e.to_string())?;
    regenerate_corefile(&identity, port);
    Ok(content)
}

fn bump_serial(content: &str) -> String {
    let today = {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // days since epoch → date components
        let days = (now / 86400) as i64;
        let mut y = 1970i64;
        let mut rem = days;
        loop {
            let in_year = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
            if rem < in_year { break; }
            rem -= in_year;
            y += 1;
        }
        let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
        let mdays = [31, if leap {29} else {28}, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let mut m = 0;
        while m < 12 && rem >= mdays[m] {
            rem -= mdays[m];
            m += 1;
        }
        format!("{}{:02}{:02}", y, m + 1, rem + 1)
    };

    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    for line in &mut lines {
        let trimmed = line.trim();
        if trimmed.contains("; Serial") {
            if let Some(serial) = trimmed.split_whitespace().next().and_then(|w| w.parse::<u32>().ok()) {
                let date_base = format!("{}00", today).parse::<u32>().unwrap_or(0);
                let new_serial = if serial >= date_base {
                    serial + 1
                } else {
                    date_base
                };
                *line = line.replacen(&serial.to_string(), &new_serial.to_string(), 1);
            }
            break;
        }
    }
    lines.join("\n")
}

const CLASSES: &[&str] = &["IN", "CH", "HS", "CS"];
const TYPES: &[&str] = &[
    "A", "AAAA", "CNAME", "MX", "NS", "TXT", "SOA", "SRV", "CAA", "PTR",
    "DNSKEY", "DS", "NAPTR", "SSHFP", "TLSA", "SPF", "HINFO", "LOC", "CERT",
    "DNAME", "AFSDB", "CDNSKEY", "CDS", "DLV", "HTTPS", "NSEC", "NSEC3",
    "NSEC3PARAM", "RRSIG", "SVCB", "URI",
];

fn is_ttl(s: &str) -> bool {
    s.chars().next().map_or(false, |c| c.is_ascii_digit())
        && s.trim_end_matches(|c: char| "smhdwSMHDW".contains(c))
            .chars()
            .all(|c| c.is_ascii_digit())
}

fn is_class(s: &str) -> bool {
    CLASSES.iter().any(|c| c.eq_ignore_ascii_case(s))
}

fn is_type(s: &str) -> bool {
    TYPES.iter().any(|t| t.eq_ignore_ascii_case(s))
}

struct RecordParts {
    name: String,
    ttl: String,
    class: String,
    rtype: String,
    rdata: String,
}

fn parse_record(line: &str) -> Option<RecordParts> {
    // Split off inline comment
    let (body, comment) = match line.find(';') {
        Some(pos) => {
            // Don't split inside quoted strings
            let before = &line[..pos];
            let quotes = before.chars().filter(|&c| c == '"').count();
            if quotes % 2 == 0 {
                (&line[..pos], Some(&line[pos..]))
            } else {
                (line, None)
            }
        }
        None => (line, None),
    };

    let tokens: Vec<&str> = body.split_whitespace().collect();
    if tokens.len() < 2 { return None; }

    let starts_with_space = line.starts_with(' ') || line.starts_with('\t');
    let mut i = 0;
    let name;

    if starts_with_space {
        name = String::new();
    } else {
        name = tokens[0].to_string();
        i = 1;
    }

    let mut ttl = String::new();
    let mut class = String::new();

    // Parse optional TTL and class (in either order)
    for _ in 0..2 {
        if i >= tokens.len() { break; }
        if ttl.is_empty() && is_ttl(tokens[i]) {
            ttl = tokens[i].to_string();
            i += 1;
        } else if class.is_empty() && is_class(tokens[i]) {
            class = tokens[i].to_string();
            i += 1;
        } else {
            break;
        }
    }

    if i >= tokens.len() { return None; }
    if !is_type(tokens[i]) { return None; }

    let rtype = tokens[i].to_uppercase();
    i += 1;

    let mut rdata = tokens[i..].join("  ");
    if let Some(c) = comment {
        if !rdata.is_empty() {
            rdata.push_str("  ");
        }
        rdata.push_str(c.trim());
    }

    Some(RecordParts { name, ttl, class, rtype, rdata })
}

fn format_zone(content: &str) -> String {
    enum Parsed {
        Passthrough(String),
        Record(RecordParts),
    }

    let mut items: Vec<Parsed> = Vec::new();
    let mut in_parens = false;
    let mut max_name: usize = 0;
    let mut max_ttl: usize = 0;
    let mut max_class: usize = 2; // minimum "IN"
    let mut max_type: usize = 0;

    for line in content.lines() {
        let trimmed = line.trim();

        // Passthrough: blank, comment-only, directive
        if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('$') {
            items.push(Parsed::Passthrough(line.to_string()));
            if trimmed.contains('(') && !trimmed.contains(')') { in_parens = true; }
            continue;
        }

        // Inside parenthesized block (SOA continuation etc)
        if in_parens {
            items.push(Parsed::Passthrough(line.to_string()));
            if trimmed.contains(')') { in_parens = false; }
            continue;
        }

        // Check if this line opens parens (SOA first line)
        let opens_parens = trimmed.contains('(');
        if opens_parens {
            items.push(Parsed::Passthrough(line.to_string()));
            if !trimmed.contains(')') { in_parens = true; }
            continue;
        }

        // Try to parse as a record
        match parse_record(line) {
            Some(rec) => {
                if rec.name.len() > max_name { max_name = rec.name.len(); }
                if rec.ttl.len() > max_ttl { max_ttl = rec.ttl.len(); }
                if rec.class.len() > max_class { max_class = rec.class.len(); }
                if rec.rtype.len() > max_type { max_type = rec.rtype.len(); }
                items.push(Parsed::Record(rec));
            }
            None => {
                items.push(Parsed::Passthrough(line.to_string()));
            }
        }
    }

    // Ensure minimums
    if max_name < 1 { max_name = 1; }
    if max_ttl < 1 { max_ttl = 1; }

    let mut out = Vec::new();
    for item in &items {
        match item {
            Parsed::Passthrough(line) => out.push(line.clone()),
            Parsed::Record(rec) => {
                let name_part = if rec.name.is_empty() {
                    " ".repeat(max_name)
                } else {
                    format!("{:<width$}", rec.name, width = max_name)
                };
                let formatted = format!(
                    "{}  {:<ttl_w$}  {:<cls_w$}  {:<typ_w$}  {}",
                    name_part,
                    rec.ttl,
                    rec.class,
                    rec.rtype,
                    rec.rdata,
                    ttl_w = max_ttl,
                    cls_w = max_class,
                    typ_w = max_type,
                );
                out.push(formatted.trim_end().to_string());
            }
        }
    }

    out.join("\n")
}

fn axfr_to_bind(raw: &str, zone: &str) -> Result<String, String> {
    let origin = format!("{}.", zone);
    let suffix = format!(".{}.", zone);
    let mut out = Vec::new();

    out.push(format!("$ORIGIN {}", origin));

    let records: Vec<&str> = raw.lines()
        .filter(|l| !l.is_empty() && !l.starts_with(';'))
        .collect();

    if records.is_empty() {
        return Err("Empty AXFR response".to_string());
    }

    // Strip trailing SOA (AXFR ends with duplicate SOA)
    let records = if records.len() > 1 {
        &records[..records.len() - 1]
    } else {
        &records[..]
    };

    for line in records {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 { continue; }

        let name = if parts[0] == origin {
            "@".to_string()
        } else if parts[0].ends_with(&suffix) {
            parts[0][..parts[0].len() - suffix.len()].to_string()
        } else {
            parts[0].to_string()
        };

        let ttl = parts[1];
        let class = parts[2];
        let rtype = parts[3].to_uppercase();

        if rtype == "SOA" && parts.len() >= 11 {
            out.push(format!("{}  {}  {}  SOA  {} {} (", name, ttl, class, parts[4], parts[5]));
            out.push(format!("    {}    ; Serial", parts[6]));
            out.push(format!("    {}    ; Refresh", parts[7]));
            out.push(format!("    {}    ; Retry", parts[8]));
            out.push(format!("    {}    ; Expire", parts[9]));
            out.push(format!("    {}    ; Minimum TTL", parts[10]));
            out.push(")".to_string());
        } else {
            let rdata = parts[4..].join(" ");
            out.push(format!("{}  {}  {}  {}  {}", name, ttl, class, rtype, rdata));
        }
    }

    Ok(out.join("\n") + "\n")
}

fn axfr_from(host: &str, dig_port: &str, name: &str) -> Result<String, String> {
    let output = std::process::Command::new("dig")
        .arg(format!("@{}", host))
        .arg("-p").arg(dig_port)
        .arg("axfr").arg(name)
        .arg("+nocomments").arg("+nostats").arg("+nocmd")
        .output()
        .map_err(|e| format!("dig failed: {}", e))?;

    if !output.status.success() {
        return Err("AXFR failed".to_string());
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    axfr_to_bind(&raw, name)
}

#[tauri::command]
pub async fn pull_zone(identity: String, name: String, port: u16) -> Result<String, String> {
    let name = name.to_lowercase();
    let cfg = config::read_config(&identity);

    let (host, dig_port) = if cfg.accept_transfers && !cfg.transfer_from.is_empty() {
        let source = &cfg.transfer_from;
        if let Some((h, p)) = source.rsplit_once(':') {
            (h.to_string(), p.to_string())
        } else {
            (source.to_string(), "53".to_string())
        }
    } else {
        ("127.0.0.1".to_string(), port.to_string())
    };

    let content = axfr_from(&host, &dig_port, &name)?;
    let path = paths::ensure_zones_dir(&identity).join(format!("{}.zone", name));
    fs::write(&path, &content).map_err(|e| e.to_string())?;

    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axfr_to_bind_relativises_names() {
        let axfr = "example.com.\t3600\tIN\tSOA\tns1.example.com. admin.example.com. 1 3600 900 604800 86400\n\
                     example.com.\t3600\tIN\tNS\tns1.example.com.\n\
                     ns1.example.com.\t3600\tIN\tA\t127.0.0.1\n\
                     www.example.com.\t3600\tIN\tA\t1.2.3.4\n\
                     example.com.\t3600\tIN\tSOA\tns1.example.com. admin.example.com. 1 3600 900 604800 86400\n";
        let result = axfr_to_bind(axfr, "example.com").unwrap();
        assert!(result.starts_with("$ORIGIN example.com.\n"));
        assert!(result.contains("@  3600  IN  SOA"));
        assert!(result.contains("@  3600  IN  NS  ns1.example.com."));
        assert!(result.contains("ns1  3600  IN  A  127.0.0.1"));
        assert!(result.contains("www  3600  IN  A  1.2.3.4"));
    }

    #[test]
    fn axfr_to_bind_formats_soa_with_parens() {
        let axfr = "test.com.\t3600\tIN\tSOA\tns1.test.com. admin.test.com. 2024 3600 900 604800 86400\n\
                     test.com.\t3600\tIN\tSOA\tns1.test.com. admin.test.com. 2024 3600 900 604800 86400\n";
        let result = axfr_to_bind(axfr, "test.com").unwrap();
        assert!(result.contains("SOA  ns1.test.com. admin.test.com. ("));
        assert!(result.contains("2024    ; Serial"));
        assert!(result.contains("86400    ; Minimum TTL"));
        assert!(result.contains(")"));
    }

    #[test]
    fn axfr_to_bind_strips_trailing_soa() {
        let axfr = "t.com.\t3600\tIN\tSOA\tns.t.com. a.t.com. 1 3600 900 604800 86400\n\
                     t.com.\t3600\tIN\tA\t1.2.3.4\n\
                     t.com.\t3600\tIN\tSOA\tns.t.com. a.t.com. 1 3600 900 604800 86400\n";
        let result = axfr_to_bind(axfr, "t.com").unwrap();
        // Only one SOA should appear
        assert_eq!(result.matches("SOA").count(), 1);
    }

    #[test]
    fn axfr_to_bind_empty_response() {
        assert!(axfr_to_bind("", "test.com").is_err());
    }

    #[test]
    fn bump_serial_increments_when_already_current() {
        // Use a serial that's already above any realistic today-based serial
        let content = "    2099010105    ; Serial\n    3600    ; Refresh";
        let result = bump_serial(content);
        assert!(result.contains("2099010106    ; Serial"), "got: {}", result);
    }

    #[test]
    fn bump_serial_rolls_to_today() {
        let content = "    2020010100    ; Serial\n    3600    ; Refresh";
        let result = bump_serial(content);
        assert!(!result.contains("2020010100"), "got: {}", result);
        assert!(result.contains("; Serial"), "got: {}", result);
    }

    #[test]
    fn bump_serial_no_serial_comment() {
        let content = "just some text\nno serial here";
        let result = bump_serial(content);
        assert_eq!(result, content);
    }

    #[test]
    fn format_zone_aligns_columns() {
        let input = "www 3600 IN A 1.2.3.4\nmail 3600 IN MX 10 mail.example.com.\nns1 3600 IN A 5.6.7.8";
        let result = format_zone(input);
        assert_eq!(result, "www   3600  IN  A   1.2.3.4\nmail  3600  IN  MX  10  mail.example.com.\nns1   3600  IN  A   5.6.7.8");
    }

    #[test]
    fn format_zone_preserves_blank_lines() {
        let input = "www 3600 IN A 1.2.3.4\n\nns1 3600 IN A 5.6.7.8";
        let result = format_zone(input);
        assert!(result.contains("\n\n"), "got: {}", result);
    }

    #[test]
    fn format_zone_preserves_comments() {
        let input = "; this is a comment\nwww 3600 IN A 1.2.3.4";
        let result = format_zone(input);
        assert!(result.starts_with("; this is a comment\n"), "got: {}", result);
    }

    #[test]
    fn format_zone_preserves_directives() {
        let input = "$TTL 3600\n$ORIGIN example.com.\nwww 3600 IN A 1.2.3.4";
        let result = format_zone(input);
        assert!(result.starts_with("$TTL 3600\n$ORIGIN example.com.\n"), "got: {}", result);
    }

    #[test]
    fn format_zone_preserves_soa_block() {
        let input = "@  3600  IN  SOA  ns1.x.com. admin.x.com. (\n    1       ; Serial\n    3600    ; Refresh\n)\n@  3600  IN  NS  ns1.x.com.";
        let result = format_zone(input);
        assert!(result.contains("SOA  ns1.x.com. admin.x.com. ("), "got: {}", result);
        assert!(result.contains("    1       ; Serial"), "got: {}", result);
        assert!(result.contains(")"), "got: {}", result);
    }

    #[test]
    fn format_zone_preserves_inline_comments() {
        let input = "www 3600 IN A 1.2.3.4 ; web server";
        let result = format_zone(input);
        assert!(result.contains("; web server"), "got: {}", result);
    }

    #[test]
    fn format_zone_handles_no_records() {
        let input = "; just comments\n$TTL 3600";
        let result = format_zone(input);
        assert_eq!(result, input);
    }
}
