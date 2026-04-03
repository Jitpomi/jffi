use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub package: PackageConfig,
    pub platforms: PlatformsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageConfig {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformsConfig {
    pub enabled: Vec<String>,
    #[serde(default)]
    pub ios: IosConfig,
    #[serde(default)]
    pub android: AndroidConfig,
    #[serde(default)]
    pub macos: MacosConfig,
    #[serde(default)]
    pub windows: WindowsConfig,
    #[serde(default)]
    pub linux: LinuxConfig,
    #[serde(default)]
    pub web: WebConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IosConfig {
    #[serde(default = "default_ios_deployment_target")]
    pub deployment_target: String,
    pub bundle_id: String,
}

impl Default for IosConfig {
    fn default() -> Self {
        Self {
            deployment_target: "16.0".to_string(),
            bundle_id: "com.example.app".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AndroidConfig {
    #[serde(default = "default_android_min_sdk")]
    pub min_sdk: u32,
    pub package: String,
}

impl Default for AndroidConfig {
    fn default() -> Self {
        Self {
            min_sdk: 26,
            package: "com.example.app".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacosConfig {
    #[serde(default = "default_macos_deployment_target")]
    pub deployment_target: String,
}

impl Default for MacosConfig {
    fn default() -> Self {
        Self {
            deployment_target: "13.0".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsConfig {
    #[serde(default = "default_windows_min_version")]
    pub min_version: String,
}

impl Default for WindowsConfig {
    fn default() -> Self {
        Self {
            min_version: "10.0.19041.0".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxConfig {
    #[serde(default = "default_gtk_version")]
    pub gtk_version: String,
}

impl Default for LinuxConfig {
    fn default() -> Self {
        Self {
            gtk_version: "4.0".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    #[serde(default = "default_web_target")]
    pub target: String,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            target: "es2020".to_string(),
        }
    }
}

fn default_ios_deployment_target() -> String {
    "16.0".to_string()
}

fn default_android_min_sdk() -> u32 {
    26
}

fn default_macos_deployment_target() -> String {
    "13.0".to_string()
}

fn default_windows_min_version() -> String {
    "10.0.19041.0".to_string()
}

fn default_gtk_version() -> String {
    "4.0".to_string()
}

fn default_web_target() -> String {
    "es2020".to_string()
}

pub fn load_config() -> Result<Config> {
    let config_path = "jffi.toml";
    let contents = fs::read_to_string(config_path)
        .context("Failed to read jffi.toml. Are you in a project directory?")?;
    
    let config: Config = toml::from_str(&contents)
        .context("Failed to parse jffi.toml")?;
    
    Ok(config)
}

pub fn save_config(config: &Config) -> Result<()> {
    let config_path = "jffi.toml";
    let contents = toml::to_string_pretty(config)
        .context("Failed to serialize config")?;
    
    fs::write(config_path, contents)
        .context("Failed to write jffi.toml")?;
    
    Ok(())
}
