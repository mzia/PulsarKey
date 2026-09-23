use std::io::stdout;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, Tabs, Wrap},
    Frame, Terminal,
};

use crate::audit::AuditEvent;
use crate::backup::EnrolledKey;
use crate::bio::FidoInfo;
use crate::hardware::FidoDeviceTelemetry;
use crate::profiles::SecurityProfile;
use crate::rescue::RecoveryStatus;

const TABS: &[&str] = &[
    "📊 Telemetry",
    "🛡️ Profiles",
    "🧬 Biometrics",
    "👯 Backup Keys",
    "🛡️ Sentinel",
    "🛟 Rescue Kit",
    "📜 Audit Log",
];

pub struct TuiApp {
    pub current_tab: usize,
    pub telemetry: FidoDeviceTelemetry,
    pub profile: SecurityProfile,
    pub autolock_enabled: bool,
    pub enrolled_keys: Vec<EnrolledKey>,
    pub fido_info: FidoInfo,
    pub rescue_status: RecoveryStatus,
    pub audit_events: Vec<AuditEvent>,
    pub audit_scroll: usize,
    pub status_message: Option<(String, Instant)>,
    pub should_quit: bool,
    pub tick_count: usize,
}

impl TuiApp {
    pub fn new() -> Self {
        let mut app = Self {
            current_tab: 0,
            telemetry: FidoDeviceTelemetry::default(),
            profile: SecurityProfile::Convenience,
            autolock_enabled: false,
            enrolled_keys: Vec::new(),
            fido_info: FidoInfo::default(),
            rescue_status: RecoveryStatus {
                total: 0,
                unused: 0,
                used: 0,
                is_configured: false,
                file_path: std::path::PathBuf::new(),
            },
            audit_events: Vec::new(),
            audit_scroll: 0,
            status_message: None,
            should_quit: false,
            tick_count: 0,
        };
        app.refresh();
        app
    }

    pub fn refresh(&mut self) {
        self.telemetry = crate::hardware::detect_fido_device();
        self.profile = crate::profiles::get_current_profile();
        let cfg = crate::config::load_config();
        self.autolock_enabled = cfg.autolock;

        let username = std::env::var("SUDO_USER")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_else(|_| "mzia".to_string());
        self.enrolled_keys = crate::backup::get_enrolled_keys(&username);
        self.fido_info = crate::bio::get_fido_info();
        self.rescue_status = crate::rescue::check_recovery_status();
        self.audit_events = crate::audit::get_recorded_events(50);
        self.set_status("Data refreshed.");
    }

    pub fn set_status(&mut self, msg: &str) {
        self.status_message = Some((msg.to_string(), Instant::now()));
    }

    pub fn toggle_sentinel(&mut self) {
        self.autolock_enabled = !self.autolock_enabled;
        let mut cfg = crate::config::load_config();
        cfg.autolock = self.autolock_enabled;
        let _ = crate::config::save_config(&cfg);

        let s = if self.autolock_enabled { "ENABLED" } else { "DISABLED" };
        self.set_status(&format!("Presence Sentinel auto-lock: {}", s));
        crate::audit::log_event(
            "CONFIG",
            "Auto-Lock Toggled (TUI)",
            if self.autolock_enabled { "Enabled" } else { "Disabled" },
            "Toggled via interactive TUI dashboard",
        );
    }

    pub fn next_tab(&mut self) {
        self.current_tab = (self.current_tab + 1) % TABS.len();
    }

    pub fn prev_tab(&mut self) {
        if self.current_tab == 0 {
            self.current_tab = TABS.len() - 1;
        } else {
            self.current_tab -= 1;
        }
    }
}

