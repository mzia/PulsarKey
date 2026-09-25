use colored::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const GITHUB_REPO: &str = "mzia/PulsarKey";
const CACHE_VALIDITY_SECS: u64 = 3600; // 1 hour cache

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub is_update_available: bool,
    pub release_name: String,
    pub release_notes: String,
    pub html_url: String,
    pub assets: Vec<ReleaseAsset>,
    pub checked_at_epoch: u64,
}

#[derive(Deserialize)]
struct GitHubReleaseResponse {
    tag_name: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    body: Option<String>,
    html_url: String,
    #[serde(default)]
    assets: Vec<GitHubAssetResponse>,
}

#[derive(Deserialize)]
struct GitHubAssetResponse {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

/// Returns the cache file location: ~/.config/pulsarkey/update_info.json
pub fn get_update_cache_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("SUDO_USER").map(|u| {
            #[cfg(target_os = "macos")]
            { format!("/Users/{}", u) }
            #[cfg(not(target_os = "macos"))]
            { format!("/home/{}", u) }
        }))
        .unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".config/pulsarkey/update_info.json")
}

/// Parses semver numbers (major, minor, patch) from a version string (e.g. "v1.4.2" or "1.4.2")
pub fn parse_version(v: &str) -> (u32, u32, u32) {
    let clean = v.trim().trim_start_matches('v').trim_start_matches('V');
    let mut parts = clean.split('.');
    let major = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let minor = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let patch = parts.next().and_then(|s| s.split('-').next()).and_then(|s| s.parse().ok()).unwrap_or(0);
    (major, minor, patch)
}

/// Returns true if latest_ver is strictly newer than current_ver
pub fn is_newer(latest_ver: &str, current_ver: &str) -> bool {
    let (l_maj, l_min, l_pat) = parse_version(latest_ver);
    let (c_maj, c_min, c_pat) = parse_version(current_ver);
    (l_maj, l_min, l_pat) > (c_maj, c_min, c_pat)
}

/// Reads cached update info from disk without performing network operations
pub fn get_cached_update_info() -> Option<UpdateInfo> {
    let path = get_update_cache_path();
    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(info) = serde_json::from_str::<UpdateInfo>(&content) {
            let current_pkg = env!("CARGO_PKG_VERSION");
            // Validate that cached update still compares against the running binary
            let is_available = is_newer(&info.latest_version, current_pkg);
            let mut validated = info;
            validated.current_version = current_pkg.to_string();
            validated.is_update_available = is_available;
            return Some(validated);
        }
    }
    None
}

/// Saves update info to cache file
pub fn save_cached_update_info(info: &UpdateInfo) -> Result<(), std::io::Error> {
    let path = get_update_cache_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(info).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    fs::write(path, json)
}

/// Queries GitHub releases API for the latest release
pub fn fetch_latest_release(timeout_secs: u64) -> Result<UpdateInfo, String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", GITHUB_REPO);
    let timeout_str = timeout_secs.to_string();

    let output = Command::new("curl")
        .args([
            "-s",
            "-L",
            "--max-time",
            &timeout_str,
            "-H",
            "User-Agent: PulsarKey",
            "-H",
            "Accept: application/vnd.github.v3+json",
            &url,
        ])
        .output()
        .map_err(|e| format!("Failed to invoke curl: {}", e))?;

    if !output.status.success() {
        return Err(format!("curl request failed with exit code: {:?}", output.status.code()));
    }

    let body = String::from_utf8_lossy(&output.stdout);
    if body.trim().is_empty() {
        return Err("GitHub API returned an empty response".to_string());
    }

    let resp: GitHubReleaseResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse GitHub release JSON: {}", e))?;

    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let latest_version = resp.tag_name.trim().trim_start_matches('v').trim_start_matches('V').to_string();
    let is_update_available = is_newer(&resp.tag_name, &current_version);

    let assets: Vec<ReleaseAsset> = resp
        .assets
        .into_iter()
        .map(|a| ReleaseAsset {
            name: a.name,
            browser_download_url: a.browser_download_url,
            size: a.size,
        })
        .collect();

    let now_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let info = UpdateInfo {
        current_version,
        latest_version,
        is_update_available,
        release_name: resp.name.unwrap_or_else(|| resp.tag_name.clone()),
        release_notes: resp.body.unwrap_or_default(),
        html_url: resp.html_url,
        assets,
        checked_at_epoch: now_epoch,
    };

    let _ = save_cached_update_info(&info);
    Ok(info)
}

