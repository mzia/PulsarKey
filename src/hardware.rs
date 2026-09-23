use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FidoVendor {
    Yubico,
    Nitrokey,
    SoloKeys,
    Feitian,
    GoogleTitan,
    KanoKey,
    Framework,
    Generic,
    Unknown,
}

impl FidoVendor {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Yubico => "Yubico",
            Self::Nitrokey => "Nitrokey",
            Self::SoloKeys => "SoloKeys",
            Self::Feitian => "Feitian",
            Self::GoogleTitan => "Google Titan",
            Self::KanoKey => "KanoKey",
            Self::Framework => "Framework",
            Self::Generic => "Generic FIDO2",
            Self::Unknown => "Unknown",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Yubico => "🛡️",
            Self::Nitrokey => "🔐",
            Self::SoloKeys => "🔑",
            Self::Feitian => "🧬",
            Self::GoogleTitan => "🌐",
            Self::KanoKey => "🗝️",
            Self::Framework => "💻",
            Self::Generic | Self::Unknown => "🔒",
        }
    }

    pub fn from_usb_ids(vendor_id: &str, product_name: Option<&str>) -> Self {
        let v = vendor_id.trim().to_lowercase();
        let name = product_name.unwrap_or("").to_lowercase();

        if v == "1050" || name.contains("yubikey") || name.contains("yubico") {
            Self::Yubico
        } else if v == "20a0" || name.contains("nitrokey") {
            Self::Nitrokey
        } else if v == "1209" || v == "0483" || name.contains("solo") {
            Self::SoloKeys
        } else if v == "096e" || name.contains("feitian") || name.contains("biopass") {
            Self::Feitian
        } else if v == "18d1" || name.contains("titan") {
            Self::GoogleTitan
        } else if v == "4944" || name.contains("kanokey") {
            Self::KanoKey
        } else if v == "32ac" && name.contains("fingerprint") {
            Self::Framework
        } else if name.contains("fido") || name.contains("u2f") {
            Self::Generic
        } else {
            Self::Unknown
        }
    }
}

#[derive(Debug, Clone)]
pub struct FidoDeviceTelemetry {
    pub is_connected: bool,
    pub vendor: FidoVendor,
    pub vendor_name: String,
    pub product_name: String,
    pub serial: Option<String>,
    pub firmware: Option<String>,
    pub form_factor: Option<String>,
    pub has_pin: bool,
    pub pin_retries: Option<u32>,
    pub has_bio: bool,
    pub bio_retries: Option<u32>,
    pub protocols: Vec<String>,
    pub is_yubikey: bool,
    pub hid_path: Option<String>,
}

impl Default for FidoDeviceTelemetry {
    fn default() -> Self {
        Self {
            is_connected: false,
            vendor: FidoVendor::Unknown,
            vendor_name: "None".to_string(),
            product_name: "No Security Key Detected".to_string(),
            serial: None,
            firmware: None,
            form_factor: None,
            has_pin: false,
            pin_retries: None,
            has_bio: false,
            bio_retries: None,
            protocols: Vec::new(),
            is_yubikey: false,
            hid_path: None,
        }
    }
}

/// Detects connected FIDO2 / U2F hardware devices across platforms.
pub fn detect_fido_device() -> FidoDeviceTelemetry {
    #[cfg(target_os = "macos")]
    {
        detect_fido_device_macos()
    }

    #[cfg(not(target_os = "macos"))]
    {
        detect_fido_device_linux()
    }
}

