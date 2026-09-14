use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, Default)]
pub struct PulsarConfig {
    pub autolock: bool,
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
        let autolock = content.contains("\"autolock\": true");
        PulsarConfig { autolock }
    } else {
        PulsarConfig::default()
    }
}

pub fn save_config(config: &PulsarConfig) -> Result<(), std::io::Error> {
    let path = get_config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = format!("{{\n  \"autolock\": {}\n}}\n", config.autolock);
    fs::write(path, json)
}
