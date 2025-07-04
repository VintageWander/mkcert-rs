use std::{fs::File, io::Read, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::Error;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub common_name: Option<String>,
    pub locality: Option<String>,
    pub country: Option<String>,
    pub org_unit: Option<String>,
    pub org_name: Option<String>,
    pub thumbprint: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            common_name: Some("Mkcert Development CA".into()),
            locality: Some("San Francisco".into()),
            country: Some("US".into()),
            org_unit: Some("Development".into()),
            org_name: Some("mkcert-rs".into()),
            thumbprint: None,
        }
    }
}

impl Config {
    pub fn write_config(config: &Config) -> Result<(), Error> {
        let config_path = get_config_path()?;
        let config_str = serde_json::to_string_pretty(config)?;
        std::fs::write(config_path.join("config.json"), config_str)?;

        Ok(())
    }

    pub fn read_config() -> Result<Config, Error> {
        let config_path = get_config_path()?;
        let file_path = config_path.join("config.json");
        if !file_path.exists() {
            let config = Config::default();
            Config::write_config(&config)?;
            Ok(config)
        } else {
            let mut file = File::open(file_path)?;
            let mut config_str = String::new();
            file.read_to_string(&mut config_str)?;
            Ok(serde_json::from_str(&config_str)?)
        }
    }
}

pub fn get_config_path() -> Result<PathBuf, Error> {
    let config_path = dirs::home_dir().ok_or(Error::NoHomeDir)?.join("mkcert-rs");
    std::fs::create_dir_all(&config_path).ok();
    Ok(config_path)
}
