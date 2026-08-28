use crate::platform::Platform;
use anyhow::{Context, Result};
use colored::*;
use std::process::Command;

pub(crate) fn installation_allowed() -> bool {
    std::env::var("JFFI_INSTALL_MISSING").as_deref() == Ok("1")
}

fn managed_tool_reconciliation_allowed() -> bool {
    if installation_allowed() {
        return true;
    }

    std::env::var("JFFI_NO_SETUP").as_deref() != Ok("1")
        && !matches!(
            std::env::var("CARGO_NET_OFFLINE").as_deref(),
            Ok("1") | Ok("true")
        )
}

fn tool_succeeds(name: &str, args: &[&str]) -> bool {
    Command::new(name)
        .args(args)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn missing_linux_system_packages(
    has_cc: bool,
    has_pkg_config: bool,
    has_gtk4: bool,
    has_adwaita: bool,
    has_flatpak_builder: bool,
) -> Vec<&'static str> {
    let mut packages = Vec::new();
    if !has_cc {
        packages.push("build-essential");
    }
    if !has_pkg_config {
        packages.push("pkg-config");
    }
    if !has_gtk4 || !has_adwaita {
        packages.extend([
            "libgtk-4-dev",
            "libadwaita-1-dev",
            "python3-gi",
            "python3-gi-cairo",
            "gir1.2-gtk-4.0",
            "gir1.2-adw-1",
        ]);
    }
    if !has_flatpak_builder {
        packages.push("flatpak-builder");
    }
    packages
}

fn resolved_uniffi_version() -> Result<String> {
    if let Ok(lock) = std::fs::read_to_string("Cargo.lock") {
        let mut in_package = false;
        for line in lock.lines() {
            let line = line.trim();
            if line == "[[package]]" {
                in_package = false;
            } else if line == "name = \"uniffi\"" {
                in_package = true;
            } else if in_package && line.starts_with("version = ") {
                return Ok(line
                    .trim_start_matches("version = ")
                    .trim_matches('"')
                    .to_string());
            }
        }
    }
    let manifest = std::fs::read_to_string("core/Cargo.toml")
        .context("Could not read core/Cargo.toml to determine the UniFFI version")?;
    let manifest: toml::Value = toml::from_str(&manifest)?;
    let dependency = manifest
        .get("dependencies")
        .and_then(|value| value.get("uniffi"))
        .context("core/Cargo.toml does not declare the uniffi dependency")?;
    let requirement = dependency
        .as_str()
        .or_else(|| dependency.get("version").and_then(|value| value.as_str()))
        .context("The uniffi dependency has no version")?;
    Ok(requirement.trim_start_matches(['=', '^', '~']).to_string())
}

fn parse_uniffi_bindgen_version(output: &str) -> Option<&str> {
    let mut fields = output.split_whitespace();
    (fields.next()? == "uniffi-bindgen")
        .then(|| fields.next())
        .flatten()
}

fn installed_uniffi_bindgen_version() -> Option<String> {
    let output = Command::new("uniffi-bindgen")
        .arg("--version")
        .output()
        .ok()?;
    output.status.success().then_some(())?;
    parse_uniffi_bindgen_version(&String::from_utf8_lossy(&output.stdout)).map(str::to_string)
}