pub fn run_tui() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = TuiApp::new();
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui(f, &app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            app.should_quit = true;
                        }
                        KeyCode::Tab | KeyCode::Right => {
                            app.next_tab();
                        }
                        KeyCode::BackTab | KeyCode::Left => {
                            app.prev_tab();
                        }
                        KeyCode::Char('1') => app.current_tab = 0,
                        KeyCode::Char('2') => app.current_tab = 1,
                        KeyCode::Char('3') => app.current_tab = 2,
                        KeyCode::Char('4') => app.current_tab = 3,
                        KeyCode::Char('5') => app.current_tab = 4,
                        KeyCode::Char('6') => app.current_tab = 5,
                        KeyCode::Char('7') => app.current_tab = 6,
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            app.refresh();
                        }
                        KeyCode::Char('s') | KeyCode::Char('S') => {
                            app.toggle_sentinel();
                        }
                        KeyCode::Char(' ') => {
                            if app.current_tab == 4 {
                                app.toggle_sentinel();
                            }
                        }
                        KeyCode::Up => {
                            if app.current_tab == 6 && app.audit_scroll > 0 {
                                app.audit_scroll -= 1;
                            }
                        }
                        KeyCode::Down => {
                            if app.current_tab == 6 && app.audit_scroll + 10 < app.audit_events.len() {
                                app.audit_scroll += 1;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.tick_count = app.tick_count.wrapping_add(1);
            last_tick = Instant::now();
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn ui(f: &mut Frame, app: &TuiApp) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(3), // Tabs
            Constraint::Min(10),   // Content
            Constraint::Length(3), // Footer / Keybinds
        ])
        .split(size);

    render_header(f, app, chunks[0]);
    render_tabs(f, app, chunks[1]);

    match app.current_tab {
        0 => render_telemetry_tab(f, app, chunks[2]),
        1 => render_profiles_tab(f, app, chunks[2]),
        2 => render_biometrics_tab(f, app, chunks[2]),
        3 => render_backup_tab(f, app, chunks[2]),
        4 => render_sentinel_tab(f, app, chunks[2]),
        5 => render_rescue_tab(f, app, chunks[2]),
        6 => render_audit_tab(f, app, chunks[2]),
        _ => {}
    }

    render_footer(f, app, chunks[3]);
}

