use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub package: PackageConfig,
    pub platforms: PlatformsConfig,
    pub bundle: Option<BundleConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageConfig {
    pub name: String,
    pub version: String,
    pub version_code: Option<u32>,
    pub release_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
    #[serde(default = "default_false")]
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
            cors: false,
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

fn default_schema_version() -> u32 {
    1
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct BundleIconsConfig {
    pub source: String,
    #[serde(default = "default_true")]
    pub generate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct BundleSigningConfig {
    #[serde(default = "default_signing_profile")]
    pub profile: String,
    pub profiles: Option<std::collections::HashMap<String, SigningProfile>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SigningProfile {
    pub apple: Option<AppleSigningProfile>,
    pub android: Option<AndroidSigningProfile>,
    pub windows: Option<WindowsSigningProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowsSigningProfile {
    pub certificate_thumbprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    #[serde(default)]
    pub provisioning_profiles: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AndroidSigningProfile {
    pub keystore_path: Option<String>,
    pub key_alias: Option<String>,
    pub store_password_env: Option<String>,
    pub key_password_env: Option<String>,
}

fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}
fn default_macos_formats() -> Vec<String> {
    vec!["app".to_string(), "dmg".to_string()]
}
fn default_macos_targets() -> Vec<String> {
    vec![
        "aarch64-apple-darwin".to_string(),
        "x86_64-apple-darwin".to_string(),
    ]
}
fn default_ios_format() -> String {
    "ipa".to_string()
}
fn default_ios_destination() -> String {
    "generic/platform=iOS".to_string()
}
fn default_ios_export_method() -> String {
    "app-store-connect".to_string()
}
fn default_android_formats() -> Vec<String> {
    vec!["aab".to_string(), "apk".to_string()]
}
fn default_android_abis() -> Vec<String> {
    vec![
        "arm64-v8a".to_string(),
        "armeabi-v7a".to_string(),
        "x86_64".to_string(),
    ]
}
fn default_android_min_sdk_bundle() -> u32 {
    23
}
fn default_android_compile_sdk() -> u32 {
    36
}
fn default_android_build_type() -> String {
    "release".to_string()
}
fn default_windows_formats() -> Vec<String> {
    vec!["msix".to_string()]
}
fn default_windows_targets() -> Vec<String> {
    vec![
        "x86_64-pc-windows-msvc".to_string(),
        "aarch64-pc-windows-msvc".to_string(),
    ]
}
fn default_linux_formats() -> Vec<String> {
    vec!["flatpak".to_string()]
}
fn default_linux_runtime() -> String {
    "org.gnome.Platform".to_string()
}
fn default_linux_runtime_version() -> String {
    "50".to_string()
}
fn default_linux_sdk() -> String {
    "org.gnome.Sdk".to_string()
}
fn default_web_dist_dir() -> String {
    "platforms/web/dist".to_string()
}
fn default_web_build_command() -> String {
    "npm run build".to_string()
}
fn default_web_base_path() -> String {
    "/".to_string()
}
fn default_signing_profile() -> String {
    "release".to_string()
}

pub fn load_config() -> Result<Config> {
    let config_path = "jffi.toml";
    let contents = fs::read_to_string(config_path)
        .context("Failed to read jffi.toml. Are you in a project directory?")?;

    let config: Config = toml::from_str(&contents).context("Failed to parse jffi.toml")?;

    if config.schema_version != 1 {
        anyhow::bail!(
            "Unsupported jffi.toml schema_version {} (this JFFI release supports schema_version 1)",
            config.schema_version
        );
    }

    Ok(config)
}

pub fn save_config(config: &Config) -> Result<()> {
    let config_path = "jffi.toml";
    let contents = toml::to_string_pretty(config).context("Failed to serialize config")?;

    fs::write(config_path, contents).context("Failed to write jffi.toml")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"
schema_version = 1

[package]
name = "test-app"
version = "0.1.0"

[platforms]
enabled = []
"#;

    #[test]
    fn accepts_schema_version_one() {
        let config: Config = toml::from_str(MINIMAL).unwrap();
        assert_eq!(config.schema_version, 1);
        assert!(!config.platforms.web.cors);
    }

    #[test]
    fn rejects_unknown_configuration_fields() {
        let invalid = format!("{}\nunknown_setting = true\n", MINIMAL);
        assert!(toml::from_str::<Config>(&invalid).is_err());
    }

    #[test]
    fn accepts_apple_profile_mapping_for_app_extensions() {
        let input = format!(
            "{}\n[bundle]\nidentifier = \"com.example.app\"\n\
             [bundle.signing.profiles.release.apple]\n\
             provisioning_profiles = {{ \"com.example.app\" = \"Main Profile\", \
             \"com.example.app.ShareExtension\" = \"Share Profile\" }}\n",
            MINIMAL
        );
        let config: Config = toml::from_str(&input).unwrap();
        let signing_profiles = config.bundle.unwrap().signing.unwrap().profiles.unwrap();
        let profiles = &signing_profiles["release"]
            .apple
            .as_ref()
            .unwrap()
            .provisioning_profiles;
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles["com.example.app.ShareExtension"], "Share Profile");
    }

    #[test]
    fn documentation_toml_blocks_are_valid_toml() {
        for (name, document) in [
            ("configuration", include_str!("../../docs/configuration.md")),
            ("apple-signing", include_str!("../../docs/apple-signing.md")),
        ] {
            let mut blocks = 0;
            let mut in_toml = false;
            let mut snippet = String::new();
            for line in document.lines() {
                if !in_toml && line.trim_end() == "```toml" {
                    in_toml = true;
                    snippet.clear();
                } else if in_toml && line.trim_end() == "```" {
                    toml::from_str::<toml::Value>(&snippet).unwrap_or_else(|error| {
                        panic!("invalid TOML example in {name}: {error}\n{snippet}")
                    });
                    blocks += 1;
                    in_toml = false;
                } else if in_toml {
                    snippet.push_str(line);
                    snippet.push('\n');
                }
            }
            assert!(!in_toml, "{name} contains an unclosed TOML fence");
            assert!(blocks > 0, "{name} must contain a TOML example");
        }
    }
}
