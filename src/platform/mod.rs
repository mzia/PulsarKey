use std::process::Command;

#[derive(Debug, Clone)]
pub struct PamPaths {
    pub sudo: &'static str,
    pub greeter_or_screensaver: &'static str,
    pub elevation_service: &'static str,
    pub greeter_label: &'static str,
    pub elevation_label: &'static str,
}

#[cfg(target_os = "macos")]
pub const PAM_PATHS: PamPaths = PamPaths {
    sudo: "/etc/pam.d/sudo_local",
    greeter_or_screensaver: "/etc/pam.d/screensaver",
    elevation_service: "/etc/pam.d/authorization",
    greeter_label: "macOS Screensaver Lock",
    elevation_label: "macOS Authorization (SecurityAgent)",
};

#[cfg(not(target_os = "macos"))]
pub const PAM_PATHS: PamPaths = PamPaths {
    sudo: "/etc/pam.d/sudo",
    greeter_or_screensaver: "/etc/pam.d/cosmic-greeter",
    elevation_service: "/etc/pam.d/polkit-1",
    greeter_label: "COSMIC Greeter Lockscreen",
    elevation_label: "Polkit GUI (pkexec)",
};

pub fn get_os_display_name() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "macOS"
    }
    #[cfg(not(target_os = "macos"))]
    {
        "Pop!_OS COSMIC"
    }
}

pub fn lock_session() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        // 1. Try AppleScript to tell System Events to sleep/lock
        let res = Command::new("osascript")
            .args(["-e", "tell application \"System Events\" to keystroke \"q\" using {control down, command down}"])
            .status();

        if let Ok(s) = res {
            if s.success() {
                return Ok(());
            }
        }

        // 2. Fallback: pmset displaysleepnow
        let _ = Command::new("pmset").arg("displaysleepnow").status();
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        let res = Command::new("loginctl").arg("lock-session").status();
        match res {
            Ok(s) if s.success() => Ok(()),
            Ok(_) => Err("loginctl lock-session returned non-zero exit code".to_string()),
            Err(e) => Err(format!("Failed to invoke loginctl: {}", e)),
        }
    }
}

pub fn send_desktop_notification(title: &str, body: &str, is_critical: bool) {
    #[cfg(target_os = "macos")]
    {
        let _ = is_critical;
        let script = format!(
            "display notification \"{}\" with title \"{}\"",
            body.replace('\"', "\\\""),
            title.replace('\"', "\\\"")
        );
        let _ = Command::new("osascript").args(["-e", &script]).status();
    }

    #[cfg(not(target_os = "macos"))]
    {
        let urgency = if is_critical { "critical" } else { "normal" };
        let _ = Command::new("notify-send")
            .args([
                "-u",
                urgency,
                "-i",
                "auth-fingerprint-symbolic",
                title,
                body,
            ])
            .status();
    }
}

pub fn launch_in_terminal(cmd: &str) {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "tell application \"Terminal\" to do script \"{}\"",
            cmd.replace('\"', "\\\"")
        );
        let _ = Command::new("osascript").args(["-e", &script]).spawn();
    }

    #[cfg(not(target_os = "macos"))]
    {
        let terminals = ["cosmic-term", "gnome-terminal", "x-terminal-emulator", "konsole", "alacritty", "kitty"];
        let mut launched = false;
        for term in &terminals {
            let res = match *term {
                "gnome-terminal" => Command::new(term).args(["--", "bash", "-c", cmd]).spawn(),
                _ => Command::new(term).args(["-e", "bash", "-c", cmd]).spawn(),
            };
            if res.is_ok() {
                launched = true;
                break;
            }
        }
        if !launched {
            eprintln!("Warning: No supported terminal emulator found to run '{}'.", cmd);
        }
    }
}

pub fn check_security_key_usb_connected() -> (bool, String) {
    let t = crate::hardware::detect_fido_device();
    (t.is_connected, t.product_name)
}

pub fn check_yubikey_usb_connected() -> (bool, String) {
    check_security_key_usb_connected()
}