fn render_header(f: &mut Frame, app: &TuiApp, area: Rect) {
    let status_color = if app.telemetry.is_connected {
        Color::Green
    } else {
        Color::Red
    };
    let status_symbol = if app.telemetry.is_connected {
        "🟢 CONNECTED"
    } else {
        "🔴 DISCONNECTED"
    };

    let profile_badge = match app.profile {
        SecurityProfile::Convenience => ("Convenience (1FA)", Color::Cyan),
        SecurityProfile::Fortress => ("Fortress (2FA)", Color::Blue),
        SecurityProfile::Lockdown => ("Lockdown (Strict)", Color::Red),
    };

    let sentinel_badge = if app.autolock_enabled {
        ("Sentinel: ON", Color::Green)
    } else {
        ("Sentinel: OFF", Color::DarkGray)
    };

    let header_line = Line::from(vec![
        Span::styled("🌌 PulsarKey ", Style::default().fg(Color::Cyan).bold()),
        Span::styled("v1.5.0 ", Style::default().fg(Color::DarkGray)),
        Span::raw(" │ "),
        Span::styled(status_symbol, Style::default().fg(status_color).bold()),
        Span::raw(" ("),
        Span::styled(&app.telemetry.product_name, Style::default().fg(Color::White)),
        Span::raw(") │ Profile: "),
        Span::styled(profile_badge.0, Style::default().fg(profile_badge.1).bold()),
        Span::raw(" │ "),
        Span::styled(sentinel_badge.0, Style::default().fg(sentinel_badge.1).bold()),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(header_line)
        .alignment(Alignment::Center)
        .block(block);

    f.render_widget(paragraph, area);
}

fn render_tabs(f: &mut Frame, app: &TuiApp, area: Rect) {
    let titles: Vec<Line> = TABS
        .iter()
        .map(|t| Line::from(Span::styled(*t, Style::default().fg(Color::White))))
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .select(app.current_tab)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )
        .divider(Span::raw(" │ "));

    f.render_widget(tabs, area);
}

fn render_telemetry_tab(f: &mut Frame, app: &TuiApp, area: Rect) {
    let sub_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left card: Hardware Telemetry
    let vendor_icon = app.telemetry.vendor.icon();
    let vendor_name = app.telemetry.vendor.display_name();

    let mut hw_lines = vec![
        Line::from(vec![
            Span::styled("Hardware Status: ", Style::default().bold()),
            if app.telemetry.is_connected {
                Span::styled("ONLINE (Active)", Style::default().fg(Color::Green).bold())
            } else {
                Span::styled("OFFLINE (Not Detected)", Style::default().fg(Color::Red).bold())
            },
        ]),
        Line::from(vec![
            Span::styled("Vendor:          ", Style::default().bold()),
            Span::raw(format!("{} {}", vendor_icon, vendor_name)),
        ]),
        Line::from(vec![
            Span::styled("Product Model:   ", Style::default().bold()),
            Span::styled(&app.telemetry.product_name, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Serial Number:   ", Style::default().bold()),
            Span::raw(app.telemetry.serial.as_deref().unwrap_or("N/A (FIDO Protected)")),
        ]),
        Line::from(vec![
            Span::styled("Firmware:        ", Style::default().bold()),
            Span::raw(app.telemetry.firmware.as_deref().unwrap_or("N/A")),
        ]),
        Line::from(vec![
            Span::styled("Form Factor:     ", Style::default().bold()),
            Span::raw(app.telemetry.form_factor.as_deref().unwrap_or("USB / Generic")),
        ]),
        Line::from(vec![
            Span::styled("FIDO2 Protocols: ", Style::default().bold()),
            Span::raw(if app.telemetry.protocols.is_empty() {
                "FIDO_2_0 / U2F_V2".to_string()
            } else {
                app.telemetry.protocols.join(", ")
            }),
        ]),
        Line::from(vec![
            Span::styled("HID Raw Node:    ", Style::default().bold()),
            Span::raw(app.telemetry.hid_path.as_deref().unwrap_or("/dev/hidraw* (Auto)")),
        ]),
    ];

    if app.telemetry.has_bio {
        hw_lines.push(Line::from(vec![
            Span::styled("Biometric Sensor:", Style::default().bold()),
            Span::styled(" Present (Fingerprint Capable)", Style::default().fg(Color::Green)),
        ]));
    }

    let left_block = Block::default()
        .title(" 🧬 Hardware & Device Telemetry ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let left_para = Paragraph::new(hw_lines).block(left_block).wrap(Wrap { trim: false });
    f.render_widget(left_para, sub_chunks[0]);

    // Right card: PAM & System Health
    let pam_sudo = if std::path::Path::new("/etc/pam.d/sudo").exists() {
        Span::styled("Configured (pam_u2f)", Style::default().fg(Color::Green))
    } else {
        Span::styled("Not Configured", Style::default().fg(Color::Red))
    };

    let pam_greeter = if std::path::Path::new("/etc/pam.d/cosmic-greeter").exists() {
        Span::styled("Configured (COSMIC Greeter)", Style::default().fg(Color::Green))
    } else {
        Span::styled("Default / Unlinked", Style::default().fg(Color::Yellow))
    };

    let pam_polkit = if std::path::Path::new("/etc/pam.d/polkit-1").exists() {
        Span::styled("Configured (Polkit GUI)", Style::default().fg(Color::Green))
    } else {
        Span::styled("Default / System", Style::default().fg(Color::Yellow))
    };

    let right_lines = vec![
        Line::from(vec![
            Span::styled("Sudo Authentication:  ", Style::default().bold()),
            pam_sudo,
        ]),
        Line::from(vec![
            Span::styled("Lockscreen Greeter:   ", Style::default().bold()),
            pam_greeter,
        ]),
        Line::from(vec![
            Span::styled("Polkit GUI Elevation: ", Style::default().bold()),
            pam_polkit,
        ]),
        Line::from(Span::raw("")),
        Line::from(vec![
            Span::styled("Registered Keys:      ", Style::default().bold()),
            Span::styled(format!("{} key(s) enrolled", app.enrolled_keys.len()), Style::default().fg(Color::Cyan).bold()),
        ]),
        Line::from(vec![
            Span::styled("Primary Key Status:   ", Style::default().bold()),
            if app.enrolled_keys.is_empty() {
                Span::styled("None enrolled", Style::default().fg(Color::Red))
            } else {
                Span::styled(&app.enrolled_keys[0].credential_summary, Style::default().fg(Color::White))
            },
        ]),
        Line::from(vec![
            Span::styled("Redundancy Health:    ", Style::default().bold()),
            if app.enrolled_keys.len() >= 2 {
                Span::styled("Protected (Backup key paired)", Style::default().fg(Color::Green).bold())
            } else {
                Span::styled("Warning (Single key - pair a backup!)", Style::default().fg(Color::Yellow).bold())
            },
        ]),
    ];

    let right_block = Block::default()
        .title(" 🛡️ System PAM & Redundancy Health ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Blue));

    let right_para = Paragraph::new(right_lines).block(right_block).wrap(Wrap { trim: false });
    f.render_widget(right_para, sub_chunks[1]);
}

fn render_profiles_tab(f: &mut Frame, app: &TuiApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(12), Constraint::Min(4)])
        .split(area);

    let profile_rows = vec![
        Row::new(vec![
            Cell::from(if app.profile == SecurityProfile::Convenience { "● Convenience" } else { "○ Convenience" })
                .style(Style::default().fg(Color::Cyan).bold()),
            Cell::from("1FA (Biometric / Touch Alone)"),
            Cell::from("Allowed (Seamless login when key absent)"),
            Cell::from("Fast daily use on laptop / desktop"),
        ]),
        Row::new(vec![
            Cell::from(if app.profile == SecurityProfile::Fortress { "● Fortress" } else { "○ Fortress" })
                .style(Style::default().fg(Color::Blue).bold()),
            Cell::from("True 2FA (Password + Key Touch)"),
            Cell::from("Denied (Both factors strictly mandatory)"),
            Cell::from("High security for developer & work devices"),
        ]),
        Row::new(vec![
            Cell::from(if app.profile == SecurityProfile::Lockdown { "● Lockdown" } else { "○ Lockdown" })
                .style(Style::default().fg(Color::Red).bold()),
            Cell::from("Strict Hardware (Key strictly mandatory)"),
            Cell::from("Zero fallback (Password authentication removed)"),
            Cell::from("Air-gapped / ultra-sensitive environments"),
        ]),
    ];

    let table = Table::new(
        profile_rows,
        [
            Constraint::Length(18),
            Constraint::Length(32),
            Constraint::Length(36),
            Constraint::Min(25),
        ],
    )
    .header(
        Row::new(vec!["Profile", "Authentication Mode", "Password Fallback", "Recommended Use"])
            .style(Style::default().fg(Color::Yellow).bold()),
    )
    .block(
        Block::default()
            .title(" 🛡️ Security Strictness Profiles ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(table, chunks[0]);

    let instructions = vec![
        Line::from(vec![
            Span::styled("Active Profile: ", Style::default().bold()),
            Span::styled(app.profile.display_name(), Style::default().fg(Color::Cyan).bold()),
        ]),
        Line::from(Span::raw("")),
        Line::from(vec![
            Span::styled("Switch Profile via CLI: ", Style::default().bold()),
            Span::styled("sudo pulsarkey profile [convenience | fortress | lockdown]", Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("Or from Desktop Applet: ", Style::default().bold()),
            Span::raw("Click top-bar applet icon -> Select 'Security Profile' submenu."),
        ]),
    ];

    let info_block = Block::default()
        .title(" ℹ️ Profile Configuration ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let info_para = Paragraph::new(instructions).block(info_block);
    f.render_widget(info_para, chunks[1]);
}

fn render_biometrics_tab(f: &mut Frame, app: &TuiApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let fp_rows = vec![
        Row::new(vec![
            Cell::from("Biometric Sensor:").style(Style::default().bold()),
            Cell::from(if app.fido_info.bio_supported { "Supported (Touch / Fingerprint)" } else { "Not Present / Touch Only" })
                .style(Style::default().fg(if app.fido_info.bio_supported { Color::Green } else { Color::DarkGray })),
        ]),
        Row::new(vec![
            Cell::from("Templates Enrolled:").style(Style::default().bold()),
            Cell::from(if app.fido_info.fingerprints_registered { "Yes (Fingerprints Enrolled)" } else { "None Enrolled" })
                .style(Style::default().fg(if app.fido_info.fingerprints_registered { Color::Green } else { Color::Yellow })),
        ]),
        Row::new(vec![
            Cell::from("Security Policy:").style(Style::default().bold()),
            Cell::from("Protected on-chip (FIDO2 PIN required to enumerate)"),
        ]),
    ];

    let fp_table = Table::new(
        fp_rows,
        [Constraint::Percentage(45), Constraint::Percentage(55)],
    )
    .header(Row::new(vec!["Biometric Capability", "Hardware Status"]).style(Style::default().fg(Color::Yellow).bold()))
    .block(
        Block::default()
            .title(" 🧬 Biometric Sensor & Templates ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Green)),
    );

    f.render_widget(fp_table, chunks[0]);

    // Right: PIN & Retries
    let pin_status = if app.fido_info.pin_set {
        Span::styled("Configured (PIN Set)", Style::default().fg(Color::Green).bold())
    } else {
        Span::styled("No PIN Set", Style::default().fg(Color::Yellow).bold())
    };

    let pin_lines = vec![
        Line::from(vec![
            Span::styled("FIDO2 Hardware PIN: ", Style::default().bold()),
            pin_status,
        ]),
        Line::from(vec![
            Span::styled("PIN Retries Left:   ", Style::default().bold()),
            Span::raw(format!("{}", app.fido_info.pin_retries.map(|n| n.to_string()).unwrap_or_else(|| "8 (Default)".to_string()))),
        ]),
        Line::from(vec![
            Span::styled("Bio Retries Left:   ", Style::default().bold()),
            Span::raw(format!("{}", app.fido_info.bio_retries.map(|n| n.to_string()).unwrap_or_else(|| "3 (Default)".to_string()))),
        ]),
        Line::from(vec![
            Span::styled("Always Require UV:  ", Style::default().bold()),
            if app.fido_info.always_uv {
                Span::styled("Enabled", Style::default().fg(Color::Green))
            } else {
                Span::raw("Disabled")
            },
        ]),
        Line::from(Span::raw("")),
        Line::from(vec![
            Span::styled("Manage via CLI:", Style::default().bold()),
        ]),
        Line::from(Span::styled("  pulsarkey bio add <label>    - Enroll new finger", Style::default().fg(Color::Cyan))),
        Line::from(Span::styled("  pulsarkey pin change         - Set / change FIDO PIN", Style::default().fg(Color::Cyan))),
    ];

    let pin_block = Block::default()
        .title(" 🔑 Security PIN & Verification ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Blue));

    let pin_para = Paragraph::new(pin_lines).block(pin_block);
    f.render_widget(pin_para, chunks[1]);
}

fn render_backup_tab(f: &mut Frame, app: &TuiApp, area: Rect) {
    let rows: Vec<Row> = if app.enrolled_keys.is_empty() {
        vec![Row::new(vec![
            Cell::from("No keys found in /etc/yubico/u2f_keys"),
            Cell::from("Run: sudo pulsarkey setup"),
            Cell::from("-"),
        ])]
    } else {
        app.enrolled_keys
            .iter()
            .map(|k| {
                Row::new(vec![
                    Cell::from(format!("Key #{}", k.index)).style(Style::default().fg(Color::Cyan).bold()),
                    Cell::from(k.credential_summary.clone()),
                    Cell::from(if k.has_uv { "Biometrics / UV Enabled" } else { "Touch Only" })
                        .style(Style::default().fg(if k.has_uv { Color::Green } else { Color::Yellow })),
                ])
            })
            .collect()
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Percentage(60),
            Constraint::Percentage(30),
        ],
    )
    .header(Row::new(vec!["Slot", "Public Credential ID", "Capabilities"]).style(Style::default().fg(Color::Yellow).bold()))
    .block(
        Block::default()
            .title(format!(" 👯 Registered Hardware Keys (Total: {}) ", app.enrolled_keys.len()))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(table, area);
}

fn render_sentinel_tab(f: &mut Frame, app: &TuiApp, area: Rect) {
    let state_text = if app.autolock_enabled {
        Line::from(vec![
            Span::styled("🛡️ PRESENCE SENTINEL: ", Style::default().bold()),
            Span::styled("ARMED & ACTIVE", Style::default().fg(Color::Green).bold()),
        ])
    } else {
        Line::from(vec![
            Span::styled("🛡️ PRESENCE SENTINEL: ", Style::default().bold()),
            Span::styled("STANDBY / DISABLED", Style::default().fg(Color::DarkGray).bold()),
        ])
    };

    let lines = vec![
        state_text,
        Line::from(Span::raw("")),
        Line::from(Span::raw("When Armed, PulsarKey continuously monitors USB hardware connection.")),
        Line::from(Span::raw("The moment your security key is unplugged, your workstation locks instantly.")),
        Line::from(Span::raw("")),
        Line::from(vec![
            Span::styled("Toggle Shortcut: ", Style::default().bold()),
            Span::styled("Press [S] or [Space] right now to toggle.", Style::default().fg(Color::Yellow).bold()),
        ]),
        Line::from(vec![
            Span::styled("CLI Command:     ", Style::default().bold()),
            Span::styled("pulsarkey autolock [enable|disable]", Style::default().fg(Color::Cyan)),
        ]),
    ];

    let block = Block::default()
        .title(" 🛡️ Presence Sentinel Auto-Lock ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if app.autolock_enabled { Style::default().fg(Color::Green) } else { Style::default().fg(Color::DarkGray) });

    let para = Paragraph::new(lines).block(block);
    f.render_widget(para, area);
}

fn render_rescue_tab(f: &mut Frame, app: &TuiApp, area: Rect) {
    let lines = vec![
        Line::from(vec![
            Span::styled("Emergency Paper Tokens: ", Style::default().bold()),
            if app.rescue_status.is_configured {
                Span::styled(format!("{}/{} Unused Tokens Ready", app.rescue_status.unused, app.rescue_status.total), Style::default().fg(Color::Green).bold())
            } else {
                Span::styled("Not Generated Yet", Style::default().fg(Color::Yellow).bold())
            },
        ]),
        Line::from(vec![
            Span::styled("Storage File:           ", Style::default().bold()),
            Span::raw(app.rescue_status.file_path.display().to_string()),
        ]),
        Line::from(Span::raw("")),
        Line::from(Span::styled("Commands:", Style::default().bold())),
        Line::from(Span::styled("  pulsarkey rescue generate   - Print 8 emergency one-time recovery codes", Style::default().fg(Color::Cyan))),
        Line::from(Span::styled("  pulsarkey rescue runbook    - Display offline chroot/LUKS rescue guide", Style::default().fg(Color::Cyan))),
        Line::from(Span::styled("  sudo pulsarkey rescue usb <PATH> - Create emergency offline rescue USB", Style::default().fg(Color::Cyan))),
    ];

    let block = Block::default()
        .title(" 🛟 Emergency Recovery Kit & Runbook ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow));

    let para = Paragraph::new(lines).block(block);
    f.render_widget(para, area);
}

fn render_audit_tab(f: &mut Frame, app: &TuiApp, area: Rect) {
    let rows: Vec<Row> = if app.audit_events.is_empty() {
        vec![Row::new(vec![
            Cell::from("No audit events recorded yet"),
            Cell::from("-"),
            Cell::from("-"),
            Cell::from("-"),
        ])]
    } else {
        app.audit_events
            .iter()
            .skip(app.audit_scroll)
            .take(15)
            .map(|e| {
                let status_color = match e.status.to_lowercase().as_str() {
                    "connected" | "enabled" | "success" => Color::Green,
                    "disconnected" | "disabled" => Color::Yellow,
                    "failed" | "error" => Color::Red,
                    _ => Color::White,
                };

                Row::new(vec![
                    Cell::from(e.timestamp.clone()).style(Style::default().fg(Color::DarkGray)),
                    Cell::from(e.category.clone()).style(Style::default().fg(Color::Cyan)),
                    Cell::from(e.action.clone()).style(Style::default().bold()),
                    Cell::from(e.status.clone()).style(Style::default().fg(status_color).bold()),
                    Cell::from(e.details.clone()),
                ])
            })
            .collect()
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(20),
            Constraint::Length(12),
            Constraint::Length(24),
            Constraint::Length(14),
            Constraint::Min(30),
        ],
    )
    .header(Row::new(vec!["Timestamp", "Category", "Action", "Status", "Details"]).style(Style::default().fg(Color::Yellow).bold()))
    .block(
        Block::default()
            .title(format!(" 📜 Authentication Audit Journal (Events: {}, Scroll: [↑/↓]) ", app.audit_events.len()))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(table, area);
}

fn render_footer(f: &mut Frame, app: &TuiApp, area: Rect) {
    let status_text = if let Some((ref msg, time)) = app.status_message {
        if time.elapsed() < Duration::from_secs(4) {
            format!("💡 {}", msg)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let nav_line = Line::from(vec![
        Span::styled("[Tab/←/→] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Navigate Tabs  │ "),
        Span::styled("[1-7] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Direct  │ "),
        Span::styled("[S] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Toggle Sentinel  │ "),
        Span::styled("[R] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Refresh  │ "),
        Span::styled("[Q/Esc] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Exit  "),
        Span::styled(status_text, Style::default().fg(Color::Green).bold()),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(nav_line)
        .alignment(Alignment::Left)
        .block(block);

    f.render_widget(paragraph, area);
}
