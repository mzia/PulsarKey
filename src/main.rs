use clap::{Parser, Subcommand};
use colored::*;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const MAPPING_DIR: &str = "/etc/yubico";
const MAPPING_FILE: &str = "/etc/yubico/u2f_keys";
const UDEV_RULE_FILE: &str = "/etc/udev/rules.d/70-yubikey-cosmic.rules";
const PAM_SUDO: &str = "/etc/pam.d/sudo";
const PAM_GREETER: &str = "/etc/pam.d/cosmic-greeter";

mod applet;

#[derive(Parser)]
#[command(
    name = "pulsarkey",
    author = "M. Zia <mzia@pop-os.local>",
    version = "1.0.0",
    about = "Hardware-backed FIDO2 & Biometric Authentication Manager for Pop!_OS COSMIC"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Setup and configure YubiKey FIDO2 for COSMIC greeter & sudo
    Setup {
        /// Force re-installation of dependencies
        #[arg(long)]
        reinstall_packages: bool,
    },
    /// Revert all PAM configurations, udev rules, and mappings
    Uninstall {
        /// Also purge libpam-u2f and pamu2fcfg packages
        #[arg(long)]
        purge_packages: bool,
    },
    /// Show current status of PAM files, hardware, and key mappings
    Status,
    /// Run the COSMIC Panel status applet
    Applet {
        /// Install desktop autostart so the applet launches automatically on login
        #[arg(long)]
        install_autostart: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Setup { reinstall_packages } => {
            ensure_root("setup");
            run_setup(reinstall_packages);
        }
        Commands::Uninstall { purge_packages } => {
            ensure_root("uninstall");
            run_uninstall(purge_packages);
        }
        Commands::Status => {
            run_status();
        }
        Commands::Applet { install_autostart } => {
            if install_autostart {
                install_applet_autostart();
            }
            println!("🌌 Launching PulsarKey COSMIC Panel Applet...");
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(applet::run_applet());
        }
    }
}

fn install_applet_autostart() {
    let (_, user_home) = get_target_user();
    let autostart_dir = user_home.join(".config/autostart");
    let apps_dir = user_home.join(".local/share/applications");

    let _ = fs::create_dir_all(&autostart_dir);
    let _ = fs::create_dir_all(&apps_dir);

    let desktop_content = "[Desktop Entry]\n\
Name=PulsarKey Security Applet\n\
Comment=COSMIC Panel Status Applet for YubiKey FIDO2\n\
Exec=/usr/bin/pulsarkey applet\n\
Icon=io.github.mzia.PulsarKey\n\
Terminal=false\n\
Type=Application\n\
Categories=COSMIC;Utility;Security;\n\
X-CosmicApplet=true\n";

    let autostart_file = autostart_dir.join("io.github.mzia.PulsarKey.desktop");
    let app_file = apps_dir.join("io.github.mzia.PulsarKey.desktop");

    let _ = fs::write(&autostart_file, desktop_content);
    let _ = fs::write(&app_file, desktop_content);
    println!("{} Installed autostart entry to {}", "✅".green(), autostart_file.display());
}

/// Detects the target non-root user even when running with sudo
fn get_target_user() -> (String, PathBuf) {
    let username = std::env::var("SUDO_USER")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "mzia".to_string());

    let home_dir = match std::env::var("SUDO_USER") {
        Ok(ref sudo_user) => PathBuf::from(format!("/home/{}", sudo_user)),
        Err(_) => std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(format!("/home/{}", username))),
    };

    (username, home_dir)
}

/// Ensures the program is executed with root/sudo privileges
fn ensure_root(action: &str) {
    let uid = unsafe { libc::geteuid() };
    if uid != 0 {
        eprintln!(
            "{} Root privileges required to {} system authentication.",
            "❌ Error:".bold().red(),
            action
        );
        eprintln!(
            "Please run with sudo: {}",
            format!("sudo pulsarkey {}", action).bold().yellow()
        );
        std::process::exit(1);
    }
}