/// Checks for updates, using cached results if recent unless force is true
pub fn check_for_updates(force: bool) -> Result<UpdateInfo, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if !force {
        if let Some(cached) = get_cached_update_info() {
            if now.saturating_sub(cached.checked_at_epoch) < CACHE_VALIDITY_SECS {
                return Ok(cached);
            }
        }
    }

    fetch_latest_release(8)
}

/// Spawns an asynchronous background task to refresh update cache without blocking
pub fn spawn_background_update_check() {
    std::thread::spawn(|| {
        let _ = check_for_updates(false);
    });
}

/// Picks the best matching download asset for the current operating system
pub fn select_best_asset<'a>(assets: &'a [ReleaseAsset]) -> Option<&'a ReleaseAsset> {
    #[cfg(target_os = "macos")]
    {
        // On macOS, prefer .pkg then .dmg
        if let Some(pkg) = assets.iter().find(|a| a.name.ends_with(".pkg")) {
            return Some(pkg);
        }
        if let Some(dmg) = assets.iter().find(|a| a.name.ends_with(".dmg")) {
            return Some(dmg);
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        // On Pop!_OS / Linux, prefer .deb package
        if let Some(deb) = assets.iter().find(|a| a.name.ends_with("_amd64.deb") || a.name.ends_with(".deb")) {
            return Some(deb);
        }
    }

    // Fallback: any asset or None
    assets.first()
}

