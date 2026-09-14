use colored::*;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn run_ssh_setup(no_resident: bool, no_git_sign: bool, custom_path: Option<String>) {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/mzia".to_string());
    let ssh_dir = PathBuf::from(&home).join(".ssh");
    let default_key_path = ssh_dir.join("id_ed25519_sk");
    let key_path = custom_path.map(PathBuf::from).unwrap_or(default_key_path);
    let pub_path = PathBuf::from(format!("{}.pub", key_path.to_string_lossy()));

    println!("{}", "==================================================".cyan());
    println!("{}", "🔑 PulsarKey Hardware SSH & Git Signing Wizard".bold().cyan());
    println!("{}", "==================================================".cyan());

    // 1. Ensure ~/.ssh exists with 0700 permissions
    if !ssh_dir.exists() {
        let _ = fs::create_dir_all(&ssh_dir);
        let _ = Command::new("chmod").args(["700", ssh_dir.to_str().unwrap()]).status();
    }

    // 2. Hardware Detection
    print!("🔍 Detecting security key... ");
    io::stdout().flush().unwrap();
    let yk_info = Command::new("ykman").arg("info").output();
    let has_key = match yk_info {
        Ok(out) if out.status.success() => {
            let info = String::from_utf8_lossy(&out.stdout);
            let name = info
                .lines()
                .find(|l| l.starts_with("Device type:"))
                .map(|l| l.replace("Device type:", "").trim().to_string())
                .unwrap_or_else(|| "YubiKey Detected".to_string());
            println!("{}", name.bold().green());
            true
        }
        _ => {
            println!("{}", "YubiKey / FIDO2 Key (Generic)".yellow());
            false
        }
    };

    if !has_key {
        println!("👉 Ensure your FIDO2 security key is plugged in before proceeding.");
    }

    // 3. Key Generation or Reuse
    let mut generate_new = true;
    if key_path.exists() {
        println!("\n{} Found existing SSH key at: {}", "⚠️ Warning:".yellow(), key_path.display());
        print!("❓ (u)se existing key for Git signing, (o)verwrite, or (c)ancel? [U/o/c]: ");
        io::stdout().flush().unwrap();
        let mut choice = String::new();
        io::stdin().read_line(&mut choice).unwrap();
        let c = choice.trim().to_lowercase();
        if c == "c" {
            println!("Aborted.");
            return;
        } else if c == "o" {
            generate_new = true;
        } else {
            generate_new = false;
        }
    }

    if generate_new {
        println!("\n{}", "🔐 Step 1: Generating Hardware-Backed FIDO2 SSH Key".bold());
        println!("👉 OpenSSH will prompt for your FIDO2 PIN (if set) and your biometric scan.");
        println!(
            "👉 When the YubiKey flashes, {} on the sensor.\n",
            "scan your fingerprint or touch".bold().yellow()
        );

        let comment = format!(
            "pulsarkey-fido2-{}",
            chrono_date_str()
        );

        let success = generate_key(&key_path, &comment, !no_resident);
        if !success {
            eprintln!("\n{} SSH key generation failed or was cancelled.", "❌ Error:".red());
            return;
        }

        println!("\n{} Hardware key successfully created at: {}", "✅".green(), key_path.display());
    }

    // 4. Verify Public Key
    if !pub_path.exists() {
        eprintln!("{} Public key not found at {}", "❌ Error:".red(), pub_path.display());
        return;
    }

    let pub_key_content = match fs::read_to_string(&pub_path) {
        Ok(c) => c.trim().to_string(),
        Err(e) => {
            eprintln!("{} Could not read public key: {}", "❌ Error:".red(), e);
            return;
        }
    };

    // 5. Configure Git Commit Signing
    if !no_git_sign {
        println!("\n{}", "🔏 Step 2: Git Commit Signing Configuration".bold());
        print!("❓ Configure Git to automatically sign all commits with this key? [Y/n]: ");
        io::stdout().flush().unwrap();
        let mut resp = String::new();
        io::stdin().read_line(&mut resp).unwrap();

        if !resp.trim().eq_ignore_ascii_case("n") {
            configure_git_signing(&home, &pub_path, &pub_key_content);
        }
    }

    // 6. Display & Clipboard
    println!("\n{}", "==================================================".green());
    println!("{}", "🎉 Hardware SSH & Git Signing Ready!".bold().green());
    println!("{}", "==================================================".green());
    println!("Public Key (add to GitHub / GitLab):");
    println!("{}", pub_key_content.bold().cyan());
    println!("{}", "==================================================".green());

    // Try copying to clipboard
    let copied = copy_to_clipboard(&pub_key_content);
    if copied {
        println!("{} Public key copied to your system clipboard!", "📋".green());
    }

    println!("\nNext Steps for GitHub:");
    println!("  1. Open: {}", "https://github.com/settings/keys".bold().cyan());
    println!("  2. Click {} ➔ Title: {}", "New SSH Key".bold(), "PulsarKey YubiKey Bio".yellow());
    println!("  3. Key Type: Choose {} (or Signing Key)", "Authentication Key".bold());
    println!("  4. Paste your key and click {}", "Add SSH Key".bold());
    println!("\nTo test SSH authentication:");
    println!("  {}", "ssh -T git@github.com".bold().cyan());
    println!("  (Your YubiKey sensor will flash for fingerprint touch verification!)\n");
}

