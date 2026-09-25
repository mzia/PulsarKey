use clap::{Parser, Subcommand};
use colored::*;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const MAPPING_DIR: &str = "/etc/yubico";
const MAPPING_FILE: &str = "/etc/yubico/u2f_keys";
const UDEV_RULE_FILE: &str = "/etc/udev/rules.d/70-pulsarkey-cosmic.rules";
const LEGACY_UDEV_RULE_FILE: &str = "/etc/udev/rules.d/70-yubikey-cosmic.rules";
const TEMPLATE_POLKIT: &str = "/usr/lib/pam.d/polkit-1";

use pulsarkey::*;

#[derive(Parser)]
#[command(
    name = "pulsarkey",
    author = "M. Zia <mzia@pop-os.local>",
    version,
    about = "Hardware-backed FIDO2 & Biometric Authentication Manager for Pop!_OS COSMIC & macOS"
)]
struct Cli {
    /// Open the graphical Settings control panel view
    #[arg(short = 'g', long)]
    gui: bool,

    /// Force classic text menu instead of modern TUI dashboard
    #[arg(short = 'm', long)]
    menu: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Setup and configure FIDO2 Security Key for COSMIC greeter, sudo & polkit
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
    /// View or configure Auto-Lock on Security Key removal
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
    /// Launch the interactive TUI security dashboard
    Tui,
    /// Launch the interactive TUI security dashboard (alias)
    Dashboard,
    /// Manage background Presence Sentinel daemon (launchd on macOS, systemd on Linux)
    Daemon {
        /// Action: "install", "uninstall", "status", "start", or "stop"
        #[arg(value_name = "ACTION")]
        action: Option<String>,
    },
    /// Manage Apple Touch ID WebAuthn integration for PAM and sudo (macOS)
    Touchid {
        /// Action: "enable", "disable", or "status"
        #[arg(value_name = "ACTION")]
        action: Option<String>,
    },
    /// Check for and install software updates from GitHub
    Update {
        /// Only check for updates without installing
        #[arg(short, long)]
        check: bool,
        /// Force re-checking GitHub API bypassing cache
        #[arg(short, long)]
        force: bool,
        /// Automatically accept and install update if available
        #[arg(short, long)]
        yes: bool,
    },
}


#[allow(deprecated)]
fn main() {
    let cli = Cli::parse();

    // Fast-path: direct -g / --gui flag (Deprecated)
    if cli.gui {
        gui::print_deprecation_notice();
        return;
    }

    match cli.command {
        Some(Commands::Setup { reinstall_packages }) => {
            ensure_root("setup");
            run_setup(reinstall_packages);
        }
        Some(Commands::Uninstall { purge_packages }) => {
            ensure_root("uninstall");
            run_uninstall(purge_packages);
        }
        Some(Commands::Status) => {
            run_status();
        }
        Some(Commands::Applet { install_autostart }) => {
            if install_autostart {
                match platform::install_daemon_service() {
                    Ok(msg) => println!("{} {}", "✅".green(), msg),
                    Err(e) => eprintln!("{} Failed to install daemon service: {}", "❌".red(), e),
                }
            }
            println!("🌌 Launching PulsarKey Security Applet...");
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(applet::run_applet());
        }
        Some(Commands::Daemon { action }) => {
            handle_daemon_cli(action);
        }
        Some(Commands::Touchid { action }) => {
            handle_touchid_cli(action);
        }
        Some(Commands::Update { check, force, yes }) => {
            updater::handle_update_cli(check, force, yes);
        }
        Some(Commands::Autolock { action }) => {
            handle_autolock(action);
        }
        Some(Commands::SshSetup {
            no_resident,
            no_git_sign,
            key_path,
        }) => {
            ssh_setup::run_ssh_setup(no_resident, no_git_sign, key_path);
        }
        Some(Commands::Bio { action }) => {
            bio::handle_bio_cli(action);
        }
        Some(Commands::Pin { action }) => {
            bio::handle_pin_cli(action);
        }
        Some(Commands::Profile { action }) => {
            profiles::handle_profile_cli(action);
        }
        Some(Commands::Audit { clear }) => {
            audit::print_audit_log(clear);
        }
        Some(Commands::Backup { action }) => {
            backup::handle_backup_cli(action);
        }
        Some(Commands::Gui) | Some(Commands::Settings) => {
            gui::print_deprecation_notice();
        }
        Some(Commands::Rescue { action, arg }) => {
            rescue::handle_rescue_cli(action, arg);
        }
        Some(Commands::Tui) | Some(Commands::Dashboard) => {
            if let Err(e) = tui::run_tui() {
                eprintln!("Failed to launch TUI: {}. Falling back to CLI menu.", e);
                run_interactive_selection();
            }
        }
        None => {
            if cli.menu {
                run_interactive_selection();
            } else if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
                if let Err(e) = tui::run_tui() {
                    eprintln!("Failed to launch TUI: {}. Falling back to CLI menu.", e);
                    run_interactive_selection();
                }
            } else {
                #[cfg(target_os = "macos")]
                {
                    platform::launch_gui_bundle_app();
                }
                #[cfg(not(target_os = "macos"))]
                run_status();
            }
        }
    }
}

