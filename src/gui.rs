use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Element, Length, Task, Theme};
use std::process::Command;

use crate::{audit, backup, config, profiles, rescue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Overview,
    Profiles,
    Biometrics,
    Audit,
    Backup,
    Rescue,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectTab(Tab),
    RefreshStatus,
    SwitchProfile(String),
    ToggleAutoLock,
    ClearAuditLog,
    GenerateRescueKit,
    LaunchTerminal(String),
}

pub struct SettingsApp {
    current_tab: Tab,
    status_message: Option<String>,
    device_info: String,
    current_profile: String,
    autolock_enabled: bool,
    redundancy_info: String,
    audit_events: Vec<audit::AuditEvent>,
    rescue_status: rescue::RecoveryStatus,
}

impl Default for SettingsApp {
    fn default() -> Self {
        let mut app = Self {
            current_tab: Tab::Overview,
            status_message: None,
            device_info: "Detecting hardware...".to_string(),
            current_profile: "Convenience (1FA)".to_string(),
            autolock_enabled: true,
            redundancy_info: "Single key".to_string(),
            audit_events: Vec::new(),
            rescue_status: rescue::check_recovery_status(),
        };
        app.reload_data();
        app
    }
}

impl SettingsApp {
    pub fn reload_data(&mut self) {
        // Hardware info
        let yk_out = Command::new("ykman").arg("info").output();
        if let Ok(out) = yk_out {
            if out.status.success() {
                let info = String::from_utf8_lossy(&out.stdout);
                let first_few: Vec<&str> = info.lines().take(4).collect();
                self.device_info = first_few.join("\n");
            } else {
                self.device_info = "No YubiKey hardware detected.".to_string();
            }
        } else {
            self.device_info = "ykman utility not found.".to_string();
        }

        // Profile info
        self.current_profile = profiles::get_current_profile().display_name().to_string();

        // Autolock info
        let cfg = config::load_config();
        self.autolock_enabled = cfg.autolock;

        // Redundancy info
        let user = std::env::var("USER").unwrap_or_else(|_| "mzia".to_string());
        let keys = backup::get_enrolled_keys(&user);
        self.redundancy_info = if keys.len() > 1 {
            format!("Protected ({} hardware keys enrolled)", keys.len())
        } else {
            "Single hardware key (No backup)".to_string()
        };

        // Audit events
        self.audit_events = audit::get_system_auth_events(15);

        // Rescue status
        self.rescue_status = rescue::check_recovery_status();
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectTab(tab) => {
                self.current_tab = tab;
                Task::none()
            }
            Message::RefreshStatus => {
                self.reload_data();
                self.status_message = Some("System & hardware state refreshed.".to_string());
                Task::none()
            }
            Message::SwitchProfile(mode) => {
                let cmd = format!("pulsarkey profile {}", mode);
                let status = crate::platform::run_elevated(&cmd);

                match status {
                    Ok(s) if s.success() => {
                        self.reload_data();
                        self.status_message = Some(format!("Successfully switched security profile to {}!", mode));
                    }
                    Ok(_) => {
                        self.status_message = Some("Authorization cancelled or denied.".to_string());
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Error applying profile: {}", e));
                    }
                }
                Task::none()
            }
            Message::ToggleAutoLock => {
                let mut cfg = config::load_config();
                cfg.autolock = !cfg.autolock;
                let _ = config::save_config(&cfg);
                self.autolock_enabled = cfg.autolock;
                self.status_message = Some(format!(
                    "Presence Sentinel Auto-Lock {}",
                    if cfg.autolock { "Enabled" } else { "Disabled" }
                ));
                Task::none()
            }
            Message::ClearAuditLog => {
                let _ = audit::clear_audit_log();
                self.audit_events.clear();
                self.status_message = Some("Authentication audit log cleared.".to_string());
                Task::none()
            }
            Message::GenerateRescueKit => {
                match rescue::generate_new_recovery_set() {
                    Ok((_, _, export_path)) => {
                        self.rescue_status = rescue::check_recovery_status();
                        self.status_message = Some(format!(
                            "Generated 8 Recovery Tokens! Saved to {}",
                            export_path.display()
                        ));
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Rescue kit generation failed: {}", e));
                    }
                }
                Task::none()
            }
            Message::LaunchTerminal(cmd) => {
                crate::platform::launch_in_terminal(&cmd);
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let title_bar = row![
            text("🌌 PulsarKey Control Panel").size(22),
            Space::new().width(Length::Fill),
            button("🔄 Refresh").on_press(Message::RefreshStatus),
        ]
        .align_y(Alignment::Center)
        .padding(10);

        let sidebar = column![
            button("📊 Overview")
                .width(Length::Fill)
                .on_press(Message::SelectTab(Tab::Overview)),
            button("🛡️ Security Profiles")
                .width(Length::Fill)
                .on_press(Message::SelectTab(Tab::Profiles)),
            button("🧬 Biometrics & PIN")
                .width(Length::Fill)
                .on_press(Message::SelectTab(Tab::Biometrics)),
            button("📜 Audit Journal")
                .width(Length::Fill)
                .on_press(Message::SelectTab(Tab::Audit)),
            button("👯 Backup Keys")
                .width(Length::Fill)
                .on_press(Message::SelectTab(Tab::Backup)),
            button("🛟 Emergency Rescue")
                .width(Length::Fill)
                .on_press(Message::SelectTab(Tab::Rescue)),
            Space::new().height(Length::Fill),
            text(format!("v1.4.0 {}", crate::platform::get_os_display_name())).size(12),
        ]
        .spacing(8)
        .width(180)
        .padding(10);

        let content_area: Element<'_, Message> = match self.current_tab {
            Tab::Overview => self.view_overview(),
            Tab::Profiles => self.view_profiles(),
            Tab::Biometrics => self.view_biometrics(),
            Tab::Audit => self.view_audit(),
            Tab::Backup => self.view_backup(),
            Tab::Rescue => self.view_rescue(),
        };

        let main_body = row![
            container(sidebar).padding(5),
            container(scrollable(content_area)).width(Length::Fill).padding(15),
        ];

        let mut root = column![title_bar];
        if let Some(ref msg) = self.status_message {
            root = root.push(
                container(text(format!("ℹ️ {}", msg)).size(14))
                    .padding(8)
                    .width(Length::Fill),
            );
        }
        root = root.push(main_body);

        container(root).width(Length::Fill).height(Length::Fill).into()
    }