pub fn run_elevated(cmd: &str) -> std::io::Result<std::process::ExitStatus> {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "do shell script \"{}\" with administrator privileges",
            cmd.replace('\"', "\\\"")
        );
        Command::new("osascript").args(["-e", &script]).status()
    }

    #[cfg(not(target_os = "macos"))]
    {
        Command::new("pkexec").args(["bash", "-c", cmd]).status()
    }
}

pub fn is_touch_id_pam_enabled() -> bool {
    #[cfg(target_os = "macos")]
    {
        let p = std::path::Path::new("/etc/pam.d/sudo_local");
        if let Ok(content) = std::fs::read_to_string(p) {
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.starts_with('#') && trimmed.contains("pam_tid.so") {
                    return true;
                }
            }
        }
        false
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

pub fn configure_touch_id_pam(enable: bool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let path = "/etc/pam.d/sudo_local";
        let mut lines: Vec<String> = if std::path::Path::new(path).exists() {
            std::fs::read_to_string(path)
                .map_err(|e| format!("Failed to read {}: {}", path, e))?
                .lines()
                .map(|s| s.to_string())
                .collect()
        } else {
            vec![
                "# sudo_local: managed by PulsarKey (survives macOS updates)".to_string(),
            ]
        };

        // Remove any existing pam_tid.so line
        lines.retain(|l| !l.contains("pam_tid.so"));

        if enable {
            let tid_line = "auth       sufficient     pam_tid.so";
            if lines.is_empty() {
                lines.push(tid_line.to_string());
            } else {
                lines.insert(0, tid_line.to_string());
            }
        }

        std::fs::write(path, lines.join("\n") + "\n")
            .map_err(|e| format!("Failed to write {}: {}", path, e))?;

        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = enable;
        Err("Touch ID PAM is only supported on macOS".to_string())
    }
}

pub fn get_user_home() -> std::path::PathBuf {
    if let Ok(sudo_user) = std::env::var("SUDO_USER") {
        #[cfg(target_os = "macos")]
        { std::path::PathBuf::from(format!("/Users/{}", sudo_user)) }
        #[cfg(not(target_os = "macos"))]
        { std::path::PathBuf::from(format!("/home/{}", sudo_user)) }
    } else if let Ok(home) = std::env::var("HOME") {
        std::path::PathBuf::from(home)
    } else {
        let user = std::env::var("USER").unwrap_or_else(|_| "mzia".to_string());
        #[cfg(target_os = "macos")]
        { std::path::PathBuf::from(format!("/Users/{}", user)) }
        #[cfg(not(target_os = "macos"))]
        { std::path::PathBuf::from(format!("/home/{}", user)) }
    }
}

pub fn get_linux_systemd_unit_path() -> std::path::PathBuf {
    get_user_home().join(".config/systemd/user/pulsarkey-applet.service")
}

pub fn get_linux_autostart_desktop_path() -> std::path::PathBuf {
    get_user_home().join(".config/autostart/io.github.mzia.PulsarKey.Applet.desktop")
}

pub fn get_launchd_plist_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    std::path::PathBuf::from(home)
        .join("Library")
        .join("LaunchAgents")
        .join("io.github.mzia.pulsarkey.plist")
}

pub fn install_daemon_service() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let plist_path = get_launchd_plist_path();
        if let Some(parent) = plist_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let exe_path = std::env::current_exe()
            .unwrap_or_else(|_| std::path::PathBuf::from("/usr/local/bin/pulsarkey"));
        let exe_str = exe_path.to_string_lossy();

        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let log_dir = format!("{}/.config/pulsarkey", home);
        let _ = std::fs::create_dir_all(&log_dir);

        let plist_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>io.github.mzia.pulsarkey</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>applet</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{}/daemon.log</string>
    <key>StandardErrorPath</key>
    <string>{}/daemon.err</string>
