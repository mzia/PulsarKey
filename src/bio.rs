use colored::*;
use std::io::{self, Write};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Default)]
pub struct FidoInfo {
    pub device_detected: bool,
    pub pin_set: bool,
    pub pin_retries: Option<u32>,
    pub bio_supported: bool,
    pub fingerprints_registered: bool,
    pub bio_retries: Option<u32>,
    pub always_uv: bool,
}

#[derive(Debug, Clone)]
pub struct Fingerprint {
    pub id: String,
    pub name: String,
}

/// Safely prompt for a PIN without echoing characters to the terminal.
pub fn prompt_pin(prompt: &str) -> io::Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;

    let stdin_fd = libc::STDIN_FILENO;
    let is_tty = unsafe { libc::isatty(stdin_fd) == 1 };

    let mut original = None;
    if is_tty {
        let mut termios = std::mem::MaybeUninit::<libc::termios>::uninit();
        unsafe {
            if libc::tcgetattr(stdin_fd, termios.as_mut_ptr()) == 0 {
                let mut raw = termios.assume_init();
                original = Some(raw);
                raw.c_lflag &= !libc::ECHO;
                libc::tcsetattr(stdin_fd, libc::TCSANOW, &raw);
            }
        }
    }

    let mut input = String::new();
    let res = io::stdin().read_line(&mut input);

    if let Some(orig) = original {
        unsafe {
            libc::tcsetattr(stdin_fd, libc::TCSANOW, &orig);
        }
    }
    println!();

    res.map(|_| input.trim_end_matches(&['\r', '\n'][..]).to_string())
}

