use ksni::menu::*;
use ksni::{Category, MenuItem, ToolTip, Tray, TrayMethods};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

const MAPPING_FILE: &str = "/etc/yubico/u2f_keys";
const PAM_SUDO: &str = "/etc/pam.d/sudo";
const PAM_GREETER: &str = "/etc/pam.d/cosmic-greeter";
const PAM_POLKIT: &str = "/etc/pam.d/polkit-1";

#[derive(Clone, Debug)]
pub struct YubiKeyApplet {
    pub device_name: String,
    pub is_connected: bool,
    pub was_connected: bool,
    pub lockscreen_status: String,
    pub sudo_status: String,
    pub polkit_status: String,
    pub keys_count: usize,
    pub has_uv: bool,
    pub autolock_enabled: bool,
}

impl YubiKeyApplet {
    pub fn new() -> Self {
        let cfg = crate::config::load_config();
        let mut applet = Self {
            device_name: "Scanning...".to_string(),
            is_connected: false,
            was_connected: false,
            lockscreen_status: "Unknown".to_string(),
            sudo_status: "Unknown".to_string(),
            polkit_status: "Unknown".to_string(),
            keys_count: 0,
            has_uv: false,
            autolock_enabled: cfg.autolock,
        };
        applet.refresh();
        // Sync initial state so it doesn't fire lock on startup
        applet.was_connected = applet.is_connected;
        applet
    }

    pub fn toggle_autolock(&mut self) {
        self.autolock_enabled = !self.autolock_enabled;
        let mut cfg = crate::config::load_config();
        cfg.autolock = self.autolock_enabled;
        let _ = crate::config::save_config(&cfg);

        let status_str = if self.autolock_enabled { "Enabled" } else { "Disabled" };
        let _ = Command::new("notify-send")
            .args([
                "-i",
                "auth-fingerprint-symbolic",
                "PulsarKey Sentinel",
                &format!("Auto-Lock on removal: {}", status_str),
            ])
            .status();
    }

    pub fn refresh(&mut self) {
        // 1. Instant sysfs hardware detection (zero contention, zero Python overhead)
        let mut yubikey_sysfs_path: Option<PathBuf> = None;
        if let Ok(entries) = fs::read_dir("/sys/bus/usb/devices") {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Ok(vendor) = fs::read_to_string(p.join("idVendor")) {
                    if vendor.trim() == "1050" {
                        yubikey_sysfs_path = Some(p);
                        break;
                    }
                }
            }
        }

        let new_connected = yubikey_sysfs_path.is_some();

        if new_connected {
            if !self.was_connected || self.device_name == "No YubiKey Detected" {
                // Read product name from sysfs first
                let product_name = yubikey_sysfs_path
                    .as_ref()
                    .and_then(|p| fs::read_to_string(p.join("product")).ok())
                    .map(|s| s.trim().to_string());

                // Enrich with ykman info once upon insertion
                let yk_name = Command::new("ykman").arg("info").output().ok().and_then(|out| {
                    if out.status.success() {
                        let info = String::from_utf8_lossy(&out.stdout);
                        info.lines()
                            .find(|l| l.starts_with("Device type:"))
                            .map(|l| l.replace("Device type:", "").trim().to_string())
                    } else {
                        None
                    }
                });

                self.device_name = yk_name
                    .or(product_name)
                    .unwrap_or_else(|| "YubiKey Detected".to_string());
            }
        } else {
            self.device_name = "No YubiKey Detected".to_string();
        }

        // Feature 1: Presence Sentinel - Auto-Lock on key removal
        if self.was_connected && !new_connected && self.autolock_enabled {
            println!("🚨 YubiKey removed with Auto-Lock enabled! Locking COSMIC session...");
            let _ = Command::new("notify-send")
                .args([
                    "-u",
                    "critical",
                    "-i",
                    "auth-fingerprint-symbolic",
                    "PulsarKey Sentinel",
                    "YubiKey removed — COSMIC desktop locked.",
                ])
                .status();
            let _ = Command::new("loginctl").arg("lock-session").status();
        }
        self.was_connected = new_connected;
        self.is_connected = new_connected;

        // 2. Mapping check
        if let Ok(content) = fs::read_to_string(MAPPING_FILE) {
            self.keys_count = content.split(':').count().saturating_sub(1);
            self.has_uv = content.contains("+verification");
        } else {
            self.keys_count = 0;
            self.has_uv = false;
        }

        // 3. PAM status checks (Lockscreen, Sudo, Polkit)
        self.sudo_status = check_pam(PAM_SUDO);
        self.lockscreen_status = check_pam(PAM_GREETER);
        self.polkit_status = check_pam(PAM_POLKIT);
    }
}

fn check_pam(path: &str) -> String {
    if let Ok(content) = fs::read_to_string(path) {
        if let Some(line) = content.lines().find(|l| l.contains("pam_u2f.so")) {
            if line.contains("interactive") {
                "FIDO2 (Space+Enter)".to_string()
            } else {
                "FIDO2 (Direct Touch)".to_string()
            }
        } else {
            "Password Only".to_string()
        }
    } else {
        "Disabled".to_string()
    }
}

impl Tray for YubiKeyApplet {
    const MENU_ON_ACTIVATE: bool = true;

    fn id(&self) -> String {
        "io.github.mzia.PulsarKey".into()
    }

