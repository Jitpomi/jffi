use crate::platform::Platform;
use anyhow::{Context, Result};
use colored::*;
use std::process::Command;

pub(crate) fn installation_allowed() -> bool {
    std::env::var("JFFI_INSTALL_MISSING").as_deref() == Ok("1")
}

fn tool_succeeds(name: &str, args: &[&str]) -> bool {
    Command::new(name)
        .args(args)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
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

pub fn ensure_uniffi_bindgen() -> Result<()> {
    let required = resolved_uniffi_version()?;
    let installed = Command::new("uniffi-bindgen").arg("--version").output();
    if installed.as_ref().is_ok_and(|output| {
        output.status.success() && String::from_utf8_lossy(&output.stdout).contains(&required)
    }) {
        println!("  {} uniffi-bindgen {}", "✓".green(), required);
        return Ok(());
    }
    if !installation_allowed() {
        anyhow::bail!(
            "uniffi-bindgen {} is required. Run `jffi setup --platform {}`",
            required,
            std::env::var("JFFI_SETUP_PLATFORM").unwrap_or_else(|_| "<platform>".to_string())
        );
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
            ensure_tool("cargo-ndk", &["cargo", "install", "cargo-ndk"])?;
            ensure_rust_targets(&[
                "aarch64-linux-android",
                "armv7-linux-androideabi",
                "x86_64-linux-android",
            ])?;
        }
        Platform::Linux => {
            if !tool_succeeds("cc", &["--version"]) {
                anyhow::bail!("C compiler (cc) is missing. Install build-essential or equivalent.");
            }

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

            if Command::new("pkg-config")
                .arg("--version")
                .output()
                .is_err()
            {
                install_system_deps(vec!["pkg-config"])?;
            }

            // Check for GTK 4 and Libadwaita
            let has_gtk4 = Command::new("pkg-config")
                .args(["--exists", "gtk4"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            let has_adwaita = Command::new("pkg-config")
                .args(["--exists", "libadwaita-1"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);

            if !has_gtk4 || !has_adwaita {
                install_system_deps(vec![
                    "libgtk-4-dev",
                    "libadwaita-1-dev",
                    "python3-gi",
                    "python3-gi-cairo",
                    "gir1.2-gtk-4.0",
                    "gir1.2-adw-1",
                ])?;
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
            ensure_rust_targets(&["x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"])?;
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

pub fn install_platform(platform: &Platform) -> Result<()> {
    std::env::set_var("JFFI_INSTALL_MISSING", "1");
    setup_platform(platform)
}