/// Queries FIDO2 / Biometric status via ykman fido info.
pub fn get_fido_info() -> FidoInfo {
    let mut info = FidoInfo::default();

    let output = match Command::new("ykman").args(["fido", "info"]).output() {
        Ok(out) => out,
        Err(_) => return info,
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let full = format!("{}\n{}", stdout, stderr);

    if full.contains("No YubiKey found") || full.contains("Error:") {
        info.device_detected = false;
        return info;
    }

    info.device_detected = true;

    for line in full.lines() {
        let l = line.trim();
        if l.starts_with("PIN is set") {
            info.pin_set = true;
            info.pin_retries = parse_retries(l);
        } else if l.starts_with("PIN is not set") {
            info.pin_set = false;
        }

        if l.starts_with("Fingerprints registered") {
            info.bio_supported = true;
            info.fingerprints_registered = true;
            info.bio_retries = parse_retries(l);
        } else if l.starts_with("No fingerprints registered") {
            info.bio_supported = true;
            info.fingerprints_registered = false;
            info.bio_retries = parse_retries(l);
        }

        if l.contains("Always Require User Verification is turned on") {
            info.always_uv = true;
        }
    }

    // Check device type for bio capabilities if bio_supported is still false
    if !info.bio_supported {
        if let Ok(dev_out) = Command::new("ykman").arg("info").output() {
            let s = String::from_utf8_lossy(&dev_out.stdout);
            if s.contains("Bio") {
                info.bio_supported = true;
            }
        }
    }

    info
}

fn parse_retries(line: &str) -> Option<u32> {
    // Looks for: "with X attempt(s) remaining"
    if let Some(idx) = line.find("attempt(s) remaining") {
        let prefix = &line[..idx].trim();
        if let Some(space_idx) = prefix.rfind(' ') {
            let num_str = &prefix[space_idx + 1..];
            return num_str.parse::<u32>().ok();
        }
    }
    None
}

/// Lists all fingerprints stored on the security key.
pub fn list_fingerprints(pin: &str) -> Result<Vec<Fingerprint>, String> {
    let output = Command::new("ykman")
        .args(["fido", "fingerprints", "list", "-P", pin])
        .output()
        .map_err(|e| format!("Failed to execute ykman: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let out = String::from_utf8_lossy(&output.stdout);
        let msg = if !err.trim().is_empty() { err } else { out };
        return Err(msg.trim().to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut prints = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.starts_with("ID:") {
            let rest = line.trim_start_matches("ID:").trim();
            // Format is: "<hex_id> (<name>)" or "<hex_id>"
            if let Some(paren_start) = rest.find('(') {
                let id = rest[..paren_start].trim().to_string();
                let name = rest[paren_start + 1..]
                    .trim_end_matches(')')
                    .trim()
                    .to_string();
                prints.push(Fingerprint { id, name });
            } else {
                prints.push(Fingerprint {
                    id: rest.to_string(),
                    name: "Unnamed Fingerprint".to_string(),
                });
            }
        }
    }

    Ok(prints)
}

/// Enrolls a new fingerprint with interactive sensor touch feedback.
pub fn enroll_fingerprint(name: &str, pin: &str) -> Result<(), String> {
    println!("{}", "\n🧬 Enrolling New Fingerprint".bold().green());
    println!("Label: {}", name.cyan());
    println!("👉 Follow the on-screen prompts and touch your sensor repeatedly until complete.\n");

    let status = Command::new("ykman")
        .args(["fido", "fingerprints", "add", name, "-P", pin])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| format!("Failed to run ykman: {}", e))?;

    if status.success() {
        println!("\n{} Fingerprint '{}' successfully enrolled!", "✅ Success:".green(), name.bold());
        Ok(())
    } else {
        Err("Fingerprint enrollment failed or timed out.".to_string())
    }
}

/// Renames a fingerprint template by ID.
pub fn rename_fingerprint(id: &str, new_name: &str, pin: &str) -> Result<(), String> {
    let output = Command::new("ykman")
        .args(["fido", "fingerprints", "rename", id, new_name, "-P", pin])
        .output()
        .map_err(|e| format!("Failed to execute ykman: {}", e))?;

    if output.status.success() {
        println!("{} Fingerprint renamed to '{}'", "✅ Success:".green(), new_name.cyan());
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(err.trim().to_string())
    }
}

/// Deletes a fingerprint template by ID.
pub fn delete_fingerprint(id: &str, pin: &str) -> Result<(), String> {
    let output = Command::new("ykman")
        .args(["fido", "fingerprints", "delete", id, "-P", pin, "-f"])
        .output()
        .map_err(|e| format!("Failed to execute ykman: {}", e))?;

    if output.status.success() {
        println!("{} Fingerprint {} deleted.", "✅ Success:".green(), id.cyan());
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(err.trim().to_string())
    }
}

/// Sets or changes the FIDO2 hardware PIN.
pub fn change_fido_pin(old_pin: Option<&str>, new_pin: &str) -> Result<(), String> {
    let mut cmd = Command::new("ykman");
    cmd.args(["fido", "access", "change-pin", "-n", new_pin]);

    if let Some(old) = old_pin {
        cmd.args(["-P", old]);
    }

    let output = cmd.output().map_err(|e| format!("Failed to execute ykman: {}", e))?;

    if output.status.success() {
        println!("{} FIDO2 PIN updated successfully!", "✅ Success:".green());
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        let out = String::from_utf8_lossy(&output.stdout);
        let msg = if !err.trim().is_empty() { err } else { out };
        Err(msg.trim().to_string())
    }
}

/// Verifies if a given PIN is correct.
pub fn verify_fido_pin(pin: &str) -> Result<(), String> {
    let output = Command::new("ykman")
        .args(["fido", "access", "verify-pin", "-P", pin])
        .output()
        .map_err(|e| format!("Failed to execute ykman: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        let out = String::from_utf8_lossy(&output.stdout);
        let msg = if !err.trim().is_empty() { err } else { out };
        Err(msg.trim().to_string())
    }
}

/// Summarizes biometric status for pulsarkey status command.
pub fn check_bio_status() -> String {
    let info = get_fido_info();
    if !info.device_detected {
        return "Not detected".red().to_string();
    }

    if !info.bio_supported {
        return "Touch Only (No biometric sensor detected)".yellow().to_string();
    }

    let prints_str = if info.fingerprints_registered {
        format!("Enrolled ({} attempt(s) remaining)", info.bio_retries.unwrap_or(3))
    } else {
        "None enrolled".yellow().to_string()
    };

    let pin_str = if info.pin_set {
        format!("Set ({} attempt(s) remaining)", info.pin_retries.unwrap_or(8))
    } else {
        "Not Set".red().to_string()
    };

    format!("Biometric Sensor Active [Prints: {}, PIN: {}]", prints_str, pin_str)
}

/// Interactive Biometric & Fingerprint Manager dashboard.
pub fn run_interactive_bio(mut cached_pin: Option<String>) {
    println!("{}", "==================================================".cyan());
    println!("{}", "🧬 PulsarKey Native Biometric & Fingerprint Manager".bold().cyan());
    println!("{}", "==================================================".cyan());

    let fido_info = get_fido_info();
    if !fido_info.device_detected {
        println!("{} No YubiKey / FIDO2 device detected. Please insert your key.", "❌ Error:".red());
        return;
    }

    println!("Hardware Status:");
    println!("  Biometric Sensor:   {}", if fido_info.bio_supported { "Supported (YubiKey Bio)".green() } else { "Touch Only / Not Supported".yellow() });
    println!("  FIDO2 PIN:          {}", if fido_info.pin_set { format!("Set ({} retries remaining)", fido_info.pin_retries.unwrap_or(8)).green() } else { "Not Set".red() });
    println!("  Enrolled Prints:    {}", if fido_info.fingerprints_registered { format!("Registered ({} retries remaining)", fido_info.bio_retries.unwrap_or(3)).green() } else { "None".yellow() });
    println!("  Always Require UV:  {}", if fido_info.always_uv { "Enabled".green() } else { "Disabled".yellow() });

    if !fido_info.pin_set {
        println!("\n{} A FIDO2 PIN is required before managing fingerprints.", "⚠️ Note:".yellow());
        print!("❓ Would you like to set a FIDO2 PIN now? [y/N]: ");
        io::stdout().flush().unwrap();
        let mut choice = String::new();
        io::stdin().read_line(&mut choice).unwrap();
        if choice.trim().eq_ignore_ascii_case("y") {
            if let Ok(new_pin) = prompt_new_pin() {
                if let Err(e) = change_fido_pin(None, &new_pin) {
                    println!("{} Failed to set PIN: {}", "❌ Error:".red(), e);
                    return;
                }
                cached_pin = Some(new_pin);
            } else {
                return;
            }
        } else {
            return;
        }
    }

    // Authenticate PIN once for this interactive session
    let pin = match cached_pin {
        Some(p) => p,
        None => {
            println!("\n🔐 Enter your FIDO2 PIN to unlock biometric management:");
            match prompt_pin("PIN: ") {
                Ok(p) => {
                    if p.is_empty() {
                        println!("Operation cancelled.");
                        return;
                    }
                    print!("Verifying PIN... ");
                    io::stdout().flush().unwrap();
                    if let Err(e) = verify_fido_pin(&p) {
                        println!("{}\n{} {}", "Failed".red(), "❌ Error:".red(), e);
                        return;
                    }
                    println!("{}", "OK".green());
                    p
                }
                Err(e) => {
                    println!("Failed to read PIN: {}", e);
                    return;
                }
            }
        }
    };

    // Main interactive loop
    loop {
        println!("\n{}", "──────────────────────────────────────────────────".blue());
        println!("{}", "📋 Enrolled Fingerprints on Token:".bold());
        let prints = match list_fingerprints(&pin) {
            Ok(p) => p,
            Err(e) => {
                println!("{} Failed to fetch fingerprints: {}", "❌ Error:".red(), e);
                break;
            }
        };

        if prints.is_empty() {
            println!("  {}", "(No fingerprints enrolled yet)".dimmed());
        } else {
            for (idx, fp) in prints.iter().enumerate() {
                println!(
                    "  [{}] {}  {} (ID: {})",
                    idx + 1,
                    "👆".cyan(),
                    fp.name.bold(),
                    fp.id.dimmed()
                );
            }
        }

        println!("\n{}", "Actions:".bold());
        println!("  [1] ➕ Enroll New Fingerprint");
        println!("  [2] 🏷️ Rename Fingerprint");
        println!("  [3] 🗑️ Delete Fingerprint");
        println!("  [4] 🔑 Change FIDO2 PIN");
        println!("  [5] 🔄 Refresh List");
        println!("  [6] 🚪 Exit");
        print!("\nSelect option [1-6]: ");
        io::stdout().flush().unwrap();

        let mut opt = String::new();
        if io::stdin().read_line(&mut opt).is_err() {
            break;
        }

        match opt.trim() {
            "1" | "add" | "enroll" => {
                print!("\nEnter a friendly name for this fingerprint [e.g. Right Index]: ");
                io::stdout().flush().unwrap();
                let mut name = String::new();
                io::stdin().read_line(&mut name).unwrap();
                let mut name = name.trim().to_string();
                if name.is_empty() {
                    name = format!("Fingerprint {}", prints.len() + 1);
                }
                if name.len() > 15 {
                    println!("{} Label truncated to 15 characters.", "⚠️ Warning:".yellow());
                    name.truncate(15);
                }
                if let Err(e) = enroll_fingerprint(&name, &pin) {
                    println!("{} {}", "❌ Error:".red(), e);
                }
            }
            "2" | "rename" => {
                if prints.is_empty() {
                    println!("No fingerprints to rename.");
                    continue;
                }
                print!("Enter fingerprint number to rename [1-{}]: ", prints.len());
                io::stdout().flush().unwrap();
                let mut num_str = String::new();
                io::stdin().read_line(&mut num_str).unwrap();
                if let Ok(idx) = num_str.trim().parse::<usize>() {
                    if idx >= 1 && idx <= prints.len() {
                        let fp = &prints[idx - 1];
                        print!("Enter new name for '{}' (max 15 chars): ", fp.name);
                        io::stdout().flush().unwrap();
                        let mut new_name = String::new();
                        io::stdin().read_line(&mut new_name).unwrap();
                        let mut new_name = new_name.trim().to_string();
                        if !new_name.is_empty() {
                            if new_name.len() > 15 {
                                new_name.truncate(15);
                            }
                            if let Err(e) = rename_fingerprint(&fp.id, &new_name, &pin) {
                                println!("{} {}", "❌ Error:".red(), e);
                            }
                        }
                    } else {
                        println!("Invalid selection.");
                    }
                }
            }
            "3" | "delete" | "remove" => {
                if prints.is_empty() {
                    println!("No fingerprints to delete.");
                    continue;
                }
                print!("Enter fingerprint number to delete [1-{}]: ", prints.len());
                io::stdout().flush().unwrap();
                let mut num_str = String::new();
                io::stdin().read_line(&mut num_str).unwrap();
                if let Ok(idx) = num_str.trim().parse::<usize>() {
                    if idx >= 1 && idx <= prints.len() {
                        let fp = &prints[idx - 1];
                        print!("Are you sure you want to delete '{}'? [y/N]: ", fp.name);
                        io::stdout().flush().unwrap();
                        let mut confirm = String::new();
                        io::stdin().read_line(&mut confirm).unwrap();
                        if confirm.trim().eq_ignore_ascii_case("y") {
                            if let Err(e) = delete_fingerprint(&fp.id, &pin) {
                                println!("{} {}", "❌ Error:".red(), e);
                            }
                        }
                    } else {
                        println!("Invalid selection.");
                    }
                }
            }
            "4" | "pin" => {
                println!("\n🔑 Changing FIDO2 PIN...");
                if let Ok(new_pin) = prompt_new_pin() {
                    if let Err(e) = change_fido_pin(Some(&pin), &new_pin) {
                        println!("{} {}", "❌ Error:".red(), e);
                    } else {
                        println!("PIN changed. Please re-run pulsarkey bio to authenticate with your new PIN.");
                        break;
                    }
                }
            }
            "5" | "refresh" => {
                continue;
            }
            "6" | "quit" | "exit" | "q" => {
                println!("Exiting Biometric Manager. Goodbye!");
                break;
            }
            _ => {
                println!("Unknown option. Please choose 1 to 6.");
            }
        }
    }
}

fn prompt_new_pin() -> io::Result<String> {
    println!("The FIDO2 PIN must be at least 4 characters long (letters, numbers, symbols).");
    let p1 = prompt_pin("Enter new PIN: ")?;
    if p1.len() < 4 {
        println!("{} PIN is too short (minimum 4 characters).", "❌ Error:".red());
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "PIN too short"));
    }
    let p2 = prompt_pin("Confirm new PIN: ")?;
    if p1 != p2 {
        println!("{} PINs do not match.", "❌ Error:".red());
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "PIN mismatch"));
    }
    Ok(p1)
}