pub fn ensure_uniffi_bindgen() -> Result<()> {
    let required = resolved_uniffi_version()?;
    let installed = installed_uniffi_bindgen_version();
    if installed.as_deref() == Some(required.as_str()) {
        println!("  {} uniffi-bindgen {}", "✓".green(), required);
        return Ok(());
    }

    if !managed_tool_reconciliation_allowed() {
        let found = installed.as_deref().unwrap_or("not installed");
        anyhow::bail!(
            "uniffi-bindgen {} is required (found {}). Automatic setup is disabled; run `jffi setup --platform {}`",
            required,
            found,
            std::env::var("JFFI_SETUP_PLATFORM").unwrap_or_else(|_| "<platform>".to_string())
        );
    }

    match installed {
        Some(version) => println!(
            "  {} Reconciling uniffi-bindgen {} → {}...",
            "→".bright_blue(),
            version,
            required
        ),
        None => println!(
            "  {} Installing required uniffi-bindgen {}...",
            "→".bright_blue(),
            required
        ),
    }

    let exact = format!("={}", required);
    let status = Command::new("cargo")
        .args([
            "install",
            "uniffi",
            "--features",
            "cli",
            "--bin",
            "uniffi-bindgen",
            "--version",
            &exact,
            "--force",
            "--locked",
        ])
        .status()
        .context("Failed to install the matching uniffi-bindgen")?;
    if !status.success() {
        anyhow::bail!("Failed to install uniffi-bindgen {}", required);
    }

    let reconciled = installed_uniffi_bindgen_version();
    if reconciled.as_deref() != Some(required.as_str()) {
        anyhow::bail!(
            "Installed uniffi-bindgen did not resolve to {} (found {})",
            required,
            reconciled.as_deref().unwrap_or("not installed")
        );
    }
    println!("  {} uniffi-bindgen {} ready", "✓".green(), required);
    Ok(())
}

/// Ensure a CLI tool is installed, or try to install it.
pub fn ensure_tool(name: &str, install_cmd: &[&str]) -> Result<()> {
    print!("  {} Checking {}... ", "→".bright_blue(), name);

    let exists = Command::new(name)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if exists {
        println!("{}", "✓".green());
        return Ok(());
    }

    println!("{}", "not found".yellow());
    if !installation_allowed() {
        anyhow::bail!(
            "{} is required but not installed. Run `jffi setup --platform {}`",
            name,
            std::env::var("JFFI_SETUP_PLATFORM").unwrap_or_else(|_| "<platform>".to_string())
        );
    }
    println!("  {} Installing {}...", "→".bright_blue(), name);

    let status = Command::new(install_cmd[0])
        .args(&install_cmd[1..])
        .status()
        .context(format!("Failed to install {}", name))?;

    if !status.success() {
        anyhow::bail!("Failed to install {}. Please install it manually.", name);
    }

    println!("  {} {} installed successfully!", "✓".green(), name);
    Ok(())
}

/// Ensure cargo-ndk is available through its supported Cargo subcommand entrypoint.
/// Recent cargo-ndk releases intentionally reject direct `cargo-ndk --version`
/// invocation even though the tool is correctly installed.
fn ensure_cargo_ndk() -> Result<()> {
    print!("  {} Checking cargo ndk... ", "→".bright_blue());
    if tool_succeeds("cargo", &["ndk", "--version"]) {
        println!("{}", "✓".green());
        return Ok(());
    }

    println!("{}", "not found".yellow());
    if !installation_allowed() {
        anyhow::bail!(
            "cargo-ndk is required but not installed. Run `jffi setup --platform {}`",
            std::env::var("JFFI_SETUP_PLATFORM").unwrap_or_else(|_| "android".to_string())
        );
    }

    println!("  {} Installing cargo-ndk...", "→".bright_blue());
    let status = Command::new("cargo")
        .args(["install", "cargo-ndk"])
        .status()
        .context("Failed to install cargo-ndk")?;
    if !status.success() || !tool_succeeds("cargo", &["ndk", "--version"]) {
        anyhow::bail!("Failed to install cargo-ndk. Please install it manually.");
    }
    println!("  {} cargo-ndk installed successfully!", "✓".green());
    Ok(())
}