    fn view_overview(&self) -> Element<'_, Message> {
        let hardware_card = column![
            text("Hardware Detection").size(18),
            text(&self.device_info).size(14),
        ]
        .spacing(6);

        let security_card = column![
            text("Current Security Configuration").size(18),
            text(format!("• Active Profile:   {}", self.current_profile)).size(14),
            text(format!("• Key Redundancy:   {}", self.redundancy_info)).size(14),
            text(format!(
                "• Sentinel Auto-Lock: {}",
                if self.autolock_enabled { "Enabled" } else { "Disabled" }
            ))
            .size(14),
            button(if self.autolock_enabled {
                "Disable Sentinel Auto-Lock"
            } else {
                "Enable Sentinel Auto-Lock"
            })
            .on_press(Message::ToggleAutoLock),
        ]
        .spacing(8);

        column![
            text("System Security Overview").size(20),
            Space::new().height(10),
            container(hardware_card).padding(10),
            Space::new().height(10),
            container(security_card).padding(10),
        ]
        .spacing(10)
        .into()
    }

    fn view_profiles(&self) -> Element<'_, Message> {
        let p_convenience = column![
            text("1. Convenience (1FA Biometric/Touch)").size(16),
            text("Touch or fingerprint alone unlocks sudo, greeter, and Polkit GUI. Password fallback enabled if token absent.").size(13),
            button("Apply Convenience Profile").on_press(Message::SwitchProfile("convenience".to_string())),
        ].spacing(6);

        let p_fortress = column![
            text("2. Fortress (True 2FA: Password + Hardware)").size(16),
            text("Requires BOTH your account password AND physical security key sensor touch.").size(13),
            button("Apply Fortress Profile").on_press(Message::SwitchProfile("fortress".to_string())),
        ].spacing(6);

        let p_lockdown = column![
            text("3. Lockdown (Hardware Strictly Mandatory)").size(16),
            text("Hardware presence strictly mandatory. Password fallback is completely disabled.").size(13),
            button("Apply Lockdown Profile").on_press(Message::SwitchProfile("lockdown".to_string())),
        ].spacing(6);

        column![
            text("Security Strictness Profiles").size(20),
            text(format!("Currently active: {}", self.current_profile)).size(14),
            Space::new().height(10),
            container(p_convenience).padding(10),
            Space::new().height(10),
            container(p_fortress).padding(10),
            Space::new().height(10),
            container(p_lockdown).padding(10),
        ]
        .spacing(8)
        .into()
    }

    fn view_biometrics(&self) -> Element<'_, Message> {
        let bio_card = column![
            text("On-Key Biometric Fingerprint Manager").size(18),
            text("Manage enrolled fingerprints and FIDO2 user verification directly on hardware.").size(13),
            row![
                button("Open Biometrics Dashboard").on_press(Message::LaunchTerminal("pulsarkey bio; read -p 'Press Enter...'".to_string())),
                button("Enroll New Fingerprint").on_press(Message::LaunchTerminal("pulsarkey bio add; read -p 'Press Enter...'".to_string())),
                button("Manage Hardware PIN").on_press(Message::LaunchTerminal("pulsarkey pin change; read -p 'Press Enter...'".to_string())),
            ].spacing(10),
        ].spacing(10);

        column![
            text("Biometrics & PIN Control").size(20),
            Space::new().height(10),
            container(bio_card).padding(10),
        ]
        .spacing(10)
        .into()
    }

    fn view_audit(&self) -> Element<'_, Message> {
        let mut events_col = column![
            row![
                text("Authentication Audit Journal").size(18),
                Space::new().width(Length::Fill),
                button("Clear Journal").on_press(Message::ClearAuditLog),
                button("View Full Journal in Terminal").on_press(Message::LaunchTerminal("pulsarkey audit; read -p 'Press Enter...'".to_string())),
            ].align_y(Alignment::Center),
            text("Harvested from ~/.config/pulsarkey/audit.log and system journalctl:").size(12),
            Space::new().height(5),
        ].spacing(8);

        for ev in &self.audit_events {
            let row_item = row![
                text(&ev.timestamp).size(12).width(130),
                text(&ev.category).size(12).width(90),
                text(&ev.action).size(12).width(180),
                text(&ev.status).size(12).width(80),
                text(&ev.details).size(12),
            ]
            .spacing(5);
            events_col = events_col.push(row_item);
        }

        if self.audit_events.is_empty() {
            events_col = events_col.push(text("No recent authentication events recorded.").size(13));
        }

        column![
            text("Audit & Event Log").size(20),
            Space::new().height(10),
            container(events_col).padding(10),
        ]
        .spacing(10)
        .into()
    }

    fn view_backup(&self) -> Element<'_, Message> {
        let backup_card = column![
            text("Hardware Key Redundancy").size(18),
            text(format!("Current status: {}", self.redundancy_info)).size(14),
            text("Registering a secondary YubiKey ensures you can unlock your device and access sudo if your primary key is misplaced.").size(13),
            Space::new().height(5),
            row![
                button("Pair Backup YubiKey Wizard").on_press(Message::LaunchTerminal("sudo pulsarkey backup pair; read -p 'Press Enter...'".to_string())),
                button("Test Backup Key").on_press(Message::LaunchTerminal("pulsarkey backup test; read -p 'Press Enter...'".to_string())),
            ].spacing(10),
        ].spacing(8);

        column![
            text("Backup Keys & Redundancy").size(20),
            Space::new().height(10),
            container(backup_card).padding(10),
        ]
        .spacing(10)
        .into()
    }

    fn view_rescue(&self) -> Element<'_, Message> {
        let rescue_card = column![
            text("Emergency Paper Recovery Tokens").size(18),
            text(format!(
                "Configured: {} (Remaining: {}/{})",
                if self.rescue_status.is_configured { "Yes" } else { "No" },
                self.rescue_status.unused,
                self.rescue_status.total
            )).size(14),
            text("Emergency paper keys can be printed and stored offline in a safe location to recover root access if all hardware tokens are destroyed.").size(13),
            Space::new().height(5),
            row![
                button("Generate New Emergency Paper Key").on_press(Message::GenerateRescueKit),
                button("Open Emergency Runbook").on_press(Message::LaunchTerminal("pulsarkey rescue runbook; read -p 'Press Enter...'".to_string())),
            ].spacing(10),
        ].spacing(8);

        column![
            text("Emergency Recovery & Rescue Runbook").size(20),
            Space::new().height(10),
            container(rescue_card).padding(10),
        ]
        .spacing(10)
        .into()
    }

    pub fn theme(&self) -> Theme {
        Theme::Dark
    }
}

pub fn run_gui() -> iced::Result {
    iced::application(
        || (SettingsApp::default(), Task::none()),
        SettingsApp::update,
        SettingsApp::view,
    )
    .title("PulsarKey — Security & Hardware Manager")
    .theme(SettingsApp::theme)
    .window(iced::window::Settings {
        size: iced::Size::new(820.0, 580.0),
        min_size: Some(iced::Size::new(700.0, 480.0)),
        resizable: true,
        ..Default::default()
    })
    .run()
}
