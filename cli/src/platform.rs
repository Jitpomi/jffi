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
    pub const ALL: [Self; 6] = [
        Self::Ios,
        Self::Macos,
        Self::Android,
        Self::Windows,
        Self::Linux,
        Self::Web,
    ];

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
            Self::Linux => cfg!(target_os = "linux"),
            Self::Android | Self::Web => true,
        }
    }

    pub fn check_requirements(&self) -> Result<()> {
        crate::setup::setup_platform_for_build(self)
    }
}

/// A JFFI-owned directory shared by native build, run, and packaging.
pub(crate) fn windows_output_dir(platform: &str, release: bool) -> PathBuf {
    Path::new("platforms/windows/bin")
        .join(platform)
        .join(if release { "Release" } else { "Debug" })
        .join("jffi")
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
        if cfg!(target_os = "macos") && std::env::var_os("JFFI_ANDROID_STUDIO").is_none() {
            let status = Command::new("open")
                .args(["-a", "Android Studio"])
                .arg(&self.project_path)
                .status()?;
            if !status.success() {
                anyhow::bail!("Could not open Android Studio");
            }
        } else {
            let executable = std::env::var_os("JFFI_ANDROID_STUDIO")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    if cfg!(target_os = "windows") {
                        PathBuf::from(
                            std::env::var_os("ProgramFiles")
                                .unwrap_or_else(|| "C:\\Program Files".into()),
                        )
                        .join("Android/Android Studio/bin/studio64.exe")
                    } else {
                        PathBuf::from("android-studio")
                    }
                });
            Command::new(executable)
                .arg(&self.project_path)
                .spawn()
                .context(
                    "Could not launch Android Studio; set JFFI_ANDROID_STUDIO to its executable",
                )?;
        }
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
    pub fn get_available(requested: Option<&str>) -> Result<(String, String)> {
        let output = Command::new("xcrun")
            .args(["simctl", "list", "devices", "available", "--json"])
            .output()
            .context("Failed to list simulators")?;
        if !output.status.success() {
            anyhow::bail!(
                "simctl list devices failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        select_ios_simulator(&serde_json::from_slice(&output.stdout)?, requested)
    }

    pub fn boot(&self, uuid: &str) -> Result<()> {
        // Boot is harmless if already booted; bootstatus is the authoritative readiness check.
        let _ = Command::new("xcrun")
            .args(["simctl", "boot", uuid])
            .output();
        let status = Command::new("xcrun")
            .args(["simctl", "bootstatus", uuid, "-b"])
            .status()?;
        if !status.success() {
            anyhow::bail!("Simulator {} did not boot", uuid);
        }
        Ok(())
    }

    pub fn open_app(&self) -> Result<()> {
        let status = Command::new("open").args(["-a", "Simulator"]).status()?;
        if !status.success() {
            anyhow::bail!("Could not open Simulator");
        }
        Ok(())
    }

    pub fn install_app(&self, uuid: &str, app_path: &Path) -> Result<()> {
        let app_path_str = app_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid app path"))?;
        let status = Command::new("xcrun")
            .args(["simctl", "install", uuid, app_path_str])
            .status()
            .context("Failed to install app")?;

        if !status.success() {
            anyhow::bail!("Failed to install app in simulator");
        }
        Ok(())
    }

    pub fn launch_app(&self, uuid: &str, bundle_id: &str) -> Result<()> {
        let status = Command::new("xcrun")
            .args(["simctl", "launch", uuid, bundle_id])
            .status()
            .context("Failed to launch app")?;

        if !status.success() {
            anyhow::bail!("Failed to launch app in simulator");
        }
        Ok(())
    }
}

fn select_ios_simulator(
    value: &serde_json::Value,
    requested: Option<&str>,
) -> Result<(String, String)> {
    let devices = value
        .get("devices")
        .and_then(|v| v.as_object())
        .context("Invalid simctl devices JSON")?;
    let mut candidates = Vec::new();
    for (runtime, entries) in devices {
        if !runtime.starts_with("com.apple.CoreSimulator.SimRuntime.iOS-") {
            continue;
        }
        for device in entries.as_array().into_iter().flatten() {
            if device.get("isAvailable").and_then(|v| v.as_bool()) != Some(true) {
                continue;
            }
            let Some(id) = device.get("udid").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(name) = device.get("name").and_then(|v| v.as_str()) else {
                continue;
            };
            if requested.is_some_and(|requested| requested != id) {
                continue;
            }
            candidates.push((
                device.get("state").and_then(|v| v.as_str()) != Some("Booted"),
                name.to_string(),
                id.to_string(),
            ));
        }
    }
    candidates.sort();
    candidates.into_iter().next().map(|(_, name, id)| (name, id)).context("No available iOS simulator matches; install an iPhone/iPad runtime in Xcode or check --device-id")
}

#[cfg(test)]
mod simulator_tests {
    use super::*;
    #[test]
    fn selects_ipad_and_excludes_other_runtimes() {
        let devices = serde_json::json!({"devices": {
            "com.apple.CoreSimulator.SimRuntime.iOS-27-0": [
                {"name":"iPhone", "udid":"phone", "isAvailable":true, "state":"Shutdown"},
                {"name":"iPad", "udid":"pad", "isAvailable":true, "state":"Booted"}],
            "com.apple.CoreSimulator.SimRuntime.tvOS-27-0": [
                {"name":"TV", "udid":"tv", "isAvailable":true, "state":"Booted"}]
        }});
        assert_eq!(select_ios_simulator(&devices, None).unwrap().1, "pad");
        assert_eq!(
            select_ios_simulator(&devices, Some("phone")).unwrap().1,
            "phone"
        );
        assert!(select_ios_simulator(&devices, Some("tv")).is_err());
    }
}
