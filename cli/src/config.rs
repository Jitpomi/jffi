use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub package: PackageConfig,
    pub platforms: PlatformsConfig,
    pub bundle: Option<BundleConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageConfig {
    pub name: String,
    pub version: String,
    pub version_code: Option<u32>,
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
    pub app_groups: Option<Vec<String>>,
}

impl Default for IosConfig {
    fn default() -> Self {
        Self {
            deployment_target: "16.0".to_string(),
            bundle_id: "com.example.app".to_string(),
            app_groups: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AndroidConfig {
    #[serde(default = "default_android_min_sdk")]
    pub min_sdk: u32,
    pub package: String,
    pub target_sdk: Option<u32>,
    pub abis: Option<Vec<String>>,
    pub rustflags: Option<String>,
    #[serde(default = "default_false")]
    pub obfuscate: bool,
    #[serde(default = "default_false")]
    pub shrink_resources: bool,
}

impl Default for AndroidConfig {
    fn default() -> Self {
        Self {
            min_sdk: 26,
            package: "com.example.app".to_string(),
            target_sdk: None,
            abis: None,
            rustflags: None,
            obfuscate: false,
            shrink_resources: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacosConfig {
    #[serde(default = "default_macos_deployment_target")]
    pub deployment_target: String,
    pub rustflags: Option<String>,
    pub app_groups: Option<Vec<String>>,
}

impl Default for MacosConfig {
    fn default() -> Self {
        Self {
            deployment_target: "13.0".to_string(),
            rustflags: None,
            app_groups: None,
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
    #[serde(default = "default_web_port")]
    pub port: u16,
    #[serde(default = "default_false")]
    pub host: bool,
    #[serde(default = "default_false")]
    pub https: bool,
    #[serde(default = "default_false")]
    pub open: bool,
    #[serde(default = "default_true")]
    pub cors: bool,
    // SEO & meta tags
    pub title: Option<String>,
    pub description: Option<String>,
    #[serde(default = "default_web_lang")]
    pub lang: String,
    pub theme_color: Option<String>,
    pub favicon: Option<String>,
    pub og_image: Option<String>,
    pub og_url: Option<String>,
    pub og_type: Option<String>,
    pub twitter_card: Option<String>,
    pub keywords: Option<String>,
    pub author: Option<String>,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            target: "es2020".to_string(),
            port: 5173,
            host: false,
            https: false,
            open: false,
            cors: true,
            title: None,
            description: None,
            lang: "en".to_string(),
            theme_color: None,
            favicon: None,
            og_image: None,
            og_url: None,
            og_type: None,
            twitter_card: None,
            keywords: None,
            author: None,
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

fn default_web_port() -> u16 {
    5173
}

fn default_web_lang() -> String {
    "en".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BundleConfig {
    pub name: Option<String>,
    pub identifier: Option<String>,
    pub version: Option<String>,
    pub display_version: Option<String>,
    pub build_number: Option<u32>,
    pub resources: Option<Vec<String>>,
    pub license_file: Option<String>,
    pub homepage: Option<String>,
    pub icons: Option<BundleIconsConfig>,
    pub macos: Option<BundleMacosConfig>,
    pub ios: Option<BundleIosConfig>,
    pub android: Option<BundleAndroidConfig>,
    pub windows: Option<BundleWindowsConfig>,
    pub linux: Option<BundleLinuxConfig>,
    pub web: Option<BundleWebConfig>,
    pub signing: Option<BundleSigningConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleIconsConfig {
    pub source: String,
    #[serde(default = "default_true")]
    pub generate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleMacosConfig {
    #[serde(default = "default_macos_formats")]
    pub formats: Vec<String>,
    #[serde(default = "default_macos_targets")]
    pub targets: Vec<String>,
    #[serde(default = "default_macos_deployment_target")]
    pub minimum_system_version: String,
    pub category: Option<String>,
    pub signing_identity: Option<String>,
    #[serde(default = "default_true")]
    pub hardened_runtime: bool,
    #[serde(default = "default_true")]
    pub notarize: bool,
    #[serde(default = "default_true")]
    pub staple: bool,
    pub icon: Option<String>,
    pub provisioning_profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleIosConfig {
    #[serde(default = "default_ios_format")]
    pub format: String,
    #[serde(default = "default_ios_destination")]
    pub destination: String,
    #[serde(default = "default_ios_export_method")]
    pub export_method: String,
    pub icon: Option<String>,
    pub provisioning_profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleAndroidConfig {
    #[serde(default = "default_android_formats")]
    pub formats: Vec<String>,
    #[serde(default = "default_android_abis")]
    pub abis: Vec<String>,
    #[serde(default = "default_android_min_sdk_bundle")]
    pub min_sdk: u32,
    #[serde(default = "default_android_compile_sdk")]
    pub compile_sdk: u32,
    #[serde(default = "default_android_build_type")]
    pub build_type: String,
    #[serde(default = "default_true")]
    pub split_debug_symbols: bool,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleWindowsConfig {
    #[serde(default = "default_windows_formats")]
    pub formats: Vec<String>,
    pub identity_name: Option<String>,
    pub publisher_id: Option<String>,
    #[serde(default = "default_windows_targets")]
    pub targets: Vec<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleLinuxConfig {
    #[serde(default = "default_linux_formats")]
    pub formats: Vec<String>,
    pub app_id: Option<String>,
    #[serde(default = "default_linux_runtime")]
    pub runtime: String,
    #[serde(default = "default_linux_runtime_version")]
    pub runtime_version: String,
    #[serde(default = "default_linux_sdk")]
    pub sdk: String,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleWebConfig {
    #[serde(default = "default_web_dist_dir")]
    pub dist_dir: String,
    #[serde(default = "default_web_build_command")]
    pub build_command: String,
    #[serde(default = "default_true")]
    pub wasm_opt: bool,
    #[serde(default = "default_web_base_path")]
    pub base_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleSigningConfig {
    #[serde(default = "default_signing_profile")]
    pub profile: String,
    pub profiles: Option<std::collections::HashMap<String, SigningProfile>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningProfile {
    pub apple: Option<AppleSigningProfile>,
    pub android: Option<AndroidSigningProfile>,
    pub windows: Option<WindowsSigningProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsSigningProfile {
    pub certificate_thumbprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppleSigningProfile {
    pub method: Option<String>,
    pub team_id: Option<String>,
    pub entitlements: Option<String>,
    pub signing_certificate: Option<String>,
    pub installer_signing_certificate: Option<String>,
    
    // Notarytool Authentication
    pub apple_id: Option<String>,
    pub apple_id_env: Option<String>,
    pub app_specific_password_env: Option<String>,
    pub api_key_path: Option<String>,
    pub api_key_id: Option<String>,
    pub api_key_issuer_id: Option<String>,
    
    // Profile-level overrides
    pub notarize: Option<bool>,
    pub formats: Option<Vec<String>>,
    pub provisioning_profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AndroidSigningProfile {
    pub keystore_path: Option<String>,
    pub key_alias: Option<String>,
    pub store_password_env: Option<String>,
    pub key_password_env: Option<String>,
}

fn default_true() -> bool { true }
fn default_false() -> bool { false }
fn default_macos_formats() -> Vec<String> { vec!["app".to_string(), "dmg".to_string()] }
fn default_macos_targets() -> Vec<String> { vec!["aarch64-apple-darwin".to_string(), "x86_64-apple-darwin".to_string()] }
fn default_ios_format() -> String { "ipa".to_string() }
fn default_ios_destination() -> String { "generic/platform=iOS".to_string() }
fn default_ios_export_method() -> String { "app-store".to_string() }
fn default_android_formats() -> Vec<String> { vec!["aab".to_string(), "apk".to_string()] }
fn default_android_abis() -> Vec<String> { vec!["arm64-v8a".to_string(), "armeabi-v7a".to_string(), "x86_64".to_string()] }
fn default_android_min_sdk_bundle() -> u32 { 23 }
fn default_android_compile_sdk() -> u32 { 36 }
fn default_android_build_type() -> String { "release".to_string() }
fn default_windows_formats() -> Vec<String> { vec!["msixbundle".to_string(), "msi".to_string()] }
fn default_windows_targets() -> Vec<String> { vec!["x86_64-pc-windows-msvc".to_string(), "aarch64-pc-windows-msvc".to_string()] }
fn default_linux_formats() -> Vec<String> { vec!["flatpak".to_string(), "appimage".to_string(), "deb".to_string()] }
fn default_linux_runtime() -> String { "org.gnome.Platform".to_string() }
fn default_linux_runtime_version() -> String { "48".to_string() }
fn default_linux_sdk() -> String { "org.gnome.Sdk".to_string() }
fn default_web_dist_dir() -> String { "platforms/web/dist".to_string() }
fn default_web_build_command() -> String { "npm run build".to_string() }
fn default_web_base_path() -> String { "/".to_string() }
fn default_signing_profile() -> String { "release".to_string() }



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
