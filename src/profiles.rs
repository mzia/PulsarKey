use colored::*;
use std::fs;
use std::path::Path;

#[cfg(not(target_os = "macos"))]
const TEMPLATE_POLKIT: &str = "/usr/lib/pam.d/polkit-1";
const MAPPING_FILE: &str = "/etc/yubico/u2f_keys";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityProfile {
    Convenience,
    Fortress,
    Lockdown,
}

impl SecurityProfile {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().trim() {
            "convenience" | "1fa" | "conv" => Some(Self::Convenience),
            "fortress" | "2fa" | "fort" => Some(Self::Fortress),
            "lockdown" | "strict" | "lock" => Some(Self::Lockdown),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Convenience => "convenience",
            Self::Fortress => "fortress",
            Self::Lockdown => "lockdown",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Convenience => "Convenience (1FA Biometric/Touch)",
            Self::Fortress => "Fortress (True 2FA: Password + Touch)",
            Self::Lockdown => "Lockdown (Hardware Strictly Mandatory)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Convenience => "Fingerprint or touch alone authorizes sudo, greeter, and Polkit GUI. Password fallback enabled if token is absent.",
            Self::Fortress => "Enforces True 2-Factor Authentication. Requires BOTH your account password AND physical security key touch.",
            Self::Lockdown => "Physical token strictly mandatory. Password fallback is completely disabled. Access denied if token is absent.",
        }
    }
}

/// Returns the currently active profile from configuration.
pub fn get_current_profile() -> SecurityProfile {
    let cfg = crate::config::load_config();
    SecurityProfile::from_str(&cfg.profile).unwrap_or(SecurityProfile::Convenience)
}

/// Applies a security profile across all PAM files.
pub fn apply_profile(profile: SecurityProfile) -> Result<(), String> {
    // 1. Safety check: ensure mapping file exists and has registered keys
    if !Path::new(MAPPING_FILE).exists() {
        return Err(format!(
            "Credential mapping file {} not found. Run 'sudo pulsarkey setup' first.",
            MAPPING_FILE
        ));
    }

    let map_content = fs::read_to_string(MAPPING_FILE)
        .map_err(|e| format!("Failed to read credential map: {}", e))?;
    if map_content.trim().is_empty() {
        return Err("No keys registered in credential map. Enroll a key before changing profiles.".to_string());
    }

    // 2. Prepare PAM lines based on profile
    let (sudo_line, greeter_line, polkit_line) = match profile {
        SecurityProfile::Convenience => (
            "auth sufficient pam_u2f.so authfile=/etc/yubico/u2f_keys cue [cue_prompt=Scan your fingerprint or touch your Security Key...] nouserok",
            "auth [success=done default=ignore] pam_u2f.so authfile=/etc/yubico/u2f_keys interactive [prompt=Press Enter to scan Security Key...] cue [cue_prompt=Scan your fingerprint or touch your Security Key...] nouserok",
            "auth sufficient pam_u2f.so authfile=/etc/yubico/u2f_keys cue [cue_prompt=Scan your fingerprint or touch your Security Key...] nouserok",
        ),
        SecurityProfile::Fortress => (
            "auth requisite pam_unix.so nullok_secure\nauth required pam_u2f.so authfile=/etc/yubico/u2f_keys cue [cue_prompt=Scan your fingerprint or touch your Security Key...]",
            "auth required pam_unix.so nullok_secure\nauth required pam_u2f.so authfile=/etc/yubico/u2f_keys interactive [prompt=Press Enter to scan Security Key...] cue [cue_prompt=Scan your fingerprint or touch your Security Key...]",
            "auth required pam_unix.so nullok_secure\nauth required pam_u2f.so authfile=/etc/yubico/u2f_keys cue [cue_prompt=Scan your fingerprint or touch your Security Key...]",
        ),
        SecurityProfile::Lockdown => (
            "auth [success=done default=die] pam_u2f.so authfile=/etc/yubico/u2f_keys cue [cue_prompt=Scan your fingerprint or touch your Security Key...]",
            "auth [success=done default=die] pam_u2f.so authfile=/etc/yubico/u2f_keys interactive [prompt=Press Enter to scan Security Key...] cue [cue_prompt=Scan your fingerprint or touch your Security Key...]",
            "auth [success=done default=die] pam_u2f.so authfile=/etc/yubico/u2f_keys cue [cue_prompt=Scan your fingerprint or touch your Security Key...]",
        ),
    };

    // 3. Inject into sudo
    update_pam_file(crate::platform::PAM_PATHS.sudo, sudo_line, profile == SecurityProfile::Fortress)?;

    // 4. Inject into cosmic-greeter / screensaver
    if Path::new(crate::platform::PAM_PATHS.greeter_or_screensaver).exists() {
        update_pam_file(crate::platform::PAM_PATHS.greeter_or_screensaver, greeter_line, profile == SecurityProfile::Fortress)?;
    }

    // 5. Inject into polkit-1 / authorization
    #[cfg(target_os = "linux")]
    if !Path::new(crate::platform::PAM_PATHS.elevation_service).exists() && Path::new(TEMPLATE_POLKIT).exists() {
        let _ = fs::copy(TEMPLATE_POLKIT, crate::platform::PAM_PATHS.elevation_service);
    }
    if Path::new(crate::platform::PAM_PATHS.elevation_service).exists() {
        update_pam_file(crate::platform::PAM_PATHS.elevation_service, polkit_line, profile == SecurityProfile::Fortress)?;
    }

    // 6. Persist to configuration
    let mut cfg = crate::config::load_config();
    cfg.profile = profile.as_str().to_string();
    let _ = crate::config::save_config(&cfg);

    // 7. Audit Logging
    crate::audit::log_event(
        "SECURITY",
        "Security Profile Changed",
        "Success",
        profile.display_name(),
    );

    // 8. Desktop Notification
    crate::platform::send_desktop_notification(
        "PulsarKey Profile",
        &format!("Active Security Profile: {}", profile.display_name()),
        false,
    );

    Ok(())
}

