use colored::*;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

#[derive(Clone, Debug)]
pub struct AuditEvent {
    pub timestamp: String,
    pub category: String,
    pub action: String,
    pub status: String,
    pub details: String,
}

pub fn get_audit_file_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("SUDO_USER").map(|u| format!("/home/{}", u)))
        .unwrap_or_else(|_| "/home/mzia".to_string());
    PathBuf::from(home).join(".config/pulsarkey/audit.log")
}

/// Logs an audit event to the persistent audit log file.
pub fn log_event(category: &str, action: &str, status: &str, details: &str) {
    let now = get_current_timestamp();
    let log_path = get_audit_file_path();
    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let line = format!("{}|{}|{}|{}|{}\n", now, category, action, status, details);

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
        let _ = file.write_all(line.as_bytes());
    }
}

fn get_current_timestamp() -> String {
    let output = Command::new("date").arg("+%Y-%m-%d %H:%M:%S").output();
    if let Ok(out) = output {
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    } else {
        "Just now".to_string()
    }
}

/// Reads recorded events from the persistent audit log.
pub fn get_recorded_events(limit: usize) -> Vec<AuditEvent> {
    let log_path = get_audit_file_path();
    let mut events = Vec::new();

    if let Ok(content) = fs::read_to_string(&log_path) {
        for line in content.lines().rev() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 5 {
                events.push(AuditEvent {
                    timestamp: parts[0].to_string(),
                    category: parts[1].to_string(),
                    action: parts[2].to_string(),
                    status: parts[3].to_string(),
                    details: parts[4].to_string(),
                });
            }
            if events.len() >= limit {
                break;
            }
        }
    }

    events
}

/// Fetches system authentication events from journalctl.
pub fn get_system_auth_events(limit: usize) -> Vec<AuditEvent> {
    let mut events = Vec::new();
    let output = Command::new("journalctl")
        .args([
            "-g",
            "sudo:session|polkitd.*successfully authenticated|cosmic-greeter",
            "-n",
            &limit.to_string(),
            "--no-pager",
        ])
        .output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines().rev() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }

            // Extract date: first 15 chars (e.g. "Sep 15 10:14:49")
            let ts = if l.len() >= 15 { &l[..15] } else { "Recent" };

            if l.contains("sudo:session): session opened") {
                let user = if let Some(idx) = l.find("for user") {
                    l[idx..].trim()
                } else {
                    "sudo elevation"
                };
                events.push(AuditEvent {
                    timestamp: ts.to_string(),
                    category: "AUTH".to_string(),
                    action: "sudo Privilege Escalation".to_string(),
                    status: "Success".to_string(),
                    details: user.to_string(),
                });
            } else if l.contains("polkitd") && l.contains("successfully authenticated") {
                let action = if let Some(idx) = l.find("for action") {
                    let rest = &l[idx + 10..];
                    rest.split_whitespace().next().unwrap_or("GUI Auth")
                } else {
                    "polkit action"
                };
                events.push(AuditEvent {
                    timestamp: ts.to_string(),
                    category: "AUTH".to_string(),
                    action: "Polkit GUI Elevation".to_string(),
                    status: "Success".to_string(),
                    details: action.to_string(),
                });
            } else if l.contains("cosmic-greeter") {
                events.push(AuditEvent {
                    timestamp: ts.to_string(),
                    category: "AUTH".to_string(),
                    action: "COSMIC Greeter Unlock".to_string(),
                    status: "Verified".to_string(),
                    details: "Lockscreen authentication".to_string(),
                });
            }

            if events.len() >= limit {
                break;
            }
        }
    }

    events
}

/// Merges PulsarKey audit logs with system journal events.
pub fn get_combined_recent_events(limit: usize) -> Vec<AuditEvent> {
    let mut all = get_recorded_events(limit);
    let sys = get_system_auth_events(limit);

    all.extend(sys);
    all.truncate(limit);
    all
}

/// Clears the audit log file.
pub fn clear_audit_log() -> Result<(), std::io::Error> {
    let log_path = get_audit_file_path();
    if log_path.exists() {
        fs::remove_file(log_path)?;
    }
    Ok(())
}

/// Prints a formatted audit table to the terminal.
pub fn print_audit_log(clear: bool) {
    if clear {
        match clear_audit_log() {
            Ok(_) => println!("{} Audit log cleared successfully.", "✅".green()),
            Err(e) => eprintln!("{} Failed to clear audit log: {}", "❌ Error:".red(), e),
        }
        return;
    }

    println!("{}", "================================================================================".cyan());
    println!("{}", "📜 PulsarKey Authentication Audit Journal (Recent Pulses)".bold().cyan());
    println!("{}", "================================================================================".cyan());

    let events = get_combined_recent_events(25);

    if events.is_empty() {
        println!("  {}", "(No recent authentication or hardware events recorded)".dimmed());
    } else {
        println!("{:<19} {:<10} {:<28} {:<12} {}", "TIMESTAMP".bold(), "CATEGORY".bold(), "ACTION".bold(), "STATUS".bold(), "DETAILS".bold());
        println!("{}", "─".repeat(80).dimmed());

        for ev in events {
            let cat_colored = match ev.category.as_str() {
                "AUTH" => ev.category.green(),
                "HARDWARE" => ev.category.cyan(),
                "SENTINEL" => ev.category.yellow(),
                "SECURITY" => ev.category.magenta(),
                _ => ev.category.white(),
            };

            let status_colored = match ev.status.as_str() {
                "Success" | "Verified" | "Connected" => ev.status.green(),
                "Locked" | "Warning" => ev.status.yellow(),
                "Failed" | "Denied" => ev.status.red(),
                _ => ev.status.normal(),
            };

            println!(
                "{:<19} {:<10} {:<28} {:<12} {}",
                ev.timestamp.dimmed(),
                cat_colored,
                ev.action.bold(),
                status_colored,
                ev.details.dimmed()
            );
        }
    }

    println!("{}", "================================================================================".cyan());
    println!("💡 Tip: Clear this log at any time with: pulsarkey audit --clear\n");
}

/// Formats the most recent events as brief strings for the COSMIC tray menu.
pub fn get_recent_summary_for_applet(limit: usize) -> Vec<String> {
    let events = get_combined_recent_events(limit);
    if events.is_empty() {
        return vec!["No recent activity".to_string()];
    }

    events
        .into_iter()
        .map(|e| {
            let time_part = if e.timestamp.len() >= 19 {
                // "2026-09-15 10:45:12" -> "10:45"
                &e.timestamp[11..16]
            } else if e.timestamp.len() >= 15 {
                // "Sep 15 10:45:12" -> "10:45"
                &e.timestamp[7..12]
            } else {
                &e.timestamp
            };
            format!("• {} {}: {} ({})", time_part, e.category, e.action, e.status)
        })
        .collect()
}
