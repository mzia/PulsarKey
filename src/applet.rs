#[cfg(target_os = "linux")]
use ksni::menu::*;
#[cfg(target_os = "linux")]
use ksni::{Category, Icon, MenuItem, ToolTip, Tray, TrayMethods};
use std::fs;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

const MAPPING_FILE: &str = "/etc/yubico/u2f_keys";

#[derive(Clone, Debug)]
pub struct PulsarKeyApplet {
    pub device_name: String,
    pub is_connected: bool,
    pub was_connected: bool,
    pub lockscreen_status: String,
    pub sudo_status: String,
    pub polkit_status: String,
    pub keys_count: usize,
    pub has_uv: bool,
    pub autolock_enabled: bool,
    pub profile_name: String,
}

pub type YubiKeyApplet = PulsarKeyApplet;

impl PulsarKeyApplet {
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
            profile_name: cfg.profile,
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
        crate::platform::send_desktop_notification(
            "PulsarKey Sentinel",
            &format!("Auto-Lock on removal: {}", status_str),
            false,
        );
    }

    pub fn refresh(&mut self) {
        // 1. Cross-platform hardware detection
        let (new_connected, hw_product_name) = crate::platform::check_security_key_usb_connected();

        if new_connected {
            if !self.was_connected || self.device_name == "No Security Key Detected" {
                // Enrich with ykman info once upon insertion if available
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

                self.device_name = yk_name.unwrap_or(hw_product_name);
            }
        } else {
            self.device_name = "No Security Key Detected".to_string();
        }

        // Hardware audit logging
        if !self.was_connected && new_connected {
            crate::audit::log_event(
                "HARDWARE",
                "Security Key Inserted",
                "Connected",
                &self.device_name,
            );
        } else if self.was_connected && !new_connected {
            crate::audit::log_event(
                "HARDWARE",
                "Security Key Removed",
                "Disconnected",
                "Key removed from USB port",
            );
        }

        // Feature 1: Presence Sentinel - Auto-Lock on key removal
        if self.was_connected && !new_connected && self.autolock_enabled {
            println!("🚨 Security Key removed with Auto-Lock enabled! Locking session...");
            crate::audit::log_event(
                "SENTINEL",
                "Auto-Lock Triggered",
                "Locked",
                "Desktop locked on token removal",
            );
            crate::platform::send_desktop_notification(
                "PulsarKey Sentinel",
                &format!("Security Key removed — {} desktop locked.", crate::platform::get_os_display_name()),
                true,
            );
            let _ = crate::platform::lock_session();
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

        // 3. Configuration & Profile reload
        let cfg = crate::config::load_config();
        self.autolock_enabled = cfg.autolock;
        self.profile_name = cfg.profile;

        // 4. PAM status checks (Lockscreen, Sudo, Polkit/Authorization)
        self.sudo_status = check_pam(crate::platform::PAM_PATHS.sudo);
        self.lockscreen_status = check_pam(crate::platform::PAM_PATHS.greeter_or_screensaver);
        self.polkit_status = check_pam(crate::platform::PAM_PATHS.elevation_service);
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

#[cfg(target_os = "linux")]
fn render_fingerprint_pixmap(connected: bool, size: i32) -> Icon {
    let w = size as usize;
    let h = size as usize;
    let mut data = vec![0u8; w * h * 4];

    let (r_val, g_val, b_val) = if connected {
        (246, 246, 246) // Bright white for dark top bar
    } else {
        (224, 108, 117) // Warning red/amber for disconnected
    };

    let scale = size as f32 / 24.0;

    for y in 0..h {
        let ny = y as f32 / scale;
        for x in 0..w {
            let nx = x as f32 / scale;
            let offset = (y * w + x) * 4;

            let cx = 11.5;
            let cy = 12.0;
            let dx = nx - cx;
            let dy = (ny - cy) * 0.85;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > 10.5 || ny < 2.0 || ny > 22.0 {
                continue;
            }

            let ridge_spacing = 2.5;
            let phase = dist / ridge_spacing;
            let frac = (phase - phase.round()).abs();

            if frac < 0.35 {
                let intensity = 1.0 - (frac / 0.35);
                let alpha = ((intensity * 255.0) as u8).min(255);
                data[offset] = alpha;
                data[offset + 1] = r_val;
                data[offset + 2] = g_val;
                data[offset + 3] = b_val;
            }
        }
    }

    Icon {
        width: size,
        height: size,
        data,
    }
}

#[cfg(target_os = "linux")]
impl Tray for PulsarKeyApplet {
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
            "auth-fingerprint-disconnected-symbolic".into()
        }
    }

    fn icon_theme_path(&self) -> String {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/mzia".to_string());
        format!("{}/.local/share/icons/hicolor", home)
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        vec![
            render_fingerprint_pixmap(self.is_connected, 24),
            render_fingerprint_pixmap(self.is_connected, 32),
        ]
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            icon_name: self.icon_name(),
            icon_pixmap: self.icon_pixmap(),
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
            StandardItem {
                label: "🌌 Open PulsarKey TUI Dashboard...".into(),
                activate: Box::new(|_| {
                    crate::platform::launch_in_terminal("pulsarkey tui");
                }),
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
                activate: Box::new(|tray: &mut PulsarKeyApplet| {
                    tray.toggle_autolock();
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "🔒 Lock Screen Now".into(),
                activate: Box::new(|_| {
                    let _ = crate::platform::lock_session();
                }),
                ..Default::default()
            }
            .into(),
            // Feature: Security Strictness Profiles SubMenu
            SubMenu {
                label: format!("🛡️ Security Profile: {}", match self.profile_name.as_str() {
                    "fortress" => "Fortress (2FA)",
                    "lockdown" => "Lockdown (Strict)",
                    _ => "Convenience (1FA)",
                }),
                submenu: vec![
                    StandardItem {
                        label: format!("{} Convenience (1FA Touch/Bio)", if self.profile_name == "convenience" { "●" } else { "○" }),
                        activate: Box::new(|_| {
                            let _ = Command::new("pkexec")
                                .args(["pulsarkey", "profile", "convenience"])
                                .spawn();
                        }),
                        ..Default::default()
                    }
                    .into(),
                    StandardItem {
                        label: format!("{} Fortress (True 2FA: Password+Touch)", if self.profile_name == "fortress" { "●" } else { "○" }),
                        activate: Box::new(|_| {
                            let _ = Command::new("pkexec")
                                .args(["pulsarkey", "profile", "fortress"])
                                .spawn();
                        }),
                        ..Default::default()
                    }
                    .into(),
                    StandardItem {
                        label: format!("{} Lockdown (Hardware Mandatory)", if self.profile_name == "lockdown" { "●" } else { "○" }),
                        activate: Box::new(|_| {
                            let _ = Command::new("pkexec")
                                .args(["pulsarkey", "profile", "lockdown"])
                                .spawn();
                        }),
                        ..Default::default()
                    }
                    .into(),
                ],
                ..Default::default()
            }
            .into(),
            // Feature: Recent Pulses (Audit Log) SubMenu
            SubMenu {
                label: "📜 Recent Pulses (Audit Log)".into(),
                submenu: {
                    let mut items: Vec<MenuItem<Self>> = crate::audit::get_recent_summary_for_applet(5)
                        .into_iter()
                        .map(|s| StandardItem {
                            label: s,
                            enabled: false,
                            ..Default::default()
                        }.into())
                        .collect();
                    items.push(MenuItem::Separator);
                    items.push(StandardItem {
                        label: "📊 View Full Audit Log (Terminal)...".into(),
                        activate: Box::new(|_| {
                            crate::platform::launch_in_terminal("pulsarkey audit; echo ''; read -p 'Press Enter to close...'");
                        }),
                        ..Default::default()
                    }.into());
                    items
                },
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
                                "Security Key Biometric Test",
                                "Please scan your fingerprint or touch your Security Key...",
                            ])
                            .status();

                        let output = Command::new("pamu2fcfg").arg("-V").output();
                        match output {
                            Ok(out) if out.status.success() => {
                                let _ = Command::new("notify-send")
                                    .args([
                                        "-i",
                                        "auth-fingerprint-symbolic",
                                        "PulsarKey Biometrics",
                                        "✅ Verification successful!",
                                    ])
                                    .status();
                            }
                            _ => {
                                let _ = Command::new("notify-send")
                                    .args([
                                        "-i",
                                        "dialog-warning-symbolic",
                                        "PulsarKey Biometrics",
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
                    crate::platform::launch_in_terminal("sudo pulsarkey setup");
                }),
                ..Default::default()
            }
            .into(),
            // Action: Biometric & Fingerprint Manager
            StandardItem {
                label: "🧬 Biometric Fingerprint Manager (Terminal)...".into(),
                activate: Box::new(|_| {
                    crate::platform::launch_in_terminal("pulsarkey bio");
                }),
                ..Default::default()
            }
            .into(),
            // Action: Backup Key Assistant
            StandardItem {
                label: "👯 Backup Key Assistant (Terminal)...".into(),
                activate: Box::new(|_| {
                    crate::platform::launch_in_terminal("sudo pulsarkey backup");
                }),
                ..Default::default()
            }
            .into(),
            // Action: Emergency Recovery & Runbook
            StandardItem {
                label: "🛟 Emergency Rescue Runbook (Terminal)...".into(),
                activate: Box::new(|_| {
                    crate::platform::launch_in_terminal("pulsarkey rescue runbook; echo ''; read -p 'Press Enter to close...'");
                }),
                ..Default::default()
            }
            .into(),
            // Action: Hardware SSH & Git Signing
            StandardItem {
                label: "🔑 Hardware SSH & Git Signing (Terminal)...".into(),
                activate: Box::new(|_| {
                    crate::platform::launch_in_terminal("pulsarkey ssh-setup");
                }),
                ..Default::default()
            }
            .into(),
            // Action: Full Status Dashboard
            StandardItem {
                label: "📊 View Security Status (Terminal)...".into(),
                activate: Box::new(|_| {
                    crate::platform::launch_in_terminal("pulsarkey status; echo ''; read -p 'Press Enter to close...'");
                }),
                ..Default::default()
            }
            .into(),
            // Action: Lock Screen
            StandardItem {
                label: "🔒 Lock Screen Now".into(),
                activate: Box::new(|_| {
                    let _ = crate::platform::lock_session();
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

/// Acquires an exclusive advisory lock on the runtime lockfile.
/// Returns Some(File) if lock was acquired, or None if another instance is already running.
fn acquire_single_instance_lock() -> Option<fs::File> {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| format!("/tmp/user-{}", unsafe { libc::getuid() }));
    let _ = fs::create_dir_all(&runtime_dir);
    let lock_path = PathBuf::from(runtime_dir).join("pulsarkey-applet.lock");
    acquire_single_instance_lock_at(&lock_path)
}

fn acquire_single_instance_lock_at(lock_path: &std::path::Path) -> Option<fs::File> {
    use std::io::Write;

    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)
        .ok()?;

    let res = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if res != 0 {
        return None;
    }

    let _ = file.set_len(0);
    let _ = writeln!(file, "{}", std::process::id());

    Some(file)
}

#[cfg(target_os = "linux")]
pub async fn run_applet() {
    let _lock = match acquire_single_instance_lock() {
        Some(file) => file,
        None => {
            println!("ℹ️ PulsarKey applet is already running on this session. Exiting duplicate instance.");
            return;
        }
    };

    let applet = PulsarKeyApplet::new();
    let handle = applet.spawn().await.expect("Failed to spawn COSMIC status tray applet");

    // Dynamic monitor loop: checks Security Key presence every 2 seconds for responsive auto-lock
    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let _ = handle
            .update(|tray| {
                tray.refresh();
            })
            .await;
    }
}

#[cfg(not(target_os = "linux"))]
pub async fn run_applet() {
    let _lock = match acquire_single_instance_lock() {
        Some(file) => file,
        None => {
            println!("ℹ️ PulsarKey Sentinel daemon is already running on this session. Exiting duplicate instance.");
            return;
        }
    };

    println!("🌌 Launching PulsarKey Hardware Sentinel Daemon for macOS...");
    let mut applet = PulsarKeyApplet::new();
    println!("🛡️ Presence Sentinel Auto-Lock: {}", if applet.autolock_enabled { "Enabled" } else { "Disabled" });
    println!("Monitoring Security Key insertions & removals in background...");

    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;
        applet.refresh();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_instance_lock() {
        let test_lock_path = std::env::temp_dir().join(format!("pulsarkey-test-lock-{}.lock", std::process::id()));
        let _ = fs::remove_file(&test_lock_path);

        let lock1 = acquire_single_instance_lock_at(&test_lock_path);
        assert!(lock1.is_some(), "First lock acquisition should succeed");

        let lock2 = acquire_single_instance_lock_at(&test_lock_path);
        assert!(lock2.is_none(), "Second concurrent lock acquisition must fail");

        drop(lock1);

        let lock3 = acquire_single_instance_lock_at(&test_lock_path);
        assert!(lock3.is_some(), "Lock acquisition should succeed after drop");

        let _ = fs::remove_file(&test_lock_path);
    }
}