/// Interactive view selection when pulsarkey is run without arguments in a terminal
#[allow(deprecated)]
fn run_interactive_selection() {
    println!("{}", "==================================================".cyan());
    println!(
        "{} {}",
        "🌌 PulsarKey — Hardware Security Suite".bold().cyan(),
        format!("({})", platform::get_os_display_name()).dimmed()
    );
    println!("{}", "==================================================".cyan());
    println!("Please select a view or action:");
    println!("  {}  🌌 Interactive TUI Dashboard (Full Screen)", "[t]".bold().cyan());
    println!("  {}  📊 View Security Status Dashboard (Default)", "[1]".bold().green());
    println!("  {}  🛡️  Security Strictness Profiles", "[2]".bold().green());
    println!("  {}  🧬 On-Key Biometrics & PIN Manager", "[3]".bold().green());
    println!("  {}  👯 Backup Key Assistant", "[4]".bold().green());
    println!("  {}  🛟 Emergency Recovery Kit & Runbook", "[5]".bold().green());
    println!("  {}  🔑 Hardware SSH & Git Signing Setup", "[6]".bold().green());
    println!("  {}  🛡️  Presence Sentinel Auto-Lock", "[7]".bold().green());
    println!("  {}  🚀 Software Updates & Upgrade", "[u]".bold().cyan());
    println!("  {}  🚪 Exit", "[q]".bold().yellow());
    println!("{}", "==================================================".cyan());
    print!("Selection [1-7, t, u, or Enter for Status]: ");
    let _ = io::stdout().flush();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        match input.trim().to_lowercase().as_str() {
            "t" | "tui" | "dashboard" => {
                let _ = tui::run_tui();
            }
            "1" | "status" | "s" | "" => {
                println!();
                run_status();
            }
            "2" | "profile" | "p" => {
                println!();
                profiles::handle_profile_cli(None);
            }
            "3" | "bio" | "b" => {
                println!();
                bio::handle_bio_cli(None);
            }
            "4" | "backup" => {
                println!();
                backup::handle_backup_cli(None);
            }
            "5" | "rescue" | "r" => {
                println!();
                rescue::handle_rescue_cli(None, None);
            }
            "6" | "ssh" => {
                println!();
                ssh_setup::run_ssh_setup(false, false, None);
            }
            "7" | "autolock" => {
                println!();
                handle_autolock(None);
            }
            "u" | "update" | "upgrade" => {
                println!();
                updater::handle_update_cli(false, false, false);
            }
            "gui" | "g" | "settings" => {
                println!();
                gui::print_deprecation_notice();
            }
            "q" | "quit" | "exit" => {
                println!("Goodbye!");
            }
            other => {
                println!("Unknown option '{}'. Exiting.", other);
            }
        }
    }
}