/// Performs interactive or automated download and installation of the new release
pub fn perform_update(info: &UpdateInfo, assume_yes: bool) -> Result<(), String> {
    println!("{}", "==================================================".cyan());
    println!("{}", "🚀 PulsarKey Application Updater".bold().cyan());
    println!("{}", "==================================================".cyan());
    println!("Current Version:  {}", info.current_version.yellow());
    println!("Latest Version:   {}", info.latest_version.green().bold());

    if !info.is_update_available {
        println!("\n{} PulsarKey is already up to date!", "✅".green());
        return Ok(());
    }

    println!("\nRelease: {}", info.release_name.bold());
    println!("Web URL: {}", info.html_url.cyan());

    if !info.release_notes.is_empty() {
        println!("\nRelease Notes:");
        let notes_lines: Vec<&str> = info.release_notes.lines().take(12).collect();
        for line in notes_lines {
            println!("  {}", line.dimmed());
        }
        if info.release_notes.lines().count() > 12 {
            println!("  ... [Full notes on GitHub]");
        }
    }

    // Check Homebrew installation on macOS
    #[cfg(target_os = "macos")]
    {
        if is_installed_via_homebrew() {
            println!("\n{} Detected Homebrew installation.", "🍺".green());
            if !assume_yes {
                print!("Upgrade PulsarKey via Homebrew? [Y/n]: ");
                use std::io::Write;
                let _ = std::io::stdout().flush();
                let mut input = String::new();
                let _ = std::io::stdin().read_line(&mut input);
                if input.trim().eq_ignore_ascii_case("n") {
                    println!("Update cancelled.");
                    return Ok(());
                }
            }
            println!("Running: brew upgrade pulsarkey...");
            let st = Command::new("brew").args(["upgrade", "pulsarkey"]).status();
            match st {
                Ok(s) if s.success() => {
                    println!("\n{} Successfully upgraded PulsarKey via Homebrew!", "🎉".green());
                    return Ok(());
                }
                _ => {
                    println!("{} Homebrew upgrade returned non-zero. Falling back to direct installer...", "⚠️".yellow());
                }
            }
        }
    }

    let asset = match select_best_asset(&info.assets) {
        Some(a) => a,
        None => {
            println!("\n{} No pre-compiled binary package was found for your OS.", "⚠️".yellow());
            println!("Please visit the release page to download or compile from source:");
            println!("  {}", info.html_url.cyan());
            let _ = open_in_browser(&info.html_url);
            return Ok(());
        }
    };

    let size_mb = (asset.size as f64) / (1024.0 * 1024.0);
    println!("\nFound installer package: {} ({:.1} MB)", asset.name.green().bold(), size_mb);

    if !assume_yes {
        print!("Do you want to download and install this update now? [Y/n]: ");
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let mut input = String::new();
        let _ = std::io::stdin().read_line(&mut input);
        if input.trim().eq_ignore_ascii_case("n") {
            println!("Update cancelled by user.");
            return Ok(());
        }
    }

    // Download to temporary path
    let tmp_dir = std::env::temp_dir();
    let download_target = tmp_dir.join(&asset.name);

    println!("\n📥 Downloading {}...", asset.name.cyan());
    let download_status = Command::new("curl")
        .args([
            "-L",
            "--progress-bar",
            "-o",
            &download_target.to_string_lossy(),
            &asset.browser_download_url,
        ])
        .status()
        .map_err(|e| format!("Failed to run curl: {}", e))?;

    if !download_status.success() {
        return Err("Download failed: curl returned non-zero exit status".to_string());
    }

    println!("{} Download complete: {}", "✅".green(), download_target.display());

    // Execute installation
    println!("⚙️  Installing update...");

    #[cfg(target_os = "macos")]
    {
        if asset.name.ends_with(".pkg") {
            println!("Running macOS package installer (may request administrator credentials)...");
            let status = Command::new("sudo")
                .args(["installer", "-pkg", &download_target.to_string_lossy(), "-target", "/"])
                .status();

            match status {
                Ok(s) if s.success() => {
                    println!("\n{} PulsarKey v{} was successfully installed!", "🎉".green().bold(), info.latest_version);
                    let _ = fs::remove_file(&download_target);
                    return Ok(());
                }
                _ => {
                    println!("{} Automated installer failed. Opening package in Finder...", "⚠️".yellow());
                    let _ = Command::new("open").arg(&download_target).status();
                    return Ok(());
                }
            }
        } else if asset.name.ends_with(".dmg") {
            println!("Opening disk image: {}...", download_target.display());
            let _ = Command::new("open").arg(&download_target).status();
            println!("\n{} Drag PulsarKey to your Applications folder to complete the update.", "👉".cyan());
            return Ok(());
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        if asset.name.ends_with(".deb") {
            println!("Installing Debian package with apt (may request sudo password)...");
            let deb_path = download_target.to_string_lossy();
            let status = Command::new("sudo")
                .args(["apt", "install", "-y", "--reinstall", &deb_path])
                .status();

            match status {
                Ok(s) if s.success() => {
                    println!("\n{} PulsarKey v{} was successfully installed!", "🎉".green().bold(), info.latest_version);
                    let _ = fs::remove_file(&download_target);
                    // Reload systemd daemon if service is installed
                    let _ = Command::new("systemctl").args(["--user", "restart", "pulsarkey-applet.service"]).status();
                    return Ok(());
                }
                _ => {
                    println!("{} apt install failed. Trying dpkg -i...", "⚠️".yellow());
                    let dpkg_status = Command::new("sudo")
                        .args(["dpkg", "-i", &deb_path])
                        .status();

                    if let Ok(ds) = dpkg_status {
                        if ds.success() {
                            println!("\n{} PulsarKey v{} was successfully installed via dpkg!", "🎉".green().bold(), info.latest_version);
                            let _ = fs::remove_file(&download_target);
                            return Ok(());
                        }
                    }
                    return Err(format!("Failed to install package. You can manually install it with: sudo apt install {}", download_target.display()));
                }
            }
        }
    }

    // Default fallback: open in browser
    println!("Opening release in browser for manual installation...");
    open_in_browser(&info.html_url)
}

pub fn open_in_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(not(target_os = "macos"))]
    let cmd = "xdg-open";

    let _ = Command::new(cmd).arg(url).spawn();
    Ok(())
}

