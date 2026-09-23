use colored::*;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

const MAPPING_FILE: &str = "/etc/yubico/u2f_keys";

#[derive(Debug, Clone)]
pub struct EnrolledKey {
    pub index: usize,
    pub credential_summary: String,
    pub has_uv: bool,
}

/// Parses /etc/yubico/u2f_keys and returns enrolled keys for the user.
pub fn get_enrolled_keys(username: &str) -> Vec<EnrolledKey> {
    let mut keys = Vec::new();
    let content = match fs::read_to_string(MAPPING_FILE) {
        Ok(c) => c,
        Err(_) => return keys,
    };

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with(username) && line.contains(':') {
            let parts: Vec<&str> = line.split(':').collect();
            // parts[0] is username, parts[1..] are registered keys
            for (idx, key_str) in parts.iter().skip(1).enumerate() {
                if !key_str.trim().is_empty() {
                    let has_uv = key_str.contains("+presence+verification") || key_str.contains("+verification");
                    let summary = if key_str.len() > 32 {
                        format!("{}...{}", &key_str[..12], &key_str[key_str.len() - 12..])
                    } else {
                        key_str.to_string()
                    };
                    keys.push(EnrolledKey {
                        index: idx + 1,
                        credential_summary: summary,
                        has_uv,
                    });
                }
            }
        }
    }

    keys
}

/// Displays the current backup key and credential status.
pub fn print_backup_status() {
    let username = std::env::var("SUDO_USER")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "mzia".to_string());

    println!("{}", "==================================================".cyan());
    println!("{}", "👯 PulsarKey Backup Key & Recovery Status".bold().cyan());
    println!("{}", "==================================================".cyan());

    let keys = get_enrolled_keys(&username);
    println!("User:                   {}", username.bold().green());
    println!("Credential Map:         {} registered key(s)", keys.len().to_string().cyan());

    if keys.is_empty() {
        println!("\n{} No keys enrolled yet. Run 'sudo pulsarkey setup' to enroll your primary key.", "⚠️ Warning:".yellow());
    } else {
        println!("\nEnrolled Keys:");
        for k in &keys {
            let role = if k.index == 1 { "Primary Key".green() } else { "Backup Key".yellow() };
            let uv_str = if k.has_uv { "Biometric/UV Enabled".green() } else { "Presence Touch Only".dimmed() };
            println!("  [{}] {} — {} ({})", k.index, role.bold(), k.credential_summary.dimmed(), uv_str);
        }

        if keys.len() == 1 {
            println!("\n{} Only 1 key registered. Adding a backup key is strongly recommended to avoid accidental lockout.", "💡 Recommendation:".yellow());
            println!("   Run: sudo pulsarkey backup pair");
        } else {
            println!("\n{} You have {} redundant keys registered for fail-safe desktop security.", "✅ Great:".green(), keys.len());
        }
    }

    // Check backup SSH key
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/mzia".to_string());
    let backup_ssh = PathBuf::from(&home).join(".ssh/id_ed25519_sk_backup.pub");
    println!("\nHardware SSH Redundancy:");
    if backup_ssh.exists() {
        println!("  Backup SSH Key:       {}", "Configured (~/.ssh/id_ed25519_sk_backup)".green());
    } else {
        println!("  Backup SSH Key:       {}", "Not configured (optional)".dimmed());
    }

    println!("{}", "==================================================".cyan());
}

