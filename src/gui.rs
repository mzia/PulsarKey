use colored::*;

#[deprecated(
    since = "1.5.0",
    note = "The graphical Settings control panel has been permanently deprecated. Please use the CLI commands or COSMIC panel applet."
)]
pub fn print_deprecation_notice() {
    eprintln!("{}", "⚠️ PulsarKey Settings GUI has been permanently deprecated.".yellow().bold());
    eprintln!("All configuration and security features are directly accessible via the CLI and panel applet:\n");
    eprintln!("  pulsarkey status       - Security & PAM status dashboard");
    eprintln!("  pulsarkey bio          - Biometric fingerprint & PIN manager");
    eprintln!("  pulsarkey profile      - Strictness profiles (convenience, fortress, lockdown)");
    eprintln!("  pulsarkey backup       - Backup key pairing & recovery");
    eprintln!("  pulsarkey rescue       - Emergency paper keys & offline runbook");
    eprintln!("  pulsarkey ssh-setup    - Hardware SSH & Git commit signing");
    eprintln!("  pulsarkey autolock     - Presence Sentinel auto-lock configuration\n");
}

#[allow(deprecated)]
pub fn run_gui() -> Result<(), Box<dyn std::error::Error>> {
    print_deprecation_notice();
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("notify-send")
            .args([
                "-u", "normal",
                "-i", "security-high-symbolic",
                "PulsarKey Settings Deprecated",
                "The graphical Settings app has been permanently deprecated.\nPlease use 'pulsarkey' or the panel applet.",
            ])
            .status();
    }
    Ok(())
}
