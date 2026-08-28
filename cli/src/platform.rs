use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Supported platforms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Ios,
    Macos,
    Android,
    Windows,
    Linux,
    Web,
}

impl Platform {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "ios" => Some(Self::Ios),
            "macos" | "macos-arm64" | "macos-x64" => Some(Self::Macos),
            "android" => Some(Self::Android),
            "windows" => Some(Self::Windows),
            "linux" => Some(Self::Linux),
            "web" => Some(Self::Web),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ios => "ios",
            Self::Macos => "macos",
            Self::Android => "android",
            Self::Windows => "windows",
            Self::Linux => "linux",
            Self::Web => "web",
        }
    }

    pub fn is_host_supported(&self) -> bool {
        match self {
            Self::Ios | Self::Macos => cfg!(target_os = "macos"),
            Self::Windows => cfg!(target_os = "windows"),
            Self::Linux => cfg!(target_os = "linux") || cfg!(target_os = "macos"),
            Self::Android | Self::Web => true,
        }
    }

    pub fn check_requirements(&self) -> Result<()> {
        crate::setup::setup_platform_for_build(self)
    }
}

/// iOS/macOS Xcode project
pub struct XcodeProject {
    pub project_path: PathBuf,
    pub scheme: String,
}

impl XcodeProject {
    pub fn find(platform: Platform) -> Result<Self> {
        let platform_str = platform.as_str();
        let platform_dir = format!("platforms/{}", platform_str);

        // Find .xcodeproj
        let project_path = std::fs::read_dir(&platform_dir)
            .context(format!("Failed to read directory '{}'", platform_dir))?
            .filter_map(|e| e.ok())
            .find(|e| {
                e.path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s == "xcodeproj")
                    .unwrap_or(false)
            })
            .map(|e| e.path())
            .context(format!(
                "Could not find .xcodeproj file in '{}'",
                platform_dir
            ))?;

        let scheme = project_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(platform_str)
            .to_string();

        Ok(Self {
            project_path,
            scheme,
        })
    }

    pub fn build(&self, destination: &str, extra_args: &[&str]) -> Result<()> {
        use std::process::Stdio;

        let verbose = std::env::var("JFFI_VERBOSE").is_ok();

        let project_path_str = self
            .project_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid project path"))?;

        let mut clean_cmd = Command::new("xcodebuild");
        clean_cmd.args([
            "-project",
            project_path_str,
            "-scheme",
            &self.scheme,
            "clean",
        ]);
        if !verbose {
            clean_cmd.stdout(Stdio::null()).stderr(Stdio::null());
        }
        let _ = clean_cmd.status();

        let mut build_cmd = Command::new("xcodebuild");
        build_cmd.args(["-project", project_path_str, "-scheme", &self.scheme]);
        if !destination.is_empty() {
            build_cmd.args(["-destination", destination]);
        }
        build_cmd.arg("build");
        build_cmd.args(extra_args);
        if !verbose {
            build_cmd.stdout(Stdio::null()).stderr(Stdio::null());
        }

        let status = build_cmd
            .status()
            .context("Failed to build with xcodebuild")?;

        if !status.success() {
            anyhow::bail!("Build failed");
        }
        Ok(())
    }

    pub fn open(&self) -> Result<()> {
        Command::new("open")
            .arg(&self.project_path)
            .status()
            .context("Failed to open Xcode")?;
        Ok(())
    }
}

/// Android project
pub struct AndroidProject {
    pub project_path: PathBuf,
}

impl AndroidProject {
    pub fn find() -> Result<Self> {
        let path = PathBuf::from("platforms/android");
        if !path.exists() {
            anyhow::bail!("Android project not found at platforms/android");
        }
        Ok(Self { project_path: path })
    }

    pub fn open(&self) -> Result<()> {
        let path_str = self
            .project_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid project path"))?;
        Command::new("open")
            .args(["-a", "Android Studio", path_str])
            .status()
            .context("Failed to open Android Studio")?;
        Ok(())
    }