</dict>
</plist>
"#,
            exe_str, log_dir, log_dir
        );

        std::fs::write(&plist_path, plist_content)
            .map_err(|e| format!("Failed to write LaunchAgent plist: {}", e))?;

        let status = Command::new("launchctl")
            .args(["load", "-w", &plist_path.to_string_lossy()])
            .status()
            .map_err(|e| format!("Failed to invoke launchctl load: {}", e))?;

        if status.success() {
            Ok(format!("Installed and loaded launchd agent at {}", plist_path.display()))
        } else {
            Err("launchctl load returned a non-zero exit code".to_string())
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let user_home = get_user_home();
        let autostart_dir = user_home.join(".config/autostart");
        let apps_dir = user_home.join(".local/share/applications");
        let systemd_dir = user_home.join(".config/systemd/user");

        let _ = std::fs::create_dir_all(&autostart_dir);
        let _ = std::fs::create_dir_all(&apps_dir);
        let _ = std::fs::create_dir_all(&systemd_dir);

        let desktop_content = "[Desktop Entry]\n\
Name=PulsarKey\n\
Comment=COSMIC Panel Status Applet for FIDO2 & Biometric Security\n\
Exec=/usr/bin/pulsarkey applet\n\
Icon=auth-fingerprint-symbolic\n\
Terminal=false\n\
Type=Application\n\
Categories=COSMIC;Utility;Security;\n\
Keywords=pulsar;pulsarkey;fido2;yubikey;nitrokey;solo;fingerprint;biometric;security;u2f;panel;applet;\n\
X-CosmicApplet=true\n\
X-CosmicShrinkable=true\n\
X-CosmicHoverPopup=Auto\n\
X-HostWaylandDisplay=true\n\
X-GNOME-Autostart-enabled=true\n\
NoDisplay=true\n";

        let autostart_file = autostart_dir.join("io.github.mzia.PulsarKey.Applet.desktop");
        let app_file = apps_dir.join("io.github.mzia.PulsarKey.Applet.desktop");
        let _ = std::fs::write(&autostart_file, desktop_content);
        let _ = std::fs::write(&app_file, desktop_content);

        let exe_path = std::env::current_exe()
            .unwrap_or_else(|_| std::path::PathBuf::from("/usr/bin/pulsarkey"));
        let exe_str = exe_path.to_string_lossy();

        let service_content = format!(
"[Unit]\n\
Description=PulsarKey FIDO2 Security Applet & Presence Sentinel\n\
PartOf=graphical-session.target\n\
After=graphical-session.target\n\
\n\
[Service]\n\
Type=simple\n\
ExecStart={} applet\n\
Restart=on-failure\n\
RestartSec=3\n\
\n\
[Install]\n\
WantedBy=graphical-session.target\n",
            exe_str
        );

        let service_file = systemd_dir.join("pulsarkey-applet.service");
        std::fs::write(&service_file, service_content)
            .map_err(|e| format!("Failed to write systemd service unit: {}", e))?;

        let _ = Command::new("systemctl").args(["--user", "daemon-reload"]).status();
        let status = Command::new("systemctl")
            .args(["--user", "enable", "--now", "pulsarkey-applet.service"])
            .status();

        if let Ok(s) = status {
            if s.success() {
                return Ok(format!(
                    "Installed COSMIC autostart and activated systemd user service ({})",
                    service_file.display()
                ));
            }
        }

        Ok(format!(
            "Installed COSMIC autostart and systemd service file at {}",
            service_file.display()
        ))
    }
}

