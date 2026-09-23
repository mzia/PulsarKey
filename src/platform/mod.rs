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
    sudo: "/etc/pam.d/sudo",
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
        let _ = Command::new("cosmic-term")
            .args(["-e", "bash", "-c", cmd])
            .spawn();
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
}