/// Handles CLI `pulsarkey update` command
pub fn handle_update_cli(check: bool, force: bool, yes: bool) {
    println!("{}", "==================================================".cyan());
    println!("{}", "🚀 PulsarKey Software Updates".bold().cyan());
    println!("{}", "==================================================".cyan());
    println!("Checking GitHub repository for updates (mzia/PulsarKey)...");

    match check_for_updates(force) {
        Ok(info) => {
            println!("Current version: {}", info.current_version.cyan());
            println!("Latest version:  {}", info.latest_version.green().bold());

            if check {
                if info.is_update_available {
                    println!("\n{} Update available: v{} -> v{}", "🚀".green().bold(), info.current_version, info.latest_version.green().bold());
                    println!("Run 'pulsarkey update' to download and install this version.");
                } else {
                    println!("\n{} PulsarKey is up to date (v{}).", "✅".green(), info.current_version);
                }
            } else {
                if let Err(e) = perform_update(&info, yes) {
                    eprintln!("\n{} Update failed: {}", "❌".red(), e);
                }
            }
        }
        Err(e) => {
            eprintln!("\n{} Failed to check for updates: {}", "❌".red(), e);
            println!("You can check releases manually at: https://github.com/mzia/PulsarKey/releases");
        }
    }
}


#[cfg(target_os = "macos")]
fn is_installed_via_homebrew() -> bool {
    if let Ok(exe) = std::env::current_exe() {
        let path_str = exe.to_string_lossy();
        if path_str.contains("/opt/homebrew/") || path_str.contains("/usr/local/Cellar/") || path_str.contains("/usr/local/Caskroom/") {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("1.4.1"), (1, 4, 1));
        assert_eq!(parse_version("v1.4.2"), (1, 4, 2));
        assert_eq!(parse_version("V2.0.0-beta1"), (2, 0, 0));
        assert_eq!(parse_version("0.9"), (0, 9, 0));
    }

    #[test]
    fn test_is_newer() {
        assert!(is_newer("v1.4.2", "1.4.1"));
        assert!(is_newer("1.5.0", "1.4.1"));
        assert!(is_newer("2.0.0", "1.99.99"));
        assert!(!is_newer("1.4.1", "1.4.1"));
        assert!(!is_newer("1.4.0", "1.4.1"));
        assert!(!is_newer("v1.4.1", "1.4.1"));
    }

    #[test]
    fn test_asset_selection() {
        let assets = vec![
            ReleaseAsset {
                name: "PulsarKey-1.4.2.dmg".to_string(),
                browser_download_url: "https://example.com/dmg".to_string(),
                size: 1000,
            },
            ReleaseAsset {
                name: "PulsarKey-1.4.2.pkg".to_string(),
                browser_download_url: "https://example.com/pkg".to_string(),
                size: 2000,
            },
            ReleaseAsset {
                name: "pulsarkey_1.4.2_amd64.deb".to_string(),
                browser_download_url: "https://example.com/deb".to_string(),
                size: 3000,
            },
        ];

        let selected = select_best_asset(&assets);
        assert!(selected.is_some());
        #[cfg(target_os = "macos")]
        assert_eq!(selected.unwrap().name, "PulsarKey-1.4.2.pkg");
        #[cfg(target_os = "linux")]
        assert_eq!(selected.unwrap().name, "pulsarkey_1.4.2_amd64.deb");
    }
}
