use colored::*;
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const RECOVERY_DIR: &str = "/etc/pulsarkey";
const RECOVERY_AUTH_FILE: &str = "/etc/pulsarkey/recovery_codes.auth";
const CODE_CHARSET: &[u8] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

pub struct RecoveryStatus {
    pub total: usize,
    pub unused: usize,
    pub used: usize,
    pub is_configured: bool,
    pub file_path: PathBuf,
}

fn get_storage_path() -> PathBuf {
    if Path::new(RECOVERY_DIR).exists() || unsafe { libc::geteuid() } == 0 {
        PathBuf::from(RECOVERY_AUTH_FILE)
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".config/pulsarkey/recovery_codes.auth")
    }
}

fn generate_random_chunk(len: usize) -> String {
    let mut bytes = vec![0u8; len];
    if let Ok(mut f) = fs::File::open("/dev/urandom") {
        use std::io::Read;
        let _ = f.read_exact(&mut bytes);
    } else {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = ((now >> (i * 8)) & 0xFF) as u8;
        }
    }

    bytes
        .iter()
        .map(|b| {
            let idx = (*b as usize) % CODE_CHARSET.len();
            CODE_CHARSET[idx] as char
        })
        .collect()
}

pub fn generate_recovery_code() -> String {
    format!(
        "{}-{}-{}-{}",
        generate_random_chunk(4),
        generate_random_chunk(4),
        generate_random_chunk(4),
        generate_random_chunk(4)
    )
}

pub fn hash_code(code: &str) -> String {
    let clean = code.trim().replace('-', "").to_uppercase();
    let mut hasher = Sha256::new();
    hasher.update(clean.as_bytes());
    let result = hasher.finalize();
    hex_encode(&result)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn check_recovery_status() -> RecoveryStatus {
    let path = get_storage_path();
    if !path.exists() {
        return RecoveryStatus {
            total: 0,
            unused: 0,
            used: 0,
            is_configured: false,
            file_path: path,
        };
    }

    if let Ok(content) = fs::read_to_string(&path) {
        let mut total = 0;
        let mut unused = 0;
        let mut used = 0;
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 {
                total += 1;
                if parts[1].eq_ignore_ascii_case("UNUSED") {
                    unused += 1;
                } else {
                    used += 1;
                }
            }
        }
        RecoveryStatus {
            total,
            unused,
            used,
            is_configured: total > 0,
            file_path: path,
        }
    } else {
        RecoveryStatus {
            total: 0,
            unused: 0,
            used: 0,
            is_configured: false,
            file_path: path,
        }
    }
}

pub fn generate_new_recovery_set() -> Result<(Vec<String>, PathBuf, PathBuf), String> {
    let storage_path = get_storage_path();
    if let Some(parent) = storage_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let mut raw_codes = Vec::new();
    let mut file_content = String::new();
    file_content.push_str("# PulsarKey Emergency Recovery Auth File\n");
    file_content.push_str("# Format: <sha256_hash>:<status>:<created_timestamp>\n");

    let now_ts = chrono_now();

    for _ in 0..8 {
        let code = generate_recovery_code();
        let hash = hash_code(&code);
        file_content.push_str(&format!("{}:UNUSED:{}\n", hash, now_ts));
        raw_codes.push(code);
    }

    if let Err(e) = fs::write(&storage_path, &file_content) {
        return Err(format!("Failed to write recovery codes to {}: {}", storage_path.display(), e));
    }

    let _ = fs::set_permissions(&storage_path, fs::Permissions::from_mode(0o600));

    // Also write emergency paper recovery kit to user Desktop or home
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let desktop = PathBuf::from(&home).join("Desktop");
    let export_path = if desktop.exists() {
        desktop.join("PulsarKey-Emergency-Recovery-Kit.txt")
    } else {
        PathBuf::from(&home).join("PulsarKey-Emergency-Recovery-Kit.txt")
    };

    let kit_document = format_emergency_kit_document(&raw_codes);
    let _ = fs::write(&export_path, kit_document);

    Ok((raw_codes, storage_path, export_path))
}