fn update_pam_file(path: &str, new_lines: &str, is_fortress: bool) -> Result<(), String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path, e))?;

    // Create safety backup
    let bak_path = format!("{}.pulsarkey.bak", path);
    if !Path::new(&bak_path).exists() {
        let _ = fs::write(&bak_path, &content);
    }

    // Remove any existing pam_u2f lines and clean up previous injections
    let mut cleaned_lines = Vec::new();
    for line in content.lines() {
        if line.contains("pam_u2f.so") {
            continue;
        }
        // In Fortress mode, pam_unix is placed explicitly at top
        if is_fortress && line.contains("pam_unix.so") && line.contains("nullok_secure") {
            continue;
        }
        cleaned_lines.push(line);
    }

    let mut final_content = String::new();
    final_content.push_str(new_lines);
    final_content.push('\n');
    final_content.push_str(&cleaned_lines.join("\n"));
    final_content.push('\n');

    fs::write(path, final_content)
        .map_err(|e| format!("Failed to write {}: {}", path, e))?;

    Ok(())
}

/// Prints current profile status and available profiles.
pub fn print_profile_status() {
    let current = get_current_profile();

    println!("{}", "==================================================".cyan());
    println!("{}", "🛡️ PulsarKey Security Strictness Profiles".bold().cyan());
    println!("{}", "==================================================".cyan());
    println!("Active Profile: {}\n", current.display_name().bold().green());

    println!("{}", "Available Profiles:".bold());
    for p in [
        SecurityProfile::Convenience,
        SecurityProfile::Fortress,
        SecurityProfile::Lockdown,
    ] {
        let is_active = p == current;
        let mark = if is_active { "● [ACTIVE]".bold().green() } else { "○".normal() };
        println!("  {} {}", mark, p.display_name().bold());
        println!("     {}", p.description().dimmed());
    }

    println!("\nSwitch profile with:");
    println!("  sudo pulsarkey profile convenience");
    println!("  sudo pulsarkey profile fortress");
    println!("  sudo pulsarkey profile lockdown");
    println!("{}", "==================================================".cyan());
}

/// Handles CLI profile subcommands.
pub fn handle_profile_cli(action: Option<String>) {
    match action.as_deref() {
        None | Some("status") => {
            print_profile_status();
        }
        Some(name) => {
            if let Some(target) = SecurityProfile::from_str(name) {
                // Changing PAM requires root
                crate::ensure_root(&format!("profile {}", target.as_str()));
                match apply_profile(target) {
                    Ok(_) => {
                        println!(
                            "\n{} Successfully switched security profile to: {}",
                            "✅".green(),
                            target.display_name().bold().green()
                        );
                        println!("   {}", target.description().dimmed());
                    }
                    Err(e) => {
                        eprintln!("\n{} Failed to apply profile: {}", "❌ Error:".red(), e);
                    }
                }
            } else {
                eprintln!(
                    "Unknown profile: '{}'. Available profiles: convenience, fortress, lockdown",
                    name
                );
            }
        }
    }
}