/// Ensure Rust targets are installed.
pub fn ensure_rust_targets(targets: &[&str]) -> Result<()> {
    let output = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .context("Failed to run rustup. Please install Rust via rustup.")?;

    let installed = String::from_utf8_lossy(&output.stdout);
    let mut missing = Vec::new();

    for target in targets {
        if !installed.lines().any(|l| l.trim() == *target) {
            missing.push(*target);
        }
    }

    if missing.is_empty() {
        return Ok(());
    }

    if !installation_allowed() {
        anyhow::bail!(
            "Missing Rust targets: {}. Run `jffi setup --platform {}`",
            missing.join(", "),
            std::env::var("JFFI_SETUP_PLATFORM").unwrap_or_else(|_| "<platform>".to_string())
        );
    }

    println!(
        "  {} Installing missing Rust targets: {}...",
        "→".bright_blue(),
        missing.join(", ")
    );

    let status = Command::new("rustup")
        .arg("target")
        .arg("add")
        .args(&missing)
        .status()
        .context("Failed to install Rust targets via rustup")?;

    if !status.success() {
        anyhow::bail!("Failed to install Rust targets: {}", missing.join(", "));
    }

    Ok(())
}

/// Ensure Python requirements are installed.
pub fn ensure_python_requirements(platform: &str) -> Result<()> {
    let requirements_path = std::path::Path::new("platforms")
        .join(platform)
        .join("requirements.txt");
    if !requirements_path.exists() {
        return Ok(());
    }

    if !installation_allowed() {
        let status = Command::new("python3")
            .args(["-c", "import gi"])
            .status()
            .context("Failed to check Python GTK bindings")?;
        if status.success() {
            return Ok(());
        }
        anyhow::bail!(
            "Python requirements for {} are missing. Run `jffi setup --platform {}`",
            platform,
            platform
        );
    }

    let status = Command::new("python3")
        .args(["-c", "import gi"])
        .status()
        .context("Failed to verify Python GTK bindings")?;
    if status.success() {
        return Ok(());
    }
    anyhow::bail!(
        "Python GTK bindings are still unavailable after platform package setup; install the dependencies listed in {} using your OS package manager",
        requirements_path.display()
    )
}