pub fn verify_and_consume_code(input_code: &str) -> (bool, String) {
    let storage_path = get_storage_path();
    if !storage_path.exists() {
        return (false, "No emergency recovery codes configured on this system.".to_string());
    }

    let input_hash = hash_code(input_code);
    let content = match fs::read_to_string(&storage_path) {
        Ok(c) => c,
        Err(e) => return (false, format!("Cannot read recovery auth file: {}", e)),
    };

    let mut found = false;
    let mut already_used = false;
    let mut new_lines = Vec::new();
    let now_ts = chrono_now();

    for line in content.lines() {
        if line.starts_with('#') || line.is_empty() {
            new_lines.push(line.to_string());
            continue;
        }

        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 2 && parts[0].eq_ignore_ascii_case(&input_hash) {
            found = true;
            if parts[1].eq_ignore_ascii_case("UNUSED") {
                new_lines.push(format!("{}:USED:{}:consumed_{}", parts[0], parts[2], now_ts));
            } else {
                already_used = true;
                new_lines.push(line.to_string());
            }
        } else {
            new_lines.push(line.to_string());
        }
    }

    if !found {
        (false, "Invalid recovery token: Hash not recognized.".to_string())
    } else if already_used {
        (false, "Token Rejected: This emergency code has already been consumed.".to_string())
    } else {
        let updated_content = new_lines.join("\n") + "\n";
        let _ = fs::write(&storage_path, updated_content);
        (true, "Authentication Verified! Emergency recovery token consumed successfully.".to_string())
    }
}

fn format_emergency_kit_document(codes: &[String]) -> String {
    let hostname = std::fs::read_to_string("/etc/hostname").unwrap_or_else(|_| "pop-os".to_string());
    let user = std::env::var("USER").unwrap_or_else(|_| "mzia".to_string());

    let mut doc = String::new();
    doc.push_str("================================================================================\n");
    doc.push_str("       🛡️ PULSARKEY EMERGENCY RECOVERY KIT & PAPER KEY (Pop!_OS COSMIC)        \n");
    doc.push_str("================================================================================\n\n");
    doc.push_str(&format!("Host System:        {}\n", hostname.trim()));
    doc.push_str(&format!("Target User:        {}\n", user));
    doc.push_str(&format!("Generated On:       {}\n", chrono_now()));
    doc.push_str("Security Mode:      SHA-256 Hashed Hardware Token Bypass Codes\n\n");
    doc.push_str("--------------------------------------------------------------------------------\n");
    doc.push_str("⚠️  CRITICAL RECOVERY NOTICE:\n");
    doc.push_str("   Print or store this paper key in a physically secure location (e.g. safe).\n");
    doc.push_str("   Each emergency code can be used ONCE to authenticate or bypass PAM hardware\n");
    doc.push_str("   enforcement if all physical YubiKeys are lost, damaged, or unreachable.\n");
    doc.push_str("--------------------------------------------------------------------------------\n\n");
    doc.push_str("EMERGENCY RECOVERY TOKENS (One-Time Use):\n");
    doc.push_str("┌──────────────────────────────────────────────────────────────────────────────┐\n");
    for (i, c) in codes.iter().enumerate() {
        doc.push_str(&format!("│  [{}]  {:<68}│\n", i + 1, c.bold()));
    }
    doc.push_str("└──────────────────────────────────────────────────────────────────────────────┘\n\n");
    doc.push_str("OFFLINE EMERGENCY RECOVERY PROCEDURES:\n\n");
    doc.push_str("1. System Shell Recovery Verification:\n");
    doc.push_str("   pulsarkey rescue verify <RECOVERY-CODE>\n\n");
    doc.push_str("2. Single-User Emergency Boot (Hardware Missing / Lockout):\n");
    doc.push_str("   a. Reboot machine, press SPACE repeatedly on system boot to open systemd-boot.\n");
    doc.push_str("   b. Press 'e' on Pop_OS kernel entry, append 'init=/bin/bash' to linux params.\n");
    doc.push_str("   c. Press ENTER to boot directly into root emergency shell.\n");
    doc.push_str("   d. Run emergency bypass: pulsarkey rollback  (or pulsarkey profile convenience)\n\n");
    doc.push_str("3. Live USB Chroot PAM Bypass One-Liner:\n");
    doc.push_str("   sudo sed -i 's/^auth.*pam_u2f.so.*/# &/' /mnt/etc/pam.d/sudo /mnt/etc/pam.d/cosmic-greeter\n\n");
    doc.push_str("================================================================================\n");
    doc
}

