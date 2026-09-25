use clap::Subcommand;
use colored::*;

pub mod applet;
pub mod audit;
pub mod backup;
pub mod bio;
pub mod config;
pub mod gui;
pub mod hardware;
pub mod platform;
pub mod profiles;
pub mod rescue;
pub mod ssh_setup;
pub mod tui;
pub mod updater;

#[derive(Subcommand, Debug, Clone)]
pub enum BioCommands {
    /// List registered fingerprints on the security key
    List {
        /// FIDO2 PIN (prompted securely if omitted)
        #[arg(short, long)]
        pin: Option<String>,
    },
    /// Enroll a new fingerprint
    Add {
        /// Name / label for the fingerprint (e.g. "Right Index")
        name: Option<String>,
        /// FIDO2 PIN (prompted securely if omitted)
        #[arg(short, long)]
        pin: Option<String>,
    },
    /// Delete a registered fingerprint
    Delete {
        /// Fingerprint ID to delete
        id: Option<String>,
        /// FIDO2 PIN (prompted securely if omitted)
        #[arg(short, long)]
        pin: Option<String>,
        /// Delete without confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
    /// Rename a registered fingerprint
    Rename {
        /// Fingerprint ID to rename
        id: Option<String>,
        /// New name / label (max 15 chars)
        name: Option<String>,
        /// FIDO2 PIN (prompted securely if omitted)
        #[arg(short, long)]
        pin: Option<String>,
    },
    /// Set or change FIDO2 hardware PIN
    Pin {
        /// Action: "change", "set", "verify", or "status"
        action: Option<String>,
    },
}

pub fn ensure_root(action: &str) {
    let uid = unsafe { libc::geteuid() };
    if uid != 0 {
        eprintln!(
            "{} Root privileges required to {} system authentication.",
            "❌ Error:".bold().red(),
            action
        );
        eprintln!(
            "Please run with sudo: {}",
            format!("sudo pulsarkey {}", action).bold().yellow()
        );
        std::process::exit(1);
    }
}