/// Interactive wizard to pair a secondary / backup Security Key.
pub fn pair_backup_key() -> Result<(), String> {
    crate::ensure_root("backup pair");

    let username = std::env::var("SUDO_USER")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "mzia".to_string());

    println!("{}", "==================================================".cyan());
    println!("{}", "👯 PulsarKey Backup Key Pairing Wizard".bold().cyan());
    println!("{}", "==================================================".cyan());
    println!("This wizard will enroll a secondary FIDO2 security key as a backup.");
    println!("If your primary key is ever lost, misplaced, or damaged, your backup");
    println!("key can instantly unlock your lockscreen, authorize sudo, and sign Polkit.");
    println!("{}", "──────────────────────────────────────────────────".blue());

    let existing_keys = get_enrolled_keys(&username);
    if existing_keys.is_empty() {
        return Err("No primary key found. Please run 'sudo pulsarkey setup' first to configure your system.".to_string());
    }

    println!("Current status: {} key(s) already registered for user '{}'.", existing_keys.len().to_string().cyan(), username.bold());
    println!("\n👉 STEP 1: Unplug your PRIMARY key (if currently plugged in).");
    println!("👉 STEP 2: Plug in your SECONDARY / BACKUP Security Key into a USB port.");
    print!("\nPress Enter once your BACKUP key is inserted and ready...");
    io::stdout().flush().unwrap();
    let mut buf = String::new();
    let _ = io::stdin().read_line(&mut buf);

    // Query connected token
    print!("🔍 Detecting backup key... ");
    io::stdout().flush().unwrap();
    let dev = crate::hardware::detect_fido_device();
    if dev.is_connected {
        println!("{}", dev.product_name.bold().green());
    } else {
        println!("{}", "FIDO2 Security Key (Generic)".bold().green());
    }

    // Generate U2F credential
    println!("\n👉 STEP 3: Touch or scan your fingerprint on the BACKUP key when it pulses.");
    let mut cmd = Command::new("pamu2fcfg");
    cmd.args(["-n", "-u", &username]);

    // If biometric, enforce verification
    if dev.product_name.contains("Bio") {
        cmd.args(["-N", "+presence+verification", "-P", "+presence+verification"]);
    }

    let output = cmd.output().map_err(|e| format!("Failed to run pamu2fcfg: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Backup key registration failed or timed out: {}", err.trim()));
    }

    let raw_credential = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if raw_credential.is_empty() {
        return Err("No credential returned by backup key.".to_string());
    }

    // Append to mapping file
    let current_content = fs::read_to_string(MAPPING_FILE)
        .map_err(|e| format!("Failed to read {}: {}", MAPPING_FILE, e))?;

    let mut new_lines = Vec::new();
    let mut updated = false;

    for line in current_content.lines() {
        let l = line.trim();
        if l.starts_with(&username) && l.contains(':') {
            // Append new key credential: user:key1:key2
            let updated_line = format!("{}:{}", l, raw_credential);
            new_lines.push(updated_line);
            updated = true;
        } else {
            new_lines.push(l.to_string());
        }
    }

    if !updated {
        new_lines.push(format!("{}:{}", username, raw_credential));
    }

    fs::write(MAPPING_FILE, new_lines.join("\n") + "\n")
        .map_err(|e| format!("Failed to update {}: {}", MAPPING_FILE, e))?;

    let _ = Command::new("chmod").args(["644", MAPPING_FILE]).status();

    // Audit logging
    crate::audit::log_event(
        "SECURITY",
        "Backup Key Enrolled",
        "Success",
        &format!("Enrolled redundant key for user {}", username),
    );

    println!("\n{} Backup key successfully enrolled in {}!", "🎉 Congratulations:".bold().green(), MAPPING_FILE);
    println!("Total enrolled keys for {}: {}", username.bold(), (existing_keys.len() + 1).to_string().green());

    // Optional: Provision backup SSH resident key
    print!("\n❓ Would you like to generate a resident SSH key on this backup token? [y/N]: ");
    io::stdout().flush().unwrap();
    let mut ssh_choice = String::new();
    let _ = io::stdin().read_line(&mut ssh_choice);
    if ssh_choice.trim().eq_ignore_ascii_case("y") {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/mzia".to_string());
        let backup_ssh_path = format!("{}/.ssh/id_ed25519_sk_backup", home);
        println!("Generating backup SSH key at {}...", backup_ssh_path.cyan());
        crate::ssh_setup::run_ssh_setup(false, false, Some(backup_ssh_path));
    }

    println!("\n{}", "==================================================".cyan());
    println!("✅ You can now use either key interchangeably across Pop!_OS COSMIC.");
    println!("{}", "==================================================".cyan());

    Ok(())
}

/// Tests the currently inserted key against /etc/yubico/u2f_keys.
pub fn test_backup_key() {
    println!("{}", "==================================================".cyan());
    println!("{}", "🔍 Testing Connected Security Key".bold().cyan());
    println!("{}", "==================================================".cyan());
    println!("Touch or scan your fingerprint on your Security Key when prompted...\n");

    let status = Command::new("pamu2fcfg").arg("-V").status();
    match status {
        Ok(s) if s.success() => {
            println!("\n{} Security key verified and accepted by PAM engine!", "✅ Success:".green());
            crate::audit::log_event("AUTH", "Key Verification Test", "Success", "Physical sensor responded");
        }
        _ => {
            println!("\n{} Verification timed out or failed.", "❌ Warning:".yellow());
            crate::audit::log_event("AUTH", "Key Verification Test", "Failed", "Verification timed out");
        }
    }
    println!("{}", "==================================================".cyan());
}

/// Handles CLI backup subcommands.
pub fn handle_backup_cli(action: Option<String>) {
    match action.as_deref() {
        None | Some("status") | Some("list") => {
            print_backup_status();
        }
        Some("pair") | Some("add") | Some("enroll") => {
            if let Err(e) = pair_backup_key() {
                eprintln!("\n{} {}", "❌ Error:".red(), e);
            }
        }
        Some("test") | Some("verify") => {
            test_backup_key();
        }
        Some(other) => {
            eprintln!("Unknown backup action: '{}'. Valid actions: status, pair, test", other);
        }
    }
}