// ---------------------------------------------------------
// SETUP
// ---------------------------------------------------------
fn run_setup(reinstall: bool) {
    let (username, user_home) = get_target_user();

    println!("{}", "==================================================".cyan());
    println!(
        "{} {}",
        "🌌 PulsarKey FIDO2 & Biometric Setup for user:".bold().green(),
        username.bold().yellow()
    );
    println!("{}", "==================================================".cyan());

    // 1. Verify/Install Packages
    println!("\n{}", "📦 Step 1: Checking required packages...".bold());
    let has_pamu2fcfg = Path::new("/usr/bin/pamu2fcfg").exists();
    let has_pam_u2f = Path::new("/usr/lib/x86_64-linux-gnu/security/pam_u2f.so").exists()
        || Path::new("/lib/x86_64-linux-gnu/security/pam_u2f.so").exists()
        || Path::new("/lib/security/pam_u2f.so").exists();

    if !has_pamu2fcfg || !has_pam_u2f || reinstall {
        println!("Installing libpam-u2f, pamu2fcfg, and yubikey-manager via apt...");
        let status = Command::new("apt")
            .args(["install", "-y", "libpam-u2f", "pamu2fcfg", "yubikey-manager"])
            .status();

        if let Err(e) = status {
            eprintln!("{} Failed to invoke apt: {}", "❌".red(), e);
            std::process::exit(1);
        }
    } else {
        println!("{} All required PAM and FIDO2 packages are installed.", "✅".green());
    }

    // 2. Hardware Udev Rules for cosmic-greeter
    println!("\n{}", "⚙️  Step 2: Configuring udev hardware rules...".bold());
    let udev_content = "KERNEL==\"hidraw*\", ATTRS{idVendor}==\"1050\", MODE=\"0660\", GROUP=\"cosmic-greeter\"\n";
    if let Err(e) = fs::write(UDEV_RULE_FILE, udev_content) {
        eprintln!("{} Failed writing udev rules: {}", "❌".red(), e);
        std::process::exit(1);
    }
    let _ = Command::new("udevadm").args(["control", "--reload-rules"]).status();
    let _ = Command::new("udevadm").args(["trigger"]).status();
    println!("{} udev rule installed at {}", "✅".green(), UDEV_RULE_FILE.cyan());

    // 3. Central Directory
    println!("\n{}", "📁 Step 3: Preparing credential storage...".bold());
    if let Err(e) = fs::create_dir_all(MAPPING_DIR) {
        eprintln!("{} Failed creating {}: {}", "❌".red(), MAPPING_DIR, e);
        std::process::exit(1);
    }

    // 4. Enroll Primary Key
    println!("\n{}", "🔑 Step 4: Primary YubiKey Enrollment".bold());
    println!(
        "👉 Insert your primary YubiKey and {} when the LED flashes...",
        "scan your fingerprint or touch".bold().yellow()
    );

    let primary_key = match enroll_key(Some(&username), true) {
        Ok(key) => {
            println!("{} Primary key registered with User Verification (Biometric)!", "✅".green());
            key
        }
        Err(_) => {
            println!("👉 User verification not supported or timed out; trying presence (touch only)...");
            match enroll_key(Some(&username), false) {
                Ok(key) => {
                    println!("{} Primary key registered with Touch Presence!", "✅".green());
                    key
                }
                Err(e) => {
                    eprintln!("{} Enrollment failed: {}", "❌".red(), e);
                    std::process::exit(1);
                }
            }
        }
    };

    // 5. Optional Backup Key Enrollment
    let mut final_mapping = primary_key;
    print!("\n❓ Do you have a secondary/backup YubiKey to enroll now? [y/N]: ");
    io::stdout().flush().unwrap();
    let mut resp = String::new();
    io::stdin().read_line(&mut resp).unwrap();

    if resp.trim().eq_ignore_ascii_case("y") {
        println!("\n{}", "🔑 Step 5: Backup YubiKey Enrollment".bold());
        println!(
            "👉 Insert your BACKUP YubiKey and {} when it flashes...",
            "scan your fingerprint or touch".bold().yellow()
        );

        let backup_key = match enroll_key(None, true) {
            Ok(key) => {
                println!("{} Backup key registered with User Verification!", "✅".green());
                key
            }
            Err(_) => {
                println!("👉 Trying presence (touch only)...");
                match enroll_key(None, false) {
                    Ok(key) => {
                        println!("{} Backup key registered with Touch Presence!", "✅".green());
                        key
                    }
                    Err(e) => {
                        eprintln!("{} Backup key enrollment failed: {}", "⚠️ Warning:".yellow(), e);
                        String::new()
                    }
                }
            }
        };

        if !backup_key.is_empty() {
            let clean_backup = backup_key.trim().trim_start_matches(':');
            final_mapping = format!("{}:{}", final_mapping.trim(), clean_backup);
            println!("{} Backup key appended to mapping.", "✅".green());
        }
    }

    // Save mapping file
    let final_content = format!("{}\n", final_mapping.trim());
    if let Err(e) = fs::write(MAPPING_FILE, &final_content) {
        eprintln!("{} Failed to write {}: {}", "❌".red(), MAPPING_FILE, e);
        std::process::exit(1);
    }
    let _ = Command::new("chmod").args(["644", MAPPING_FILE]).status();
    println!("{} Saved credential mapping to {}", "✅".green(), MAPPING_FILE.cyan());

    // Synchronize user home config to prevent stale files
    let user_yubico_dir = user_home.join(".config/Yubico");
    if user_yubico_dir.exists() {
        let user_key_file = user_yubico_dir.join("u2f_keys");
        let _ = fs::write(&user_key_file, &final_content);
        let _ = Command::new("chown")
            .args([format!("{}:{}", username, username), user_key_file.to_string_lossy().to_string()])
            .status();
        println!("{} Synchronized {}", "✅".green(), user_key_file.display());
    }

    // 6. Update PAM Files
    println!("\n{}", "📝 Step 6: Updating PAM configurations...".bold());
    let sudo_pam_line = format!(
        "auth sufficient pam_u2f.so authfile={} cue [cue_prompt=Scan your fingerprint...] nouserok",
        MAPPING_FILE
    );
    let greeter_pam_line = format!(
        "auth sufficient pam_u2f.so authfile={} interactive [prompt=Press Space then Enter to scan YubiKey...] cue [cue_prompt=Scan your fingerprint...] nouserok",
        MAPPING_FILE
    );

    update_pam_file(PAM_SUDO, &sudo_pam_line);
    update_pam_file(PAM_GREETER, &greeter_pam_line);

    println!("\n{}", "==================================================".green());
    println!("{}", "🎉 Configuration finished successfully!".bold().green());
    println!("{}", "==================================================".green());
    println!("Verification Steps:");
    println!("  1. Open a new terminal and test: {}", "sudo -k && sudo whoami".bold().cyan());
    println!("  2. Lock your screen with {} to test biometric unlock.", "Super + L".bold().cyan());
}