#[cfg(not(target_os = "macos"))]
fn detect_fido_device_linux() -> FidoDeviceTelemetry {
    use std::fs;

    let mut telemetry = FidoDeviceTelemetry::default();
    let mut detected_vendor = FidoVendor::Unknown;
    let mut detected_product = String::new();
    let mut is_connected = false;

    // 1. Scan /sys/bus/usb/devices
    let usb_devices = Path::new("/sys/bus/usb/devices");
    if let Ok(entries) = fs::read_dir(usb_devices) {
        for entry in entries.flatten() {
            let v_path = entry.path().join("idVendor");
            if let Ok(v_id) = fs::read_to_string(&v_path) {
                let v_clean = v_id.trim().to_lowercase();
                let p_path = entry.path().join("product");
                let p_name = fs::read_to_string(&p_path)
                    .ok()
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();

                let vendor = FidoVendor::from_usb_ids(&v_clean, Some(&p_name));
                if vendor != FidoVendor::Unknown {
                    is_connected = true;
                    detected_vendor = vendor;
                    detected_product = if !p_name.is_empty() {
                        p_name
                    } else {
                        format!("{} FIDO2 Device", vendor.display_name())
                    };
                    break;
                }
            }
        }
    }

    // 2. Scan /sys/class/hidraw for FIDO / U2F devices
    let mut found_hid_path = None;
    if let Ok(entries) = fs::read_dir("/sys/class/hidraw") {
        for entry in entries.flatten() {
            let uevent_path = entry.path().join("device/uevent");
            if let Ok(uevent) = fs::read_to_string(&uevent_path) {
                let lower = uevent.to_lowercase();
                if lower.contains("hid_name=") && (lower.contains("yubikey") || lower.contains("fido") || lower.contains("u2f") || lower.contains("nitrokey") || lower.contains("solo")) {
                    let file_name = entry.file_name();
                    found_hid_path = Some(format!("/dev/{}", file_name.to_string_lossy()));
                    if !is_connected {
                        is_connected = true;
                        if lower.contains("yubico") || lower.contains("yubikey") {
                            detected_vendor = FidoVendor::Yubico;
                            detected_product = "YubiKey FIDO2".to_string();
                        } else if lower.contains("nitrokey") {
                            detected_vendor = FidoVendor::Nitrokey;
                            detected_product = "Nitrokey 3 (FIDO2)".to_string();
                        } else if lower.contains("solo") {
                            detected_vendor = FidoVendor::SoloKeys;
                            detected_product = "SoloKeys Solo 2".to_string();
                        } else {
                            detected_vendor = FidoVendor::Generic;
                            detected_product = "FIDO2 Security Key".to_string();
                        }
                    }
                    break;
                }
            }
        }
    }

    if !is_connected {
        return telemetry;
    }

    telemetry.is_connected = true;
    telemetry.vendor = detected_vendor;
    telemetry.vendor_name = detected_vendor.display_name().to_string();
    telemetry.product_name = detected_product;
    telemetry.is_yubikey = detected_vendor == FidoVendor::Yubico;
    telemetry.hid_path = found_hid_path;

    // 3. Enrich with ykman if vendor is Yubico and ykman is installed
    if telemetry.is_yubikey {
        enrich_with_ykman(&mut telemetry);
    } else {
        // Try generic fido2-token enrichment if installed
        enrich_with_fido2_tools(&mut telemetry);
    }

    telemetry
}

#[cfg(target_os = "macos")]
fn detect_fido_device_macos() -> FidoDeviceTelemetry {
    let mut telemetry = FidoDeviceTelemetry::default();

    let output = Command::new("ioreg").args(["-p", "IOUSB", "-l"]).output();
    if let Ok(out) = output {
        let s = String::from_utf8_lossy(&out.stdout);
        let s_lower = s.to_lowercase();

        let vendor = if s_lower.contains("1050") || s_lower.contains("yubikey") {
            FidoVendor::Yubico
        } else if s_lower.contains("20a0") || s_lower.contains("nitrokey") {
            FidoVendor::Nitrokey
        } else if s_lower.contains("1209") || s_lower.contains("solokeys") {
            FidoVendor::SoloKeys
        } else if s_lower.contains("096e") || s_lower.contains("feitian") {
            FidoVendor::Feitian
        } else if s_lower.contains("fido") || s_lower.contains("u2f") {
            FidoVendor::Generic
        } else {
            FidoVendor::Unknown
        };

        if vendor != FidoVendor::Unknown {
            telemetry.is_connected = true;
            telemetry.vendor = vendor;
            telemetry.vendor_name = vendor.display_name().to_string();
            telemetry.is_yubikey = vendor == FidoVendor::Yubico;

            let name = if s_lower.contains("bio") {
                format!("{} Bio (macOS)", vendor.display_name())
            } else {
                format!("{} FIDO2 (macOS)", vendor.display_name())
            };
            telemetry.product_name = name;

            if telemetry.is_yubikey {
                enrich_with_ykman(&mut telemetry);
            }
        }
    }

    telemetry
}