pub fn generate_offline_rescue_usb(target_dir: &Path) -> Result<PathBuf, String> {
    if !target_dir.exists() {
        return Err(format!("Target directory '{}' does not exist.", target_dir.display()));
    }

    let script_path = target_dir.join("pulsar-rescue.sh");
    let script_content = r#"#!/usr/bin/env bash
# ==============================================================================
# 🌌 PulsarKey Automated Emergency Rescue & PAM Bypass Script
# Designed for Pop!_OS COSMIC Live USB environments
# ==============================================================================

set -e

if [ "$EUID" -ne 0 ]; then
    echo "❌ Error: This rescue script must be run as root: sudo bash pulsar-rescue.sh"
    exit 1
fi

echo "=================================================================="
echo " 🛡️ PulsarKey Emergency System Rescue & PAM Restore"
echo "=================================================================="

# Detect root partition / LUKS cryptdata
MOUNT_DIR="/mnt/pulsar_rescue_root"
mkdir -p "$MOUNT_DIR"

if [ -f "/etc/pam.d/sudo" ] && [ -d "/etc/yubico" ]; then
    echo "👉 Running directly on target host system."
    TARGET_PAM="/etc/pam.d"
else
    echo "👉 Searching for Pop!_OS root filesystem..."
    ROOT_DEV=""
    if [ -e "/dev/mapper/data-root" ]; then
        ROOT_DEV="/dev/mapper/data-root"
    elif [ -e "/dev/mapper/cryptdata" ]; then
        ROOT_DEV="/dev/mapper/cryptdata"
    else
        # Try to find LUKS partition
        for dev in $(lsblk -lno NAME,FSTYPE | grep crypto_LUKS | awk '{print "/dev/"$1}'); do
            echo "🔍 Found LUKS container: $dev"
            echo "Enter disk encryption passphrase if prompted:"
            cryptsetup luksOpen "$dev" pulsar_crypt_rescue || true
            if [ -e "/dev/mapper/pulsar_crypt_rescue" ]; then
                ROOT_DEV="/dev/mapper/pulsar_crypt_rescue"
                break
            fi
        done
    fi

    if [ -z "$ROOT_DEV" ]; then
        echo "⚠️ Could not auto-detect LUKS partition. Please enter root device manually (e.g. /dev/nvme0n1p3):"
        read -r USER_DEV
        ROOT_DEV="$USER_DEV"
    fi

    echo "📦 Mounting $ROOT_DEV to $MOUNT_DIR..."
    mount "$ROOT_DEV" "$MOUNT_DIR" || {
        echo "❌ Mount failed. Ensure disk is decrypted."
        exit 1
    }
    TARGET_PAM="$MOUNT_DIR/etc/pam.d"
fi

echo "📝 Restoring /etc/pam.d to standard password authentication..."
for file in sudo cosmic-greeter polkit-1 common-auth; do
    if [ -f "$TARGET_PAM/$file" ]; then
        cp "$TARGET_PAM/$file" "$TARGET_PAM/$file.rescue.bak"
        # Disable pam_u2f line
        sed -i 's/^auth.*pam_u2f.so.*/# &/' "$TARGET_PAM/$file"
        echo "  ✓ Disabled hardware lock in $file"
    fi
done

echo ""
echo "🎉 Emergency bypass completed successfully!"
echo "You can now reboot. Standard password login is fully restored."
echo "=================================================================="
"#;

    if let Err(e) = fs::write(&script_path, script_content) {
        return Err(format!("Failed to write rescue script: {}", e));
    }

    let _ = fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755));
    Ok(script_path)
}