pub fn uninstall_daemon_service() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let plist_path = get_launchd_plist_path();
        if plist_path.exists() {
            let _ = Command::new("launchctl")
                .args(["unload", &plist_path.to_string_lossy()])
                .status();
            let _ = std::fs::remove_file(&plist_path);
            Ok("Uninstalled and unloaded launchd agent".to_string())
        } else {
            Ok("No launchd agent plist was found to remove".to_string())
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let user_home = get_user_home();
        let service_file = user_home.join(".config/systemd/user/pulsarkey-applet.service");
        let autostart_file = user_home.join(".config/autostart/io.github.mzia.PulsarKey.Applet.desktop");
        let app_file = user_home.join(".local/share/applications/io.github.mzia.PulsarKey.Applet.desktop");

        let _ = Command::new("systemctl")
            .args(["--user", "disable", "--now", "pulsarkey-applet.service"])
            .status();

        if service_file.exists() {
            let _ = std::fs::remove_file(&service_file);
        }
        let _ = Command::new("systemctl").args(["--user", "daemon-reload"]).status();

        if autostart_file.exists() {
            let _ = std::fs::remove_file(&autostart_file);
        }
        if app_file.exists() {
            let _ = std::fs::remove_file(&app_file);
        }

        Ok("Disabled systemd user service and removed COSMIC autostart desktop entries".to_string())
    }
}

pub fn get_daemon_service_status() -> (bool, String) {
    #[cfg(target_os = "macos")]
    {
        let plist_path = get_launchd_plist_path();
        let installed = plist_path.exists();
        if let Ok(out) = Command::new("launchctl").arg("list").output() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.contains("io.github.mzia.pulsarkey") {
                return (true, "Active (launchd loaded & running)".to_string());
            }
        }
        if installed {
            (false, "Installed (launchd loaded but idle)".to_string())
        } else {
            (false, "Not Installed (Run 'pulsarkey daemon install')".to_string())
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        if let Ok(out) = Command::new("systemctl").args(["--user", "is-active", "pulsarkey-applet.service"]).output() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if s == "active" {
                return (true, "Active (systemd user unit running)".to_string());
            }
        }
        let service_file = get_linux_systemd_unit_path();
        let autostart_file = get_linux_autostart_desktop_path();
        if service_file.exists() || autostart_file.exists() {
            (false, "Installed (systemd unit present but inactive)".to_string())
        } else {
            (false, "Not Installed (Run 'pulsarkey daemon install')".to_string())
        }
    }
}

pub fn launch_gui_bundle_app() {
    #[cfg(target_os = "macos")]
    {
        let exe = std::env::current_exe().unwrap_or_default();
        let parent = exe.parent();
        let bar_path = parent.map(|p| p.join("PulsarKeyBar")).unwrap_or_default();

        if bar_path.exists() {
            let _ = Command::new(bar_path).spawn();
        } else {
            let _ = Command::new(&exe).arg("applet").spawn();
        }

        launch_in_terminal(&format!("\"{}\"", exe.to_string_lossy()));
    }
    #[cfg(not(target_os = "macos"))]
    {
        let exe = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("pulsarkey"));
        launch_in_terminal(&format!("\"{}\"", exe.to_string_lossy()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_os_display_name() {
        let name = get_os_display_name();
        assert!(!name.is_empty());
        #[cfg(target_os = "macos")]
        assert_eq!(name, "macOS");
        #[cfg(target_os = "linux")]
        assert_eq!(name, "Pop!_OS COSMIC");
    }

    #[test]
    fn test_pam_paths_non_empty() {
        assert!(!PAM_PATHS.sudo.is_empty());
        assert!(!PAM_PATHS.greeter_or_screensaver.is_empty());
        assert!(!PAM_PATHS.elevation_service.is_empty());
        assert!(!PAM_PATHS.greeter_label.is_empty());
        assert!(!PAM_PATHS.elevation_label.is_empty());
    }

    #[test]
    fn test_user_home_resolution() {
        let home = get_user_home();
        assert!(!home.to_string_lossy().is_empty());
        #[cfg(target_os = "macos")]
        {
            let plist = get_launchd_plist_path();
            assert!(plist.to_string_lossy().contains("io.github.mzia.pulsarkey.plist"));
        }
        #[cfg(target_os = "linux")]
        {
            let unit = get_linux_systemd_unit_path();
            assert!(unit.to_string_lossy().contains("pulsarkey-applet.service"));
            let desktop = get_linux_autostart_desktop_path();
            assert!(desktop.to_string_lossy().contains("io.github.mzia.PulsarKey.Applet.desktop"));
        }
    }
}
