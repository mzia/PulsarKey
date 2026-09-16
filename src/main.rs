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
const PAM_POLKIT: &str = "/etc/pam.d/polkit-1";
const TEMPLATE_POLKIT: &str = "/usr/lib/pam.d/polkit-1";

use pulsarkey::*;

#[derive(Parser)]
#[command(
    name = "pulsarkey",
    author = "M. Zia <mzia@pop-os.local>",
    version,
    about = "Hardware-backed FIDO2 & Biometric Authentication Manager for Pop!_OS COSMIC"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Setup and configure YubiKey FIDO2 for COSMIC greeter, sudo & polkit
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
    /// View or configure Auto-Lock on YubiKey removal
    Autolock {
        /// Action: "enable", "disable", or "status"
        #[arg(value_name = "ACTION")]
        action: Option<String>,
    },
    /// Generate hardware-backed SSH keys and configure Git commit signing
    SshSetup {
        /// Do not store resident key on token
        #[arg(long)]
        no_resident: bool,
        /// Do not configure Git commit signing
        #[arg(long)]
        no_git_sign: bool,
        /// Custom key output path (default: ~/.ssh/id_ed25519_sk)
        #[arg(long, value_name = "PATH")]
        key_path: Option<String>,
    },
    /// Manage on-key fingerprints and biometrics (enroll, list, delete, rename)
    Bio {
        #[command(subcommand)]
        action: Option<BioCommands>,
    },
    /// Manage FIDO2 hardware PIN code
    Pin {
        /// Action: "change", "set", "verify", or "status"
        #[arg(value_name = "ACTION")]
        action: Option<String>,
    },
    /// Manage security strictness profiles (convenience, fortress, lockdown)
    Profile {
        /// Profile name: "convenience", "fortress", "lockdown", or "status"
        #[arg(value_name = "PROFILE")]
        action: Option<String>,
    },
    /// View authentication audit journal (recent pulses)
    Audit {
        /// Clear the audit journal log
        #[arg(long)]
        clear: bool,
    },
    /// Manage secondary / backup security keys (pairing, testing, recovery)
    Backup {
        /// Action: "status", "pair", or "test"
        #[arg(value_name = "ACTION")]
        action: Option<String>,
    },
    /// Launch the COSMIC native settings control panel GUI
    Gui,
    /// Open the COSMIC native settings control panel GUI
    Settings,
    /// Manage emergency paper recovery keys and offline rescue runbooks
    Rescue {
        /// Action: "generate", "status", "verify", "runbook", or "usb"
        #[arg(value_name = "ACTION")]
        action: Option<String>,
        /// Optional argument (e.g. recovery code or USB mount path)
        #[arg(value_name = "ARG")]
        arg: Option<String>,
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
        Commands::Autolock { action } => {
            handle_autolock(action);
        }
        Commands::SshSetup {
            no_resident,
            no_git_sign,
            key_path,
        } => {
            ssh_setup::run_ssh_setup(no_resident, no_git_sign, key_path);
        }
        Commands::Bio { action } => {
            bio::handle_bio_cli(action);
        }
        Commands::Pin { action } => {
            bio::handle_pin_cli(action);
        }
        Commands::Profile { action } => {
            profiles::handle_profile_cli(action);
        }
        Commands::Audit { clear } => {
            audit::print_audit_log(clear);
        }
        Commands::Backup { action } => {
            backup::handle_backup_cli(action);
        }
        Commands::Gui | Commands::Settings => {
            if let Err(e) = gui::run_gui() {
                eprintln!("Failed to launch PulsarKey Settings GUI: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Rescue { action, arg } => {
            rescue::handle_rescue_cli(action, arg);
        }
    }
}

fn handle_autolock(action: Option<String>) {
    let mut cfg = config::load_config();
    match action.as_deref() {
        Some("enable") | Some("on") | Some("1") => {
            cfg.autolock = true;
            let _ = config::save_config(&cfg);
            println!("{} Auto-Lock on YubiKey removal: {}", "🛡️".green(), "ENABLED".bold().green());
        }
        Some("disable") | Some("off") | Some("0") => {
            cfg.autolock = false;
            let _ = config::save_config(&cfg);
            println!("{} Auto-Lock on YubiKey removal: {}", "🛡️".yellow(), "DISABLED".bold().yellow());
        }
        Some("status") | None => {
            let state = if cfg.autolock { "ENABLED".green() } else { "DISABLED".yellow() };
            println!("🛡️ Auto-Lock on YubiKey removal is currently: {}", state.bold());
            println!("To toggle: {}", "pulsarkey autolock [enable|disable]".cyan());
        }
        Some(other) => {
            eprintln!("Unknown action: '{}'. Please use 'enable', 'disable', or 'status'.", other);
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

    // 6. Update PAM Files (Sudo, COSMIC Greeter, Polkit GUI)
    println!("\n{}", "📝 Step 6: Updating PAM configurations...".bold());
    let sudo_pam_line = format!(
        "auth sufficient pam_u2f.so authfile={} cue [cue_prompt=Scan your fingerprint...] nouserok",
        MAPPING_FILE
    );
    let greeter_pam_line = format!(
        "auth sufficient pam_u2f.so authfile={} interactive [prompt=Press Space then Enter to scan YubiKey...] cue [cue_prompt=Scan your fingerprint...] nouserok",
        MAPPING_FILE
    );
    let polkit_pam_line = format!(
        "auth sufficient pam_u2f.so authfile={} cue [cue_prompt=Scan your fingerprint...] nouserok",
        MAPPING_FILE
    );

    update_pam_file(PAM_SUDO, &sudo_pam_line);
    update_pam_file(PAM_GREETER, &greeter_pam_line);
    update_pam_file(PAM_POLKIT, &polkit_pam_line);

    println!("\n{}", "==================================================".green());
    println!("{}", "🎉 Configuration finished successfully!".bold().green());
    println!("{}", "==================================================".green());
    println!("Verification Steps:");
    println!("  1. Sudo CLI:             {}", "sudo -k && sudo whoami".bold().cyan());
    println!("  2. Polkit GUI dialogs:   {}", "pkexec whoami".bold().cyan());
    println!("  3. COSMIC Lockscreen:    Lock with {} and press Space then Enter.", "Super + L".bold().cyan());
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
    let mut clean_lines: Vec<String> = Vec::new();

    if !p.exists() {
        // If polkit-1 doesn't exist in /etc/pam.d, initialize it from system template in /usr/lib/pam.d
        if path == PAM_POLKIT && Path::new(TEMPLATE_POLKIT).exists() {
            println!("Initializing {} from system template...", path.cyan());
            if let Ok(f) = File::open(TEMPLATE_POLKIT) {
                for l in BufReader::new(f).lines().flatten() {
                    if !l.contains("pam_u2f.so") {
                        clean_lines.push(l);
                    }
                }
            }
        } else {
            println!("{} {} does not exist, skipping.", "[-]".yellow(), path);
            return;
        }
    } else {
        let file = match File::open(p) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("{} Could not open {}: {}", "❌".red(), path, e);
                return;
            }
        };

        let reader = BufReader::new(file);
        for line_res in reader.lines().flatten() {
            // Strip any existing pam_u2f lines
            if !line_res.contains("pam_u2f.so") {
                clean_lines.push(line_res);
            }
        }
    }

    // Insert the new line at the top
    clean_lines.insert(0, line_to_insert.to_string());

    let mut out = match OpenOptions::new().write(true).create(true).truncate(true).open(p) {
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

    // 1. Remove PAM lines from Sudo, Greeter, Polkit
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

    // Clean up /etc/pam.d/polkit-1
    if Path::new(PAM_POLKIT).exists() {
        println!("Restoring system default for {}...", PAM_POLKIT);
        let _ = fs::remove_file(PAM_POLKIT);
        println!("{} Removed override {}", "✅".green(), PAM_POLKIT);
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

    // Check PAM polkit-1
    check_pam_status("polkit-1 (GUI)", PAM_POLKIT);

    // Check Security Profile
    let current_profile = profiles::get_current_profile();
    println!("Security Profile:        {}", current_profile.display_name().bold().green());

    // Check Auto-Lock
    let cfg = config::load_config();
    println!(
        "Sentinel Auto-Lock:      {}",
        if cfg.autolock { "Enabled (locks desktop on removal)".green() } else { "Disabled".yellow() }
    );

    // Check SSH & Git Signing
    let (ssh_status, git_status) = ssh_setup::check_ssh_git_status();
    println!("Hardware SSH Key:        {}", ssh_status.cyan());
    println!("Git Commit Signing:      {}", git_status.cyan());

    // Check Biometric Engine
    let bio_status = bio::check_bio_status();
    println!("Biometric Engine:        {}", bio_status);

    // Check Backup Redundancy
    let username = std::env::var("USER").unwrap_or_else(|_| "mzia".to_string());
    let enrolled_keys = backup::get_enrolled_keys(&username);
    println!(
        "Key Redundancy:          {}",
        if enrolled_keys.len() > 1 {
            format!("Protected ({} keys enrolled)", enrolled_keys.len()).green()
        } else {
            "Single key enrolled (No backup)".yellow()
        }
    );

    // Check Emergency Rescue Kit
    let rescue_st = rescue::check_recovery_status();
    println!(
        "Emergency Rescue Kit:    {}",
        if rescue_st.is_configured {
            format!("Active ({} paper tokens available)", rescue_st.unused).green()
        } else {
            "Not generated (Run 'pulsarkey rescue generate')".yellow()
        }
    );

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
        println!("PAM {:<20} {}", format!("{}:", name), "Standard (Password only)".yellow());
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
