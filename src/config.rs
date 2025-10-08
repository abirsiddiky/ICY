use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const CONFIG_DIR: &str = "/etc/icy/configs";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub name: String,
    pub path: String,
    pub snapshot_dir: String,
    pub retention: RetentionPolicy,
    #[serde(default)]
    pub fs_type: FilesystemType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FilesystemType {
    Btrfs,
    Lvm,
    Auto,
}

impl Default for FilesystemType {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    #[serde(default = "default_hourly")]
    pub hourly: usize,
    #[serde(default = "default_daily")]
    pub daily: usize,
    #[serde(default = "default_weekly")]
    pub weekly: usize,
    #[serde(default = "default_monthly")]
    pub monthly: usize,
}

fn default_hourly() -> usize {
    0
}
fn default_daily() -> usize {
    7
}
fn default_weekly() -> usize {
    4
}
fn default_monthly() -> usize {
    3
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            hourly: default_hourly(),
            daily: default_daily(),
            weekly: default_weekly(),
            monthly: default_monthly(),
        }
    }
}

pub struct ConfigManager {
    config_dir: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Result<Self> {
        let config_dir = PathBuf::from(CONFIG_DIR);

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .context("Failed to create config directory")?;
        }

        Ok(Self { config_dir })
    }

    pub fn load_config(&self, name: &str) -> Result<Config> {
        let config_path = self.config_dir.join(format!("{}.yaml", name));

        if !config_path.exists() {
            anyhow::bail!("Configuration '{}' not found. Use 'icy init' to create it.", name);
        }

        let content = fs::read_to_string(&config_path)
            .context(format!("Failed to read config file: {:?}", config_path))?;

        let config: Config = serde_yaml::from_str(&content)
            .context("Failed to parse config file")?;

        Ok(config)
    }

    pub fn save_config(&self, config: &Config) -> Result<()> {
        let config_path = self.config_dir.join(format!("{}.yaml", config.name));

        let yaml = serde_yaml::to_string(config)
            .context("Failed to serialize config")?;

        fs::write(&config_path, yaml)
            .context(format!("Failed to write config file: {:?}", config_path))?;

        Ok(())
    }

    pub fn list_configs(&self) -> Result<Vec<Config>> {
        let mut configs = Vec::new();

        if !self.config_dir.exists() {
            return Ok(configs);
        }

        for entry in fs::read_dir(&self.config_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    match self.load_config(name) {
                        Ok(config) => configs.push(config),
                        Err(e) => log::warn!("Failed to load config {}: {}", name, e),
                    }
                }
            }
        }

        configs.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(configs)
    }

    pub fn create_config(&self, name: &str, path: &str, snapshot_dir: &str) -> Result<()> {
        let config = Config {
            name: name.to_string(),
            path: path.to_string(),
            snapshot_dir: snapshot_dir.to_string(),
            retention: RetentionPolicy::default(),
            fs_type: FilesystemType::Auto,
        };

        // Create snapshot directory
        let snap_path = Path::new(snapshot_dir);
        if !snap_path.exists() {
            fs::create_dir_all(snap_path)
                .context("Failed to create snapshot directory")?;
        }

        self.save_config(&config)?;
        Ok(())
    }

    pub fn delete_config(&self, name: &str) -> Result<()> {
        let config_path = self.config_dir.join(format!("{}.yaml", name));

        if config_path.exists() {
            fs::remove_file(&config_path)
                .context(format!("Failed to delete config file: {:?}", config_path))?;
        }

        Ok(())
    }
}