pub fn print_runbook() {
    println!("{}", "================================================================================".cyan());
    println!("{}", " 📖 PulsarKey Emergency Rescue Runbook (Pop!_OS COSMIC)".bold().cyan());
    println!("{}", "================================================================================".cyan());
    println!("\n{}", "Scenario: What to do if all registered YubiKeys are lost, broken, or locked:".bold().yellow());

    println!("\n{}", "Method 1: Offline Emergency Paper Key Verification".bold());
    println!("  If you have root or console access and an unused emergency recovery code:");
    println!("  Run: {}", "pulsarkey rescue verify <XXXX-XXXX-XXXX-XXXX>".green());
    println!("  This consumes the one-time paper token and authorizes emergency operations.");

    println!("\n{}", "Method 2: Built-in Single-User Mode Recovery (No Live USB needed)".bold());
    println!("  1. Restart your computer.");
    println!("  2. Repeatedly press {} as soon as the computer powers on to show systemd-boot menu.", "SPACEBAR".bold());
    println!("  3. Highlight {} and press {} to edit boot parameters.", "Pop!_OS (Current)".green(), "e".bold());
    println!("  4. Append {} to the end of the kernel parameters line.", "init=/bin/bash".cyan());
    println!("  5. Press {} to boot directly into an unauthenticated root maintenance shell.", "ENTER".bold());
    println!("  6. Mount root as read-write: {}", "mount -o remount,rw /".cyan());
    println!("  7. Revert PAM to standard:   {}", "pulsarkey profile convenience".green());
    println!("  8. Reboot normally:          {}", "reboot -f".cyan());

    println!("\n{}", "Method 3: Live USB Boot & One-Liner Chroot Bypass".bold());
    println!("  1. Boot from any Pop!_OS or Ubuntu installation Live USB.");
    println!("  2. Open terminal and unlock your encrypted storage:");
    println!("     {}", "sudo cryptsetup luksOpen /dev/nvme0n1p3 cryptdata".cyan());
    println!("     {}", "sudo mount /dev/mapper/data-root /mnt".cyan());
    println!("  3. Execute the emergency PAM disable one-liner:");
    println!("     {}", "sudo sed -i 's/^auth.*pam_u2f.so.*/# &/' /mnt/etc/pam.d/sudo /mnt/etc/pam.d/cosmic-greeter".green());
    println!("  4. Reboot into your desktop with standard password access!");

    println!("\n{}", "Method 4: Automated Offline Rescue USB Script".bold());
    println!("  Create an offline rescue script on any flash drive before emergencies:");
    println!("  Run: {}", "sudo pulsarkey rescue usb /media/$USER/<USB_NAME>".green());
    println!("{}", "================================================================================".cyan());
}