fn enroll_key(username_opt: Option<&str>, user_verification: bool) -> Result<String, String> {
    let mut cmd = Command::new("pamu2fcfg");

    if let Some(user) = username_opt {
        cmd.arg("-u").arg(user);
    } else {
        cmd.arg("-n");
    }

    if user_verification {
        cmd.arg("-V");
    }

    cmd.stdin(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    let output = cmd.output().map_err(|e| format!("Failed to run pamu2fcfg: {}", e))?;

    if !output.status.success() {
        return Err(format!("pamu2fcfg exited with status {}", output.status));
    }

    let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if result.is_empty() {
        return Err("pamu2fcfg returned empty credential output".into());
    }

    Ok(result)
}

fn update_pam_file(path: &str, line_to_insert: &str) {
    let p = Path::new(path);
    if !p.exists() {
        println!("{} {} does not exist, skipping.", "[-]".yellow(), path);
        return;
    }

    let file = match File::open(p) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{} Could not open {}: {}", "❌".red(), path, e);
            return;
        }
    };

    let reader = BufReader::new(file);
    let mut clean_lines: Vec<String> = Vec::new();

    for line_res in reader.lines() {
        if let Ok(line) = line_res {
            // Strip any existing pam_u2f lines
            if !line.contains("pam_u2f.so") {
                clean_lines.push(line);
            }
        }
    }

    // Insert the new line at the top
    clean_lines.insert(0, line_to_insert.to_string());

    let mut out = match OpenOptions::new().write(true).truncate(true).open(p) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{} Could not write to {}: {}", "❌".red(), path, e);
            return;
        }
    };

    for l in clean_lines {
        let _ = writeln!(out, "{}", l);
    }

    println!("{} Updated {}", "✅".green(), path.cyan());
}