/// Setup all dependencies for a platform.
pub fn setup_platform(platform: &Platform) -> Result<()> {
    std::env::set_var("JFFI_SETUP_PLATFORM", platform.as_str());
    println!(
        "{}",
        format!("🔧 Checking environment for {}...", platform.as_str())
            .bright_cyan()
            .bold()
    );

    // Bindgen must exactly match the project's resolved UniFFI library.
    ensure_uniffi_bindgen()?;

    match platform {
        Platform::Ios => {
            if !tool_succeeds("xcodebuild", &["-version"]) {
                anyhow::bail!("Xcode (xcodebuild) is required for iOS builds. Please install it from the App Store.");
            }
            ensure_rust_targets(&[
                "aarch64-apple-ios",
                "x86_64-apple-ios",
                "aarch64-apple-ios-sim",
            ])?;
        }
        Platform::Macos => {
            if !tool_succeeds("xcodebuild", &["-version"]) {
                anyhow::bail!("Xcode (xcodebuild) is required for macOS builds. Please install it from the App Store.");
            }
            ensure_rust_targets(&["aarch64-apple-darwin", "x86_64-apple-darwin"])?;
        }
        Platform::Android => {
            ensure_cargo_ndk()?;
            let configured = crate::config::load_config().ok();
            let targets = android_rust_targets(
                configured
                    .as_ref()
                    .and_then(|value| value.platforms.android.abis.as_deref()),
            )?;
            ensure_rust_targets(&targets)?;
        }
        Platform::Linux => {
            // Helper to install system packages
            let install_system_deps = |packages: Vec<&str>| -> Result<()> {
                if !installation_allowed() {
                    anyhow::bail!(
                        "Missing system dependencies: {}. Run `jffi setup --platform linux`",
                        packages.join(", ")
                    );
                }
                if std::env::consts::OS == "linux"
                    && Command::new("apt-get").arg("--version").output().is_ok()
                {
                    let has_sudo = Command::new("sudo")
                        .arg("-n")
                        .arg("true")
                        .status()
                        .map(|s| s.success())
                        .unwrap_or(false);
                    let mut cmd = if has_sudo {
                        let mut c = Command::new("sudo");
                        c.arg("apt-get");
                        c
                    } else {
                        Command::new("apt-get")
                    };

                    println!(
                        "  {} Installing Linux system dependencies: {}...",
                        "→".bright_blue(),
                        packages.join(", ")
                    );
                    let status = cmd.arg("update").status()?;
                    if !status.success() {
                        anyhow::bail!("System package index update failed");
                    }

                    let mut cmd = if has_sudo {
                        let mut c = Command::new("sudo");
                        c.arg("apt-get");
                        c
                    } else {
                        Command::new("apt-get")
                    };
                    let status = cmd.args(["install", "-y"]).args(packages).status()?;
                    if !status.success() {
                        anyhow::bail!("System package installation failed");
                    }
                    Ok(())
                } else if std::env::consts::OS == "macos"
                    && Command::new("brew").arg("--version").output().is_ok()
                {
                    println!(
                        "  {} Installing macOS system dependencies for Linux support: {}...",
                        "→".bright_blue(),
                        packages.join(", ")
                    );

                    // Map linux package names to brew package names if needed
                    let brew_packages: Vec<&str> = packages
                        .iter()
                        .map(|&p| match p {
                            "libgtk-4-dev" => "gtk4",
                            "libadwaita-1-dev" => "libadwaita",
                            "python3-gi" => "pygobject3",
                            _ => p,
                        })
                        .collect();

                    let status = Command::new("brew")
                        .arg("install")
                        .args(brew_packages)
                        .status()?;

                    if !status.success() {
                        anyhow::bail!("Homebrew failed to install dependencies.");
                    }
                    Ok(())
                } else {
                    if std::env::consts::OS != "linux" && std::env::consts::OS != "macos" {
                        anyhow::bail!("Building for Linux requires a Linux or macOS host. {} is not supported.", std::env::consts::OS);
                    } else {
                        anyhow::bail!("Missing system dependencies: {}. Please install them manually using your package manager.", packages.join(", "));
                    }
                }
            };

            let packages = missing_linux_system_packages(
                tool_succeeds("cc", &["--version"]),
                tool_succeeds("pkg-config", &["--version"]),
                tool_succeeds("pkg-config", &["--exists", "gtk4"]),
                tool_succeeds("pkg-config", &["--exists", "libadwaita-1"]),
                tool_succeeds("flatpak-builder", &["--version"]),
            );
            if !packages.is_empty() {
                install_system_deps(packages)?;
            }

            // Install Python requirements
            ensure_python_requirements("linux")?;
        }
        Platform::Windows => {
            if std::env::consts::OS != "windows" {
                anyhow::bail!("Building for Windows requires a Windows host. Cross-compilation from {} is not yet fully supported.", std::env::consts::OS);
            }
            if !tool_succeeds("dotnet", &["--version"]) {
                anyhow::bail!(".NET SDK is required for Windows builds. Please install it from https://dotnet.microsoft.com/");
            }
            let configured = crate::config::load_config().ok();
            let targets = windows_rust_targets(
                configured
                    .as_ref()
                    .and_then(|value| value.bundle.as_ref())
                    .and_then(|bundle| bundle.windows.as_ref())
                    .map(|windows| windows.targets.as_slice()),
            )?;
            ensure_rust_targets(&targets)?;
            crate::commands::build::ensure_uniffi_bindgen_cs()?;
        }
        Platform::Web => {
            ensure_tool("wasm-pack", &["cargo", "install", "wasm-pack"])?;
            ensure_rust_targets(&["wasm32-unknown-unknown"])?;
            crate::commands::build::ensure_wasm_bindgen_cli()?;
        }
    }

    Ok(())
}