fn generate_key(key_path: &Path, comment: &str, use_resident: bool) -> bool {
    let mut args: Vec<String> = vec![
        "-t".into(),
        "ed25519-sk".into(),
        "-O".into(),
        "verify-required".into(),
    ];

    if use_resident {
        args.push("-O".into());
        args.push("resident".into());
    }

    args.push("-C".into());
    args.push(comment.into());
    args.push("-f".into());
    args.push(key_path.to_string_lossy().to_string());

    let status = Command::new("ssh-keygen")
        .args(&args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    match status {
        Ok(s) if s.success() => true,
        _ if use_resident => {
            println!("\n⚠️ Resident key generation unsupported or timed out; retrying standard FIDO2 key...");
            generate_key(key_path, comment, false)
        }
        _ => false,
    }
}

fn configure_git_signing(home: &str, pub_path: &Path, pub_key_content: &str) {
    // 1. Get user email
    let email_output = Command::new("git").args(["config", "--global", "user.email"]).output();
    let email = match email_output {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }
        _ => String::new(),
    };

    let user_email = if !email.is_empty() {
        email
    } else {
        print!("Enter your Git email address: ");
        io::stdout().flush().unwrap();
        let mut inp = String::new();
        io::stdin().read_line(&mut inp).unwrap();
        let trimmed = inp.trim().to_string();
        let _ = Command::new("git").args(["config", "--global", "user.email", &trimmed]).status();
        trimmed
    };

    // 2. Set Git configs
    let _ = Command::new("git").args(["config", "--global", "gpg.format", "ssh"]).status();
    let _ = Command::new("git")
        .args(["config", "--global", "user.signingkey", pub_path.to_str().unwrap()])
        .status();
    let _ = Command::new("git").args(["config", "--global", "commit.gpgsign", "true"]).status();
    let _ = Command::new("git").args(["config", "--global", "tag.gpgsign", "true"]).status();

    // 3. Configure allowed_signers for local signature verification
    let allowed_signers_path = PathBuf::from(home).join(".ssh/allowed_signers");
    let signer_entry = format!("{} {}\n", user_email, pub_key_content);

    let mut exists = false;
    if allowed_signers_path.exists() {
        if let Ok(file) = File::open(&allowed_signers_path) {
            for line in BufReader::new(file).lines().flatten() {
                if line.contains(pub_key_content) {
                    exists = true;
                    break;
                }
            }
        }
    }

    if !exists {
        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&allowed_signers_path)
        {
            let _ = f.write_all(signer_entry.as_bytes());
        }
    }

    let _ = Command::new("git")
        .args([
            "config",
            "--global",
            "gpg.ssh.allowedSignersFile",
            allowed_signers_path.to_str().unwrap(),
        ])
        .status();

    println!("{} Configured Git to sign all commits via SSH FIDO2!", "✅".green());
    println!("{} Added signing identity to: {}", "✅".green(), allowed_signers_path.display());
}

fn copy_to_clipboard(text: &str) -> bool {
    // Try xclip
    if let Ok(mut child) = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(Stdio::piped())
        .spawn()
    {
        if let Some(ref mut stdin) = child.stdin {
            let _ = stdin.write_all(text.as_bytes());
        }
        if let Ok(s) = child.wait() {
            if s.success() {
                return true;
            }
        }
    }

    // Try wl-copy
    if let Ok(mut child) = Command::new("wl-copy").stdin(Stdio::piped()).spawn() {
        if let Some(ref mut stdin) = child.stdin {
            let _ = stdin.write_all(text.as_bytes());
        }
        if let Ok(s) = child.wait() {
            if s.success() {
                return true;
            }
        }
    }

    false
}

fn chrono_date_str() -> String {
    let output = Command::new("date").arg("+%Y%m%d").output();
    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        _ => "2026".to_string(),
    }
}

/// Checks current status of hardware SSH keys and git commit signing
pub fn check_ssh_git_status() -> (String, String) {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/mzia".to_string());
    let default_key = PathBuf::from(&home).join(".ssh/id_ed25519_sk.pub");

    let ssh_status = if default_key.exists() {
        if let Ok(content) = fs::read_to_string(&default_key) {
            if content.contains("sk-ssh-ed25519") {
                "FIDO2 Active (~/.ssh/id_ed25519_sk)".to_string()
            } else if content.contains("sk-ecdsa") {
                "FIDO2 Active (~/.ssh/id_ecdsa_sk)".to_string()
            } else {
                "Standard SSH key".to_string()
            }
        } else {
            "Key unreadable".to_string()
        }
    } else {
        "Not configured (run: pulsarkey ssh-setup)".to_string()
    };

    let git_signing = Command::new("git").args(["config", "--global", "commit.gpgsign"]).output();
    let is_signing_enabled = match git_signing {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim() == "true"
        }
        _ => false,
    };

    let git_format = Command::new("git").args(["config", "--global", "gpg.format"]).output();
    let is_ssh_format = match git_format {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim() == "ssh"
        }
        _ => false,
    };

    let git_status = if is_signing_enabled && is_ssh_format {
        "Enabled (SSH FIDO2 touch required)".to_string()
    } else if is_signing_enabled {
        "Enabled (GPG format)".to_string()
    } else {
        "Disabled".to_string()
    };

    (ssh_status, git_status)
}