pub fn handle_rescue_cli(action: Option<String>, arg: Option<String>) {
    match action.as_deref() {
        Some("generate") => {
            println!("{}", "🛡️ Generating PulsarKey Emergency Recovery Paper Key...".bold());
            match generate_new_recovery_set() {
                Ok((codes, auth_path, export_path)) => {
                    println!("{} Generated 8 high-entropy single-use recovery codes.", "✅".green());
                    println!("{} Hashed auth store:  {}", "🔒".green(), auth_path.display().to_string().cyan());
                    println!("{} Printable kit saved: {}", "📄".green(), export_path.display().to_string().bold().green());
                    println!("\n{}", "┌────────────────────────────────────────────────────────────────────────┐".dimmed());
                    for (i, c) in codes.iter().enumerate() {
                        println!("│  [{}]  {:<64}│", i + 1, c.bold().yellow());
                    }
                    println!("{}", "└────────────────────────────────────────────────────────────────────────┘".dimmed());
                    println!("\n{}", "⚠️ Keep this document confidential. Store printed paper in a fireproof safe.".bold().red());
                }
                Err(e) => eprintln!("{} Generation failed: {}", "❌".red(), e),
            }
        }
        Some("status") => {
            let st = check_recovery_status();
            println!("{}", "==================================================".cyan());
            println!("{}", " 🛡️ PulsarKey Emergency Paper Key Status".bold().cyan());
            println!("{}", "==================================================".cyan());
            println!("Configured:         {}", if st.is_configured { "Yes (Active)".green() } else { "No (Run 'pulsarkey rescue generate')".yellow() });
            println!("Unused Tokens:      {}", format!("{}/{} remaining", st.unused, st.total).bold().cyan());
            println!("Consumed Tokens:    {}", st.used.to_string().dimmed());
            println!("Auth Store:         {}", st.file_path.display().to_string().dimmed());
            println!("{}", "==================================================".cyan());
        }
        Some("verify") => {
            if let Some(code) = arg {
                let (success, msg) = verify_and_consume_code(&code);
                if success {
                    println!("{} {}", "✅".green(), msg.bold().green());
                } else {
                    eprintln!("{} {}", "❌".red(), msg.bold().red());
                    std::process::exit(1);
                }
            } else {
                eprintln!("{} Please provide code: pulsarkey rescue verify <XXXX-XXXX-XXXX-XXXX>", "❌".red());
                std::process::exit(1);
            }
        }
        Some("runbook") => {
            print_runbook();
        }
        Some("usb") => {
            let target = arg.unwrap_or_else(|| "/tmp".to_string());
            let p = PathBuf::from(&target);
            match generate_offline_rescue_usb(&p) {
                Ok(out) => {
                    println!("{} Offline emergency rescue script written to: {}", "✅".green(), out.display().to_string().bold().green());
                    println!("To execute in emergency: {}", format!("sudo bash {}", out.display()).cyan());
                }
                Err(e) => eprintln!("{} Failed to create rescue script: {}", "❌".red(), e),
            }
        }
        Some(other) => {
            eprintln!("Unknown rescue action '{}'. Available: generate, status, verify, runbook, usb", other);
        }
        None => {
            print_runbook();
        }
    }
}

fn chrono_now() -> String {
    let output = Command::new("date")
        .arg("+%Y-%m-%d %H:%M:%S %Z")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "2026-09-15 00:00:00 UTC".to_string());
    output.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_format() {
        let code = generate_recovery_code();
        let parts: Vec<&str> = code.split('-').collect();
        assert_eq!(parts.len(), 4, "Recovery code should have 4 hyphen-separated chunks");
        for chunk in parts {
            assert_eq!(chunk.len(), 4, "Each chunk should be 4 characters");
            for c in chunk.chars() {
                assert!(CODE_CHARSET.contains(&(c as u8)), "Character {} not in charset", c);
            }
        }
    }

    #[test]
    fn test_hashing_consistency() {
        let code1 = "A2B3-C4D5-E6F7-G8H9";
        let code2 = "a2b3-c4d5-e6f7-g8h9";
        let code3 = "a2b3c4d5e6f7g8h9";
        let h1 = hash_code(code1);
        let h2 = hash_code(code2);
        let h3 = hash_code(code3);
        assert_eq!(h1, h2, "Hashing should be case-insensitive");
        assert_eq!(h1, h3, "Hashing should ignore hyphens");
        assert_eq!(h1.len(), 64, "SHA-256 hex string should be 64 characters");
    }
}