fn android_rust_targets(abis: Option<&[String]>) -> Result<Vec<&'static str>> {
    let supported = [
        ("arm64-v8a", "aarch64-linux-android"),
        ("armeabi-v7a", "armv7-linux-androideabi"),
        ("x86_64", "x86_64-linux-android"),
    ];

    let Some(abis) = abis else {
        return Ok(supported.iter().map(|(_, target)| *target).collect());
    };

    let unknown: Vec<&str> = abis
        .iter()
        .map(String::as_str)
        .filter(|abi| {
            !supported
                .iter()
                .any(|(supported_abi, _)| abi == supported_abi)
        })
        .collect();
    if !unknown.is_empty() {
        anyhow::bail!(
            "Unsupported Android ABIs in jffi.toml: {}",
            unknown.join(", ")
        );
    }

    let targets: Vec<&'static str> = supported
        .iter()
        .filter(|(abi, _)| abis.iter().any(|configured_abi| configured_abi == abi))
        .map(|(_, target)| *target)
        .collect();
    if targets.is_empty() {
        anyhow::bail!("platforms.android.abis must enable at least one supported ABI");
    }
    Ok(targets)
}

fn windows_rust_targets(targets: Option<&[String]>) -> Result<Vec<&'static str>> {
    let supported = ["x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"];
    let Some(targets) = targets else {
        return Ok(supported.to_vec());
    };

    let unknown: Vec<&str> = targets
        .iter()
        .map(String::as_str)
        .filter(|target| !supported.contains(target))
        .collect();
    if !unknown.is_empty() {
        anyhow::bail!(
            "Unsupported Windows targets in jffi.toml: {}",
            unknown.join(", ")
        );
    }
    if targets.is_empty() {
        anyhow::bail!("bundle.windows.targets must enable at least one supported target");
    }
    Ok(supported
        .iter()
        .filter(|target| targets.iter().any(|configured| configured == *target))
        .copied()
        .collect())
}

pub fn install_platform(platform: &Platform) -> Result<()> {
    std::env::set_var("JFFI_INSTALL_MISSING", "1");
    setup_platform(platform)
}

#[cfg(test)]
mod tests {
    use super::{
        android_rust_targets, missing_linux_system_packages, parse_uniffi_bindgen_version,
        windows_rust_targets,
    };

    #[test]
    fn parses_uniffi_bindgen_version() {
        assert_eq!(
            parse_uniffi_bindgen_version("uniffi-bindgen 0.31.1\n"),
            Some("0.31.1")
        );
    }

    #[test]
    fn rejects_unrelated_or_incomplete_version_output() {
        assert_eq!(parse_uniffi_bindgen_version("uniffi 0.31.1"), None);
        assert_eq!(parse_uniffi_bindgen_version("uniffi-bindgen"), None);
    }

    #[test]
    fn android_setup_honors_configured_abis() {
        let abis = vec!["arm64-v8a".to_string(), "armeabi-v7a".to_string()];
        assert_eq!(
            android_rust_targets(Some(&abis)).unwrap(),
            vec!["aarch64-linux-android", "armv7-linux-androideabi"]
        );
    }

    #[test]
    fn android_setup_rejects_unknown_abis() {
        let abis = vec!["mips".to_string()];
        assert!(android_rust_targets(Some(&abis)).is_err());
    }

    #[test]
    fn windows_setup_honors_bundle_targets() {
        let targets = vec!["x86_64-pc-windows-msvc".to_string()];
        assert_eq!(
            windows_rust_targets(Some(&targets)).unwrap(),
            vec!["x86_64-pc-windows-msvc"]
        );
    }

    #[test]
    fn windows_setup_rejects_unknown_targets() {
        let targets = vec!["i686-pc-windows-msvc".to_string()];
        assert!(windows_rust_targets(Some(&targets)).is_err());
    }

    #[test]
    fn linux_setup_includes_build_and_flatpak_tooling() {
        assert_eq!(
            missing_linux_system_packages(false, false, false, false, false),
            vec![
                "build-essential",
                "pkg-config",
                "libgtk-4-dev",
                "libadwaita-1-dev",
                "python3-gi",
                "python3-gi-cairo",
                "gir1.2-gtk-4.0",
                "gir1.2-adw-1",
                "flatpak-builder",
            ]
        );
        assert!(missing_linux_system_packages(true, true, true, true, true).is_empty());
    }
}