fn enrich_with_ykman(telemetry: &mut FidoDeviceTelemetry) {
    if let Ok(out) = Command::new("ykman").arg("info").output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            for line in s.lines() {
                let trimmed = line.trim();
                if let Some(v) = trimmed.strip_prefix("Device type:") {
                    telemetry.product_name = v.trim().to_string();
                } else if let Some(v) = trimmed.strip_prefix("Serial number:") {
                    telemetry.serial = Some(v.trim().to_string());
                } else if let Some(v) = trimmed.strip_prefix("Firmware version:") {
                    telemetry.firmware = Some(v.trim().to_string());
                } else if let Some(v) = trimmed.strip_prefix("Form factor:") {
                    telemetry.form_factor = Some(v.trim().to_string());
                }
            }
        }
    }

    if let Ok(out) = Command::new("ykman").args(["fido", "info"]).output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            for line in s.lines() {
                let trimmed = line.trim();
                if trimmed.contains("PIN is set") {
                    telemetry.has_pin = true;
                } else if trimmed.contains("Bio") || trimmed.contains("Fingerprint") {
                    telemetry.has_bio = true;
                }
            }
        }
    }
}

fn enrich_with_fido2_tools(telemetry: &mut FidoDeviceTelemetry) {
    if let Some(ref path) = telemetry.hid_path {
        if let Ok(out) = Command::new("fido2-token").args(["-I", path]).output() {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout);
                for line in s.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("proto:") {
                        telemetry.protocols.push(trimmed.replace("proto:", "").trim().to_string());
                    } else if trimmed.contains("clientPin: true") {
                        telemetry.has_pin = true;
                    } else if trimmed.contains("uv: true") || trimmed.contains("bio: true") {
                        telemetry.has_bio = true;
                    } else if let Some(v) = trimmed.strip_prefix("pin retries:") {
                        telemetry.pin_retries = v.trim().parse::<u32>().ok();
                    } else if let Some(v) = trimmed.strip_prefix("uv retries:") {
                        telemetry.bio_retries = v.trim().parse::<u32>().ok();
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vendor_from_ids() {
        assert_eq!(FidoVendor::from_usb_ids("1050", None), FidoVendor::Yubico);
        assert_eq!(FidoVendor::from_usb_ids("20a0", None), FidoVendor::Nitrokey);
        assert_eq!(FidoVendor::from_usb_ids("1209", None), FidoVendor::SoloKeys);
        assert_eq!(FidoVendor::from_usb_ids("096e", None), FidoVendor::Feitian);
        assert_eq!(FidoVendor::from_usb_ids("18d1", None), FidoVendor::GoogleTitan);
        assert_eq!(FidoVendor::from_usb_ids("9999", Some("FIDO2 Authenticator")), FidoVendor::Generic);
        assert_eq!(FidoVendor::from_usb_ids("9999", Some("USB Flash Drive")), FidoVendor::Unknown);
    }

    #[test]
    fn test_telemetry_defaults() {
        let t = FidoDeviceTelemetry::default();
        assert!(!t.is_connected);
        assert_eq!(t.vendor, FidoVendor::Unknown);
    }
}