// ---------------------------------------------------------
// UNINSTALL
// ---------------------------------------------------------
fn run_uninstall(purge: bool) {
    let (_username, user_home) = get_target_user();

    println!("{}", "==================================================".yellow());
    println!("{}", "🔄 Reverting FIDO2 YubiKey configuration...".bold().yellow());
    println!("{}", "==================================================".yellow());

    // 1. Remove PAM lines
    for pam_file in [PAM_SUDO, PAM_GREETER] {
        if Path::new(pam_file).exists() {
            println!("Cleaning up {}...", pam_file);
            let file = File::open(pam_file).unwrap();
            let reader = BufReader::new(file);
            let mut clean_lines: Vec<String> = Vec::new();

            for line in reader.lines().flatten() {
                if !line.contains("pam_u2f.so") {
                    clean_lines.push(line);
                }
            }

            let mut out = OpenOptions::new().write(true).truncate(true).open(pam_file).unwrap();
            for l in clean_lines {
                let _ = writeln!(out, "{}", l);
            }
            println!("{} Removed pam_u2f from {}", "✅".green(), pam_file);
        }
    }

    // 2. Remove udev rule
    if Path::new(UDEV_RULE_FILE).exists() {
        let _ = fs::remove_file(UDEV_RULE_FILE);
        let _ = Command::new("udevadm").args(["control", "--reload-rules"]).status();
        let _ = Command::new("udevadm").args(["trigger"]).status();
        println!("{} Removed udev rule: {}", "✅".green(), UDEV_RULE_FILE);
    }

    // 3. Remove system mapping
    if Path::new(MAPPING_FILE).exists() {
        let _ = fs::remove_file(MAPPING_FILE);
        println!("{} Deleted {}", "✅".green(), MAPPING_FILE);
    }
    if Path::new(MAPPING_DIR).exists() {
        let _ = fs::remove_dir(MAPPING_DIR);
    }

    // 4. User mapping cleanup
    let user_key_file = user_home.join(".config/Yubico/u2f_keys");
    if user_key_file.exists() {
        print!("❓ Remove user-level mapping ({})? [y/N]: ", user_key_file.display());
        io::stdout().flush().unwrap();
        let mut resp = String::new();
        io::stdin().read_line(&mut resp).unwrap();
        if resp.trim().eq_ignore_ascii_case("y") {
            let _ = fs::remove_file(&user_key_file);
            println!("{} Removed user mapping.", "✅".green());
        }
    }

    // 5. Optional Package Purge
    if purge {
        println!("Purging libpam-u2f, pamu2fcfg, and yubikey-manager...");
        let _ = Command::new("apt")
            .args(["purge", "-y", "libpam-u2f", "pamu2fcfg", "yubikey-manager"])
            .status();
        let _ = Command::new("apt").args(["autoremove", "--purge", "-y"]).status();
        println!("{} Packages purged.", "✅".green());
    }

    println!("\n{}", "🎉 Rollback complete! Password-only authentication restored.".bold().green());
}

// ---------------------------------------------------------
// STATUS
// ---------------------------------------------------------
fn run_status() {
    println!("{}", "==================================================".cyan());
    println!("{}", " 🔍 PulsarKey Security Status (Pop!_OS COSMIC)".bold().cyan());
    println!("{}", "==================================================".cyan());

    // Check packages
    let has_pamu2fcfg = Path::new("/usr/bin/pamu2fcfg").exists();
    println!(
        "pamu2fcfg tool:          {}",
        if has_pamu2fcfg { "Installed".green() } else { "Missing".red() }
    );

    // Check udev rule
    let has_udev = Path::new(UDEV_RULE_FILE).exists();
    println!(
        "COSMIC udev rules:       {}",
        if has_udev { "Configured".green() } else { "Not found".yellow() }
    );

    // Check mapping file
    let has_mapping = Path::new(MAPPING_FILE).exists();
    if has_mapping {
        if let Ok(content) = fs::read_to_string(MAPPING_FILE) {
            let keys_count = content.split(':').count().saturating_sub(1);
            let has_uv = content.contains("+verification");
            println!(
                "System Credential Map:   {} ({} keys registered, Biometrics/UV: {})",
                "Present".green(),
                keys_count.to_string().cyan(),
                if has_uv { "Enabled".green() } else { "Presence only".yellow() }
            );
        } else {
            println!("System Credential Map:   {}", "Unreadable".red());
        }
    } else {
        println!("System Credential Map:   {}", "Not present".yellow());
    }

    // Check PAM sudo
    check_pam_status("sudo", PAM_SUDO);

    // Check PAM cosmic-greeter
    check_pam_status("cosmic-greeter", PAM_GREETER);

    // Check connected YubiKey
    println!("\nHardware Detection:");
    let yk_output = Command::new("ykman").arg("info").output();
    match yk_output {
        Ok(out) if out.status.success() => {
            let info = String::from_utf8_lossy(&out.stdout);
            for line in info.lines().take(5) {
                println!("  {}", line.dimmed());
            }
        }
        _ => println!("  {}", "No YubiKey detected or ykman not installed.".dimmed()),
    }
    println!("{}", "==================================================".cyan());
}

fn check_pam_status(name: &str, path: &str) {
    let p = Path::new(path);
    if !p.exists() {
        println!("PAM {:<20} {}", format!("{}:", name), "File not found".red());
        return;
    }

    if let Ok(content) = fs::read_to_string(p) {
        if let Some(line) = content.lines().find(|l| l.contains("pam_u2f.so")) {
            let is_interactive = line.contains("interactive");
            println!(
                "PAM {:<20} {} (Interactive: {})",
                format!("{}:", name),
                "FIDO2 Enabled".green(),
                if is_interactive { "Yes".green() } else { "No (Direct touch)".yellow() }
            );
        } else {
            println!("PAM {:<20} {}", format!("{}:", name), "Standard (Password only)".yellow());
        }
    }
}
