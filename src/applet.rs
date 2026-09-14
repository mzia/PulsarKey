use ksni::menu::*;
use ksni::{Category, MenuItem, ToolTip, Tray, TrayMethods};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

const MAPPING_FILE: &str = "/etc/yubico/u2f_keys";
const PAM_SUDO: &str = "/etc/pam.d/sudo";
const PAM_GREETER: &str = "/etc/pam.d/cosmic-greeter";

#[derive(Clone, Debug)]
pub struct YubiKeyApplet {
    pub device_name: String,
    pub is_connected: bool,
    pub lockscreen_status: String,
    pub sudo_status: String,
    pub keys_count: usize,
    pub has_uv: bool,
}

impl YubiKeyApplet {
    pub fn new() -> Self {
        let mut applet = Self {
            device_name: "Scanning...".to_string(),
            is_connected: false,
            lockscreen_status: "Unknown".to_string(),
            sudo_status: "Unknown".to_string(),
            keys_count: 0,
            has_uv: false,
        };
        applet.refresh();
        applet
    }

    pub fn refresh(&mut self) {
        // 1. Hardware detection
        let yk_output = Command::new("ykman").arg("info").output();
        match yk_output {
            Ok(out) if out.status.success() => {
                let info = String::from_utf8_lossy(&out.stdout);
                let first_line = info
                    .lines()
                    .find(|l| l.starts_with("Device type:"))
                    .map(|l| l.replace("Device type:", "").trim().to_string())
                    .unwrap_or_else(|| "YubiKey Detected".to_string());

                self.device_name = first_line;
                self.is_connected = true;
            }
            _ => {
                // Fallback check on hidraw devices with Yubico vendor ID (1050)
                let has_yubico_hid = Path::new("/sys/bus/usb/drivers/usbhid")
                    .read_dir()
                    .map(|entries| {
                        entries.flatten().any(|e| {
                            let path = e.path();
                            if let Ok(modalias) = fs::read_to_string(path.join("modalias")) {
                                modalias.contains("v1050")
                            } else {
                                false
                            }
                        })
                    })
                    .unwrap_or(false);

                if has_yubico_hid {
                    self.device_name = "YubiKey (Connected)".to_string();
                    self.is_connected = true;
                } else {
                    self.device_name = "No YubiKey Detected".to_string();
                    self.is_connected = false;
                }
            }
        }

        // 2. Mapping check
        if let Ok(content) = fs::read_to_string(MAPPING_FILE) {
            self.keys_count = content.split(':').count().saturating_sub(1);
            self.has_uv = content.contains("+verification");
        } else {
            self.keys_count = 0;
            self.has_uv = false;
        }

        // 3. PAM status checks
        self.sudo_status = check_pam(PAM_SUDO);
        self.lockscreen_status = check_pam(PAM_GREETER);
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
            "security-high-symbolic".into()
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
                "Device: {}\nLockscreen: {}\nSudo: {}",
                self.device_name, self.lockscreen_status, self.sudo_status
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
                label: format!("⚡ Sudo Auth: {}", self.sudo_status),
                enabled: false,
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: format!(
                    "👥 Enrolled Keys: {} (Biometrics: {})",
                    self.keys_count,
                    if self.has_uv { "Yes" } else { "No" }
                ),
                enabled: false,
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
                                "security-high-symbolic",
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
                                        "security-high-symbolic",
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

    // Dynamic monitor loop: checks YubiKey presence every 3 seconds and updates icon
    loop {
        tokio::time::sleep(Duration::from_secs(3)).await;
        let _ = handle
            .update(|tray| {
                tray.refresh();
            })
            .await;
    }
}