    pub fn find_tool(&self, tool_name: &str) -> Option<String> {
        if Command::new(tool_name).arg("--version").output().is_ok() {
            return Some(tool_name.to_string());
        }

        let home = std::env::var("HOME").ok()?;
        let tool_subdir = if tool_name == "emulator" {
            "emulator"
        } else {
            "platform-tools"
        };

        let mut possible_paths = vec![
            format!("{}/Library/Android/sdk/{}/{}", home, tool_subdir, tool_name),
            format!("{}/Android/Sdk/{}/{}", home, tool_subdir, tool_name),
            format!("{}/.android/sdk/{}/{}", home, tool_subdir, tool_name),
        ];

        if let Ok(sdk_root) = std::env::var("ANDROID_SDK_ROOT") {
            possible_paths.push(format!("{}/{}/{}", sdk_root, tool_subdir, tool_name));
        }
        if let Ok(sdk_home) = std::env::var("ANDROID_HOME") {
            possible_paths.push(format!("{}/{}/{}", sdk_home, tool_subdir, tool_name));
        }

        possible_paths.into_iter().find(|p| Path::new(p).exists())
    }
}

/// iOS Simulator utilities
pub struct IOSSimulator;

impl IOSSimulator {
    pub fn get_available() -> Result<(String, String)> {
        let output = Command::new("xcrun")
            .args(["simctl", "list", "devices", "available"])
            .output()
            .context("Failed to list simulators")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to get simulator list: {}", stderr);
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let mut booted_iphones: Vec<(String, String)> = Vec::new();
        let mut shutdown_iphones: Vec<(String, String)> = Vec::new();

        for line in output_str.lines() {
            if line.contains("iPhone") {
                let trimmed = line.trim();
                if let Some(first_paren) = trimmed.find(" (") {
                    let name = &trimmed[..first_paren];
                    let rest = &trimmed[first_paren + 2..];
                    if let Some(second_paren) = rest.find(')') {
                        let uuid = &rest[..second_paren];

                        if line.contains("(Booted)") {
                            booted_iphones.push((name.to_string(), uuid.to_string()));
                        } else if line.contains("(Shutdown)") {
                            shutdown_iphones.push((name.to_string(), uuid.to_string()));
                        }
                    }
                }
            }
        }

        // Prefer booted, then first available
        if let Some((name, uuid)) = booted_iphones.into_iter().next() {
            return Ok((name, uuid));
        }
        if let Some((name, uuid)) = shutdown_iphones.into_iter().next() {
            return Ok((name, uuid));
        }

        anyhow::bail!("No iPhone simulators found. Please install one in Xcode.")
    }

    pub fn boot(&self, uuid: &str) -> Result<()> {
        Command::new("xcrun")
            .args(["simctl", "boot", uuid])
            .output()
            .ok();
        Ok(())
    }

    pub fn open_app(&self) -> Result<()> {
        Command::new("open").args(["-a", "Simulator"]).status().ok();
        Ok(())
    }

    pub fn install_app(&self, app_path: &Path) -> Result<()> {
        let app_path_str = app_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid app path"))?;
        let status = Command::new("xcrun")
            .args(["simctl", "install", "booted", app_path_str])
            .status()
            .context("Failed to install app")?;

        if !status.success() {
            anyhow::bail!("Failed to install app in simulator");
        }
        Ok(())
    }

    pub fn launch_app(&self, bundle_id: &str) -> Result<()> {
        let status = Command::new("xcrun")
            .args(["simctl", "launch", "booted", bundle_id])
            .status()
            .context("Failed to launch app")?;

        if !status.success() {
            anyhow::bail!("Failed to launch app in simulator");
        }
        Ok(())
    }
}

/// Utility to find built iOS app in DerivedData
pub fn find_ios_app_bundle(project_name: &str) -> Result<PathBuf> {
    let home = std::env::var("HOME").context("Could not get HOME environment variable")?;
    let derived_data = format!("{}/Library/Developer/Xcode/DerivedData", home);

    std::fs::read_dir(&derived_data)
        .context("Could not read DerivedData directory")?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with(project_name))
        .find_map(|project_dir| {
            let app_path = project_dir
                .path()
                .join("Build/Products/Debug-iphonesimulator")
                .join(format!("{}.app", project_name));
            if app_path.exists() {
                Some(app_path)
            } else {
                None
            }
        })
        .context(format!("Could not find built .app bundle for {}. Try running 'jffi build --platform ios' first.", project_name))
}