fn handle_autolock(action: Option<String>) {
    let mut cfg = config::load_config();
    match action.as_deref() {
        Some("enable") | Some("on") | Some("1") => {
            cfg.autolock = true;
            let _ = config::save_config(&cfg);
            println!("{} Auto-Lock on Security Key removal: {}", "🛡️".green(), "ENABLED".bold().green());
        }
        Some("disable") | Some("off") | Some("0") => {
            cfg.autolock = false;
            let _ = config::save_config(&cfg);
            println!("{} Auto-Lock on Security Key removal: {}", "🛡️".yellow(), "DISABLED".bold().yellow());
        }
        Some("status") | None => {
            let state = if cfg.autolock { "ENABLED".green() } else { "DISABLED".yellow() };
            println!("🛡️ Auto-Lock on Security Key removal is currently: {}", state.bold());
            println!("To toggle: {}", "pulsarkey autolock [enable|disable]".cyan());
        }
        Some(other) => {
            eprintln!("Unknown action: '{}'. Please use 'enable', 'disable', or 'status'.", other);
        }
    }
}

fn handle_daemon_cli(action: Option<String>) {
    let act = action.unwrap_or_else(|| "status".to_string()).to_lowercase();
    match act.as_str() {
        "install" => {
            match platform::install_daemon_service() {
                Ok(msg) => println!("{} {}", "✅".green(), msg),
                Err(e) => eprintln!("{} Failed to install daemon service: {}", "❌".red(), e),
            }
        }
        "uninstall" => {
            match platform::uninstall_daemon_service() {
                Ok(msg) => println!("{} {}", "✅".green(), msg),
                Err(e) => eprintln!("{} Failed to uninstall daemon service: {}", "❌".red(), e),
            }
        }
        "status" => {
            let (active, desc) = platform::get_daemon_service_status();
            println!("{}", "==================================================".cyan());
            println!("🌌 PulsarKey Presence Sentinel Daemon Status");
            println!("{}", "==================================================".cyan());
            println!("Status:      {}", if active { desc.green().bold() } else { desc.yellow() });
            #[cfg(target_os = "macos")]
            {
                let plist = platform::get_launchd_plist_path();
                println!("Plist:       {}", plist.display());
                let home = std::env::var("HOME").unwrap_or_default();
                let log_file = format!("{}/.config/pulsarkey/daemon.log", home);
                if Path::new(&log_file).exists() {
                    println!("Log File:    {}", log_file);
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                let unit = platform::get_linux_systemd_unit_path();
                println!("Unit:        {}", unit.display());
                let autostart = platform::get_linux_autostart_desktop_path();
                println!("Autostart:   {}", autostart.display());
            }
        }
        "start" => {
            #[cfg(target_os = "macos")]
            {
                let plist = platform::get_launchd_plist_path();
                let _ = Command::new("launchctl").args(["load", "-w", &plist.to_string_lossy()]).status();
                println!("{} Started PulsarKey launchd agent.", "✅".green());
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = Command::new("systemctl").args(["--user", "start", "pulsarkey-applet.service"]).status();
                println!("{} Started PulsarKey systemd service.", "✅".green());
            }
        }
        "stop" => {
            #[cfg(target_os = "macos")]
            {
                let plist = platform::get_launchd_plist_path();
                let _ = Command::new("launchctl").args(["unload", &plist.to_string_lossy()]).status();
                println!("{} Stopped PulsarKey launchd agent.", "✅".green());
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = Command::new("systemctl").args(["--user", "stop", "pulsarkey-applet.service"]).status();
                println!("{} Stopped PulsarKey systemd service.", "✅".green());
            }
        }
        other => {
            eprintln!("Unknown daemon action '{}'. Use: install, uninstall, status, start, or stop.", other);
        }
    }
}

fn handle_touchid_cli(action: Option<String>) {
    let act = action.unwrap_or_else(|| "status".to_string()).to_lowercase();
    match act.as_str() {
        "status" => {
            let tid = crate::hardware::detect_touch_id();
            println!("{}", "==================================================".cyan());
            println!("🍏 Apple Touch ID (WebAuthn) Status");
            println!("{}", "==================================================".cyan());
            println!("Hardware Sensor:     {}", if tid.is_supported { "Present".green() } else { "Not Detected (Desktop Mac or Closed Lid)".yellow() });
            println!("Enrolled Biometrics: {}", if tid.is_enrolled { "Yes (Fingerprints Registered)".green() } else { "No fingerprints enrolled".yellow() });
            println!("PAM Module:          {}", if tid.pam_tid_available { "Available (/usr/lib/pam/pam_tid.so)".green() } else { "Not found".yellow() });
            println!("sudo_local Active:   {}", if tid.sudo_local_configured { "Yes (Touch ID enables sudo)".green().bold() } else { "Disabled".yellow() });
            if !tid.sudo_local_configured && tid.is_supported {
                println!("\n👉 To enable Touch ID for sudo alongside your security keys, run:\n   sudo pulsarkey touchid enable");
            }
        }
        "enable" => {
            ensure_root("touchid enable");
            let tid = crate::hardware::detect_touch_id();
            if !tid.is_supported {
                eprintln!("{} Touch ID hardware is not detected on this system.", "⚠️".yellow());
            }
            match platform::configure_touch_id_pam(true) {
                Ok(()) => println!("{} Apple Touch ID enabled in /etc/pam.d/sudo_local! Sudo will now accept Touch ID and Security Keys.", "✅".green()),
                Err(e) => eprintln!("{} Failed to configure Touch ID in sudo_local: {}", "❌".red(), e),
            }
        }
        "disable" => {
            ensure_root("touchid disable");
            match platform::configure_touch_id_pam(false) {
                Ok(()) => println!("{} Apple Touch ID disabled in /etc/pam.d/sudo_local. (Security Keys remain active)", "✅".green()),
                Err(e) => eprintln!("{} Failed to disable Touch ID in sudo_local: {}", "❌".red(), e),
            }
        }
        other => {
            eprintln!("Unknown touchid action '{}'. Use: status, enable, or disable.", other);
        }
    }
}

#[allow(dead_code)]
fn install_applet_autostart() {
    match platform::install_daemon_service() {
        Ok(msg) => println!("{} {}", "✅".green(), msg),
        Err(e) => eprintln!("{} Failed to install daemon service: {}", "❌".red(), e),
    }
}

/// Detects the target non-root user even when running with sudo
fn get_target_user() -> (String, PathBuf) {
    let username = std::env::var("SUDO_USER")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "mzia".to_string());

    let home_dir = match std::env::var("SUDO_USER") {
        Ok(ref sudo_user) => {
            #[cfg(target_os = "macos")]
            { PathBuf::from(format!("/Users/{}", sudo_user)) }
            #[cfg(not(target_os = "macos"))]
            { PathBuf::from(format!("/home/{}", sudo_user)) }
        }
        Err(_) => std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                #[cfg(target_os = "macos")]
                { PathBuf::from(format!("/Users/{}", username)) }
                #[cfg(not(target_os = "macos"))]
                { PathBuf::from(format!("/home/{}", username)) }
            }),
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
    #[cfg(target_os = "macos")]
    {
        let has_pamu2fcfg = Command::new("which").arg("pamu2fcfg").output().map(|o| o.status.success()).unwrap_or(false);
        let has_pam_u2f = Path::new("/opt/homebrew/lib/pam/pam_u2f.so").exists()
            || Path::new("/usr/local/lib/pam/pam_u2f.so").exists()
            || Path::new("/opt/homebrew/lib/security/pam_u2f.so").exists()
            || Path::new("/usr/local/lib/security/pam_u2f.so").exists()
            || Path::new("/usr/lib/pam/pam_u2f.so").exists();

        if !has_pamu2fcfg || !has_pam_u2f || reinstall {
            println!("Please install pam-u2f and ykman via Homebrew if not already installed:\n  brew install pam-u2f ykman");
        } else {
            println!("{} All required macOS PAM and FIDO2 packages are installed.", "✅".green());
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
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
    }

    // 2. Hardware Access Rules
    #[cfg(target_os = "macos")]
    {
        println!("\n{}", "⚙️  Step 2: Checking hardware access rules...".bold());
        println!("{} Native macOS IOKit USB HID subsystem active (udev rules not required).", "✅".green());
    }
    #[cfg(not(target_os = "macos"))]
    {
        println!("\n{}", "⚙️  Step 2: Configuring udev hardware rules...".bold());
        let udev_content = "# PulsarKey Universal FIDO2 Security Key rules for cosmic-greeter\n\
KERNEL==\"hidraw*\", SUBSYSTEM==\"hidraw\", ATTRS{idVendor}==\"1050|20a0|1209|096e|18d1|2021|2fc6\", MODE=\"0660\", GROUP=\"cosmic-greeter\"\n\
KERNEL==\"hidraw*\", SUBSYSTEM==\"hidraw\", ENV{ID_SECURITY_TOKEN}==\"1\", MODE=\"0660\", GROUP=\"cosmic-greeter\"\n";
        if Path::new(LEGACY_UDEV_RULE_FILE).exists() {
            let _ = fs::remove_file(LEGACY_UDEV_RULE_FILE);
        }
        if let Err(e) = fs::write(UDEV_RULE_FILE, udev_content) {
            eprintln!("{} Failed writing udev rules: {}", "❌".red(), e);
            std::process::exit(1);
        }
        let _ = Command::new("udevadm").args(["control", "--reload-rules"]).status();
        let _ = Command::new("udevadm").args(["trigger"]).status();
        println!("{} udev rule installed at {}", "✅".green(), UDEV_RULE_FILE.cyan());
    }

    // 3. Central Directory
    println!("\n{}", "📁 Step 3: Preparing credential storage...".bold());
    if let Err(e) = fs::create_dir_all(MAPPING_DIR) {
        eprintln!("{} Failed creating {}: {}", "❌".red(), MAPPING_DIR, e);
        std::process::exit(1);
    }

    // 4. Enroll Primary Key
    println!("\n{}", "🔑 Step 4: Primary Security Key Enrollment".bold());
    println!(
        "👉 Insert your primary Security Key and {} when the LED flashes...",
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
    print!("\n❓ Do you have a secondary/backup Security Key to enroll now? [y/N]: ");
    io::stdout().flush().unwrap();
    let mut resp = String::new();
    io::stdin().read_line(&mut resp).unwrap();

    if resp.trim().eq_ignore_ascii_case("y") {
        println!("\n{}", "🔑 Step 5: Backup Security Key Enrollment".bold());
        println!(
            "👉 Insert your BACKUP Security Key and {} when it flashes...",
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
        "auth sufficient pam_u2f.so authfile={} interactive [prompt=Press Space then Enter to scan Security Key...] cue [cue_prompt=Scan your fingerprint...] nouserok",
        MAPPING_FILE
    );
    let polkit_pam_line = format!(
        "auth sufficient pam_u2f.so authfile={} cue [cue_prompt=Scan your fingerprint...] nouserok",
        MAPPING_FILE
    );

    update_pam_file(platform::PAM_PATHS.sudo, &sudo_pam_line);
    if Path::new(platform::PAM_PATHS.greeter_or_screensaver).exists() {
        update_pam_file(platform::PAM_PATHS.greeter_or_screensaver, &greeter_pam_line);
    }
    if Path::new(platform::PAM_PATHS.elevation_service).exists() {
        update_pam_file(platform::PAM_PATHS.elevation_service, &polkit_pam_line);
    }

    #[cfg(target_os = "macos")]
    {
        let tid = crate::hardware::detect_touch_id();
        if tid.is_supported {
            println!("\n{}", "🍏 Step 7: Apple Touch ID (WebAuthn) Integration".bold());
            println!("Apple Touch ID biometric hardware was detected on this Mac.");
            println!("Configuring /etc/pam.d/sudo_local with Touch ID alongside Security Keys...");
            let _ = platform::configure_touch_id_pam(true);
            println!("{} Touch ID and Security Keys are both active for sudo!", "✅".green());
        }

        println!("\n{}", "🛡️ Step 8: Background Presence Sentinel".bold());
        println!("To automatically lock your Mac when you remove your Security Key, install the launchd agent:");
        println!("   {}", "pulsarkey daemon install".cyan());
    }
    #[cfg(not(target_os = "macos"))]
    {
        println!("\n{}", "🛡️ Step 7: COSMIC Panel Applet & Presence Sentinel".bold());
        println!("To launch the COSMIC top panel status applet and enable Presence Sentinel auto-lock on login:");
        println!("   {}", "pulsarkey daemon install".cyan());
    }

    println!("\n{}", "==================================================".green());
    println!("{}", "🎉 Configuration finished successfully!".bold().green());
    println!("{}", "==================================================".green());
    println!("Verification Steps:");
    #[cfg(target_os = "macos")]
    {
        println!("  1. Sudo CLI:             {}", "sudo -k && sudo whoami".bold().cyan());
        println!("  2. Screen Lock:          Wake screensaver and scan fingerprint or touch Security Key.");
        println!("  3. Menu Bar Applet:      {}", "pulsarkey applet".bold().cyan());
    }
    #[cfg(not(target_os = "macos"))]
    {
        println!("  1. Sudo CLI:             {}", "sudo -k && sudo whoami".bold().cyan());
        println!("  2. Polkit/Auth dialogs:  {}", "pkexec whoami".bold().cyan());
        println!("  3. Lockscreen:           Lock desktop and press Space then Enter.");
        println!("  4. COSMIC Panel Applet:  {}", "pulsarkey applet".bold().cyan());
    }
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
        // If elevation service doesn't exist in /etc/pam.d, initialize it from system template in /usr/lib/pam.d (Linux)
        if path == platform::PAM_PATHS.elevation_service && Path::new(TEMPLATE_POLKIT).exists() {
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
    println!("{}", "🔄 Reverting FIDO2 Security Key configuration...".bold().yellow());
    println!("{}", "==================================================".yellow());

    // 1. Remove PAM lines from Sudo, Greeter/Screensaver
    for pam_file in [platform::PAM_PATHS.sudo, platform::PAM_PATHS.greeter_or_screensaver] {
        if Path::new(pam_file).exists() {
            println!("Cleaning up {}...", pam_file);
            if let Ok(file) = File::open(pam_file) {
                let reader = BufReader::new(file);
                let mut clean_lines: Vec<String> = Vec::new();

                for line in reader.lines().flatten() {
                    if !line.contains("pam_u2f.so") {
                        clean_lines.push(line);
                    }
                }

                if let Ok(mut out) = OpenOptions::new().write(true).truncate(true).open(pam_file) {
                    for l in clean_lines {
                        let _ = writeln!(out, "{}", l);
                    }
                    println!("{} Removed pam_u2f from {}", "✅".green(), pam_file);
                }
            }
        }
    }

    // Clean up elevation service
    if Path::new(platform::PAM_PATHS.elevation_service).exists() {
        #[cfg(target_os = "linux")]
        {
            println!("Restoring system default for {}...", platform::PAM_PATHS.elevation_service);
            let _ = fs::remove_file(platform::PAM_PATHS.elevation_service);
            println!("{} Removed override {}", "✅".green(), platform::PAM_PATHS.elevation_service);
        }
        #[cfg(not(target_os = "linux"))]
        {
            if let Ok(file) = File::open(platform::PAM_PATHS.elevation_service) {
                let reader = BufReader::new(file);
                let mut clean_lines: Vec<String> = Vec::new();
                for line in reader.lines().flatten() {
                    if !line.contains("pam_u2f.so") {
                        clean_lines.push(line);
                    }
                }
                if let Ok(mut out) = OpenOptions::new().write(true).truncate(true).open(platform::PAM_PATHS.elevation_service) {
                    for l in clean_lines {
                        let _ = writeln!(out, "{}", l);
                    }
                    println!("{} Removed pam_u2f from {}", "✅".green(), platform::PAM_PATHS.elevation_service);
                }
            }
        }
    }

    // 2. Remove udev rules
    let mut udev_removed = false;
    for udev_path in [UDEV_RULE_FILE, LEGACY_UDEV_RULE_FILE] {
        if Path::new(udev_path).exists() {
            let _ = fs::remove_file(udev_path);
            println!("{} Removed udev rule: {}", "✅".green(), udev_path);
            udev_removed = true;
        }
    }
    if udev_removed {
        let _ = Command::new("udevadm").args(["control", "--reload-rules"]).status();
        let _ = Command::new("udevadm").args(["trigger"]).status();
    }

    // 3. Remove system mapping
    if Path::new(MAPPING_FILE).exists() {
        let _ = fs::remove_file(MAPPING_FILE);
        println!("{} Deleted {}", "✅".green(), MAPPING_FILE);
    }
    if Path::new(MAPPING_DIR).exists() {
        let _ = fs::remove_dir(MAPPING_DIR);
        println!("{} Deleted {}", "✅".green(), MAPPING_DIR);
    }

    // 4. Remove user mapping if present
    let user_key_file = user_home.join(".config/Yubico/u2f_keys");
    if user_key_file.exists() {
        let _ = fs::remove_file(&user_key_file);
        println!("{} Deleted {}", "✅".green(), user_key_file.display());
    }

    // 5. Purge Packages if requested
    if purge {
        #[cfg(target_os = "linux")]
        {
            println!("\nPurging libpam-u2f and pamu2fcfg via apt...");
            let _ = Command::new("apt")
                .args(["purge", "-y", "libpam-u2f", "pamu2fcfg"])
                .status();
        }
    }

    println!("\n{}", "==================================================".green());
    println!("{}", "🎉 PulsarKey successfully uninstalled.".bold().green());
    println!("{}", "==================================================".green());
}

// ---------------------------------------------------------
// STATUS
// ---------------------------------------------------------
fn run_status() {
    println!("{}", "==================================================".cyan());
    println!("{}", format!(" 🔍 PulsarKey Security Status ({})", platform::get_os_display_name()).bold().cyan());
    println!("{}", "==================================================".cyan());

    // Check hardware
    let (is_connected, hw_name) = platform::check_security_key_usb_connected();
    println!(
        "Security Hardware:       {}",
        if is_connected { hw_name.green().bold() } else { "None detected".yellow() }
    );

    // Check packages
    let has_pamu2fcfg = Path::new("/usr/bin/pamu2fcfg").exists()
        || Path::new("/opt/homebrew/bin/pamu2fcfg").exists()
        || Path::new("/usr/local/bin/pamu2fcfg").exists();
    println!(
        "pamu2fcfg tool:          {}",
        if has_pamu2fcfg { "Installed".green() } else { "Missing".red() }
    );

    // Check udev rule (Linux)
    #[cfg(target_os = "linux")]
    {
        let has_udev = Path::new(UDEV_RULE_FILE).exists() || Path::new(LEGACY_UDEV_RULE_FILE).exists();
        println!(
            "COSMIC udev rules:       {}",
            if has_udev { "Configured".green() } else { "Not found".yellow() }
        );
    }

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
    check_pam_status("sudo", platform::PAM_PATHS.sudo);

    // Check PAM greeter / screensaver
    check_pam_status(platform::PAM_PATHS.greeter_label, platform::PAM_PATHS.greeter_or_screensaver);

    // Check PAM elevation service (polkit or authorization)
    check_pam_status(platform::PAM_PATHS.elevation_label, platform::PAM_PATHS.elevation_service);

    // Check Apple Touch ID / WebAuthn & Daemon status on macOS
    #[cfg(target_os = "macos")]
    {
        let tid = crate::hardware::detect_touch_id();
        println!("\nApple Platform Biometrics (WebAuthn):");
        if tid.is_supported {
            println!(
                "  Hardware Sensor:       {}",
                if tid.is_enrolled { "Present & Enrolled".green() } else { "Present (No fingerprints enrolled)".yellow() }
            );
            println!(
                "  sudo_local Touch ID:   {}",
                if tid.sudo_local_configured { "Active (pam_tid.so enabled)".green().bold() } else { "Disabled (Run 'sudo pulsarkey touchid enable')".yellow() }
            );
        } else {
            println!("  Hardware Sensor:       {}", "Not Detected (Desktop Mac or Closed Lid)".dimmed());
        }
    }

    // Check Background Presence Sentinel Daemon / Applet
    let (daemon_active, daemon_desc) = platform::get_daemon_service_status();
    println!(
        "Background Sentinel:     {}",
        if daemon_active { daemon_desc.green().bold() } else { daemon_desc.yellow() }
    );

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

    // Check Software Updates
    if let Some(update_info) = updater::get_cached_update_info() {
        if update_info.is_update_available {
            println!(
                "Software Update:         {} (v{} -> v{}, run 'pulsarkey update')",
                "Update Available".yellow().bold(),
                update_info.current_version,
                update_info.latest_version.green().bold()
            );
        } else {
            println!("Software Update:         {} (v{})", "Up to date".green(), update_info.current_version);
        }
    } else {
        updater::spawn_background_update_check();
        println!("Software Update:         {} (Checking in background...)", env!("CARGO_PKG_VERSION").cyan());
    }


    // Check connected Security Key
    println!("\nHardware Detection:");
    let dev = crate::hardware::detect_fido_device();
    if dev.is_connected {
        println!("  Device:   {}", dev.product_name.green().bold());
        println!("  Vendor:   {:?}", dev.vendor);
        if let Some(ref serial) = dev.serial {
            println!("  Serial:   {}", serial);
        }
        if let Some(ref fw) = dev.firmware {
            println!("  Firmware: {}", fw);
        }
    } else {
        println!("  {}", dev.product_name.dimmed());
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
        let has_u2f = content.lines().any(|l| l.contains("pam_u2f.so"));
        let has_tid = content.lines().any(|l| !l.trim().starts_with('#') && l.contains("pam_tid.so"));

        if has_u2f && has_tid {
            println!("PAM {:<20} {}", format!("{}:", name), "FIDO2 + Apple Touch ID Enabled".green().bold());
        } else if has_u2f {
            let is_interactive = content.lines().any(|l| l.contains("interactive"));
            println!(
                "PAM {:<20} {} (Interactive: {})",
                format!("{}:", name),
                "FIDO2 Enabled".green(),
                if is_interactive { "Yes".green() } else { "No (Direct touch)".yellow() }
            );
        } else if has_tid {
            println!("PAM {:<20} {}", format!("{}:", name), "Apple Touch ID Enabled".green());
        } else {
            println!("PAM {:<20} {}", format!("{}:", name), "Standard (Password only)".yellow());
        }
    }
}