/// Handles CLI subcommands for pulsarkey bio.
pub fn handle_bio_cli(
    cmd: Option<crate::BioCommands>,
) {
    match cmd {
        None => {
            run_interactive_bio(None);
        }
        Some(crate::BioCommands::List { pin }) => {
            let p = match pin {
                Some(p) => p,
                None => match prompt_pin("Enter FIDO2 PIN: ") {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        return;
                    }
                },
            };
            match list_fingerprints(&p) {
                Ok(prints) => {
                    println!("{}", "Enrolled Fingerprints:".bold());
                    if prints.is_empty() {
                        println!("  (No fingerprints enrolled)");
                    } else {
                        for fp in prints {
                            println!("  {}  {} (ID: {})", "👆".cyan(), fp.name.bold(), fp.id);
                        }
                    }
                }
                Err(e) => eprintln!("{} {}", "❌ Error:".red(), e),
            }
        }
        Some(crate::BioCommands::Add { name, pin }) => {
            let p = match pin {
                Some(p) => p,
                None => match prompt_pin("Enter FIDO2 PIN: ") {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        return;
                    }
                },
            };
            let label = name.unwrap_or_else(|| {
                print!("Fingerprint name [e.g. Right Index]: ");
                io::stdout().flush().unwrap();
                let mut buf = String::new();
                io::stdin().read_line(&mut buf).unwrap();
                let trimmed = buf.trim().to_string();
                if trimmed.is_empty() { "Fingerprint".to_string() } else { trimmed }
            });
            if let Err(e) = enroll_fingerprint(&label, &p) {
                eprintln!("{} {}", "❌ Error:".red(), e);
            }
        }
        Some(crate::BioCommands::Delete { id, pin, force }) => {
            let p = match pin {
                Some(p) => p,
                None => match prompt_pin("Enter FIDO2 PIN: ") {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        return;
                    }
                },
            };
            let target_id = match id {
                Some(i) => i,
                None => {
                    eprintln!("{} Please provide fingerprint ID to delete.", "❌ Error:".red());
                    return;
                }
            };
            if !force {
                print!("Are you sure you want to delete fingerprint {}? [y/N]: ", target_id);
                io::stdout().flush().unwrap();
                let mut confirm = String::new();
                io::stdin().read_line(&mut confirm).unwrap();
                if !confirm.trim().eq_ignore_ascii_case("y") {
                    println!("Aborted.");
                    return;
                }
            }
            if let Err(e) = delete_fingerprint(&target_id, &p) {
                eprintln!("{} {}", "❌ Error:".red(), e);
            }
        }
        Some(crate::BioCommands::Rename { id, name, pin }) => {
            let p = match pin {
                Some(p) => p,
                None => match prompt_pin("Enter FIDO2 PIN: ") {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        return;
                    }
                },
            };
            let target_id = match id {
                Some(i) => i,
                None => {
                    eprintln!("{} Please provide fingerprint ID to rename.", "❌ Error:".red());
                    return;
                }
            };
            let new_label = match name {
                Some(n) => n,
                None => {
                    print!("New name (max 15 chars): ");
                    io::stdout().flush().unwrap();
                    let mut buf = String::new();
                    io::stdin().read_line(&mut buf).unwrap();
                    buf.trim().to_string()
                }
            };
            if let Err(e) = rename_fingerprint(&target_id, &new_label, &p) {
                eprintln!("{} {}", "❌ Error:".red(), e);
            }
        }
        Some(crate::BioCommands::Pin { action }) => {
            handle_pin_cli(action);
        }
    }
}