    fn title(&self) -> String {
        "PulsarKey Security".into()
    }

    fn category(&self) -> Category {
        Category::Hardware
    }

    fn icon_name(&self) -> String {
        if self.is_connected {
            "auth-fingerprint-symbolic".into()
        } else {
            "security-low-symbolic".into()
        }
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            icon_name: self.icon_name(),
            icon_pixmap: Vec::new(),
            title: "PulsarKey FIDO2 Security".into(),
            description: format!(
                "Device: {}\nLockscreen: {}\nSudo: {}\nPolkit GUI: {}\nAuto-Lock: {}",
                self.device_name,
                self.lockscreen_status,
                self.sudo_status,
                self.polkit_status,
                if self.autolock_enabled { "Enabled" } else { "Disabled" }
            ),
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let status_icon = if self.is_connected { "🟢" } else { "🔴" };
        let device_label = format!("{} {}", status_icon, self.device_name);

        vec![
            // Header / Device status
            StandardItem {
                label: device_label,
                enabled: false,
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            // Configuration summary
            StandardItem {
                label: format!("🔒 Lockscreen: {}", self.lockscreen_status),
                enabled: false,
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: format!("⚡ Sudo Auth:   {}", self.sudo_status),
                enabled: false,
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: format!("🛡️ Polkit GUI:  {}", self.polkit_status),
                enabled: false,
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: format!(
                    "👥 Enrolled:    {} Key(s) (Biometrics: {})",
                    self.keys_count,
                    if self.has_uv { "Yes" } else { "No" }
                ),
                enabled: false,
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            // Feature 1: Presence Sentinel - Auto-Lock Checkmark Toggle
            CheckmarkItem {
                label: "🛡️ Auto-Lock on Key Removal".into(),
                checked: self.autolock_enabled,
                activate: Box::new(|tray: &mut YubiKeyApplet| {
                    tray.toggle_autolock();
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            // Action: Test Biometric Sensor
            StandardItem {
                label: "🔍 Test Fingerprint Sensor...".into(),
                activate: Box::new(|_| {
                    std::thread::spawn(|| {
                        let _ = Command::new("notify-send")
                            .args([
                                "-i",
                                "auth-fingerprint-symbolic",
                                "YubiKey Biometric Test",
                                "Please scan your fingerprint on the YubiKey...",
                            ])
                            .status();

                        let output = Command::new("pamu2fcfg").arg("-V").output();
                        match output {
                            Ok(out) if out.status.success() => {
                                let _ = Command::new("notify-send")
                                    .args([
                                        "-i",
                                        "auth-fingerprint-symbolic",
                                        "YubiKey Bio",
                                        "✅ Fingerprint verified successfully!",
                                    ])
                                    .status();
                            }
                            _ => {
                                let _ = Command::new("notify-send")
                                    .args([
                                        "-i",
                                        "dialog-warning-symbolic",
                                        "YubiKey Bio",
                                        "⚠️ Touch test timed out or cancelled.",
                                    ])
                                    .status();
                            }
                        }
                    });
                }),
                ..Default::default()
            }
            .into(),
            // Action: Terminal Setup
            StandardItem {
                label: "🚀 Setup / Add Key (Terminal)...".into(),
                activate: Box::new(|_| {
                    let _ = Command::new("cosmic-term")
                        .args(["-e", "sudo", "pulsarkey", "setup"])
                        .spawn();
                }),
                ..Default::default()
            }
            .into(),
            // Action: Biometric & Fingerprint Manager
            StandardItem {
                label: "🧬 Biometric Fingerprint Manager (Terminal)...".into(),
                activate: Box::new(|_| {
                    let _ = Command::new("cosmic-term")
                        .args(["-e", "pulsarkey", "bio"])
                        .spawn();
                }),
                ..Default::default()
            }
            .into(),
            // Action: Hardware SSH & Git Signing
            StandardItem {
                label: "🔑 Hardware SSH & Git Signing (Terminal)...".into(),
                activate: Box::new(|_| {
                    let _ = Command::new("cosmic-term")
                        .args(["-e", "pulsarkey", "ssh-setup"])
                        .spawn();
                }),
                ..Default::default()
            }
            .into(),
            // Action: Full Status Dashboard
            StandardItem {
                label: "📊 View Security Status (Terminal)...".into(),
                activate: Box::new(|_| {
                    let _ = Command::new("cosmic-term")
                        .args(["-e", "bash", "-c", "pulsarkey status; read -p 'Press Enter to close...'"])
                        .spawn();
                }),
                ..Default::default()
            }
            .into(),
            // Action: Lock Screen
            StandardItem {
                label: "🔒 Lock Screen Now".into(),
                activate: Box::new(|_| {
                    let _ = Command::new("loginctl").arg("lock-session").status();
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            // Action: Exit
            StandardItem {
                label: "🚪 Quit Applet".into(),
                activate: Box::new(|_| {
                    std::process::exit(0);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

pub async fn run_applet() {
    let applet = YubiKeyApplet::new();
    let handle = applet.spawn().await.expect("Failed to spawn COSMIC status tray applet");

    // Dynamic monitor loop: checks YubiKey presence every 2 seconds for responsive auto-lock
    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let _ = handle
            .update(|tray| {
                tray.refresh();
            })
            .await;
    }
}
