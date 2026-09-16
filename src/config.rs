use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct PulsarConfig {
    pub autolock: bool,
    pub profile: String,
}

impl Default for PulsarConfig {
    fn default() -> Self {
        Self {
            autolock: true,
            profile: "convenience".to_string(),
        }
    }
}

pub fn get_config_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("SUDO_USER").map(|u| format!("/home/{}", u)))
        .unwrap_or_else(|_| "/home/mzia".to_string());
    PathBuf::from(home).join(".config/pulsarkey/config.json")
}

pub fn load_config() -> PulsarConfig {
    let path = get_config_path();
    if let Ok(content) = fs::read_to_string(&path) {
        let autolock = !content.contains("\"autolock\": false");
        let profile = if content.contains("\"profile\": \"fortress\"") {
            "fortress".to_string()
        } else if content.contains("\"profile\": \"lockdown\"") {
            "lockdown".to_string()
        } else {
            "convenience".to_string()
        };
        PulsarConfig { autolock, profile }
    } else {
        PulsarConfig::default()
    }
}

pub fn save_config(config: &PulsarConfig) -> Result<(), std::io::Error> {
    let path = get_config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = format!(
        "{{\n  \"autolock\": {},\n  \"profile\": \"{}\"\n}}\n",
        config.autolock, config.profile
    );
    fs::write(path, json)
}