/// Handles CLI subcommands for pulsarkey pin.
pub fn handle_pin_cli(action: Option<String>) {
    let act = action.as_deref().unwrap_or("status");
    match act {
        "status" => {
            let info = get_fido_info();
            println!("{}", "FIDO2 Hardware PIN Status:".bold());
            println!("  PIN Set:      {}", if info.pin_set { "Yes".green() } else { "No".red() });
            println!("  Retries Left: {}", info.pin_retries.map(|r| r.to_string()).unwrap_or_else(|| "N/A".to_string()).cyan());
        }
        "change" => {
            let old = match prompt_pin("Enter current FIDO2 PIN: ") {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return;
                }
            };
            if let Ok(new_pin) = prompt_new_pin() {
                if let Err(e) = change_fido_pin(Some(&old), &new_pin) {
                    eprintln!("{} {}", "❌ Error:".red(), e);
                }
            }
        }
        "set" => {
            if let Ok(new_pin) = prompt_new_pin() {
                if let Err(e) = change_fido_pin(None, &new_pin) {
                    eprintln!("{} {}", "❌ Error:".red(), e);
                }
            }
        }
        "verify" => {
            let p = match prompt_pin("Enter FIDO2 PIN to verify: ") {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return;
                }
            };
            match verify_fido_pin(&p) {
                Ok(_) => println!("{} FIDO2 PIN verified successfully!", "✅ Success:".green()),
                Err(e) => eprintln!("{} Verification failed: {}", "❌ Error:".red(), e),
            }
        }
        other => {
            eprintln!("Unknown PIN action: '{}'. Valid actions: status, change, set, verify", other);
        }
    }
}
