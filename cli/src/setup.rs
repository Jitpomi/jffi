use anyhow::{Context, Result};
use colored::*;
use std::process::Command;
use crate::platform::Platform;

/// Ensure a CLI tool is installed, or try to install it.
pub fn ensure_tool(name: &str, install_cmd: &[&str]) -> Result<()> {
    print!("  {} Checking {}... ", "→".bright_blue(), name);
    
    let exists = if name == "uniffi-bindgen" {
        // Special check for uniffi-bindgen as it might be a cargo subcommand
        Command::new("cargo").args(["uniffi-bindgen", "--version"]).output().is_ok() ||
        Command::new(name).arg("--version").output().is_ok()
    } else {
        Command::new(name).arg("--version").output().is_ok()
    };

    if exists {
        println!("{}", "✓".green());
        return Ok(());
    }

    println!("{}", "not found".yellow());
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

    println!("  {} Installing missing Rust targets: {}...", "→".bright_blue(), missing.join(", "));

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
    let requirements_path = std::path::Path::new("platforms").join(platform).join("requirements.txt");
    if !requirements_path.exists() {
        return Ok(());
    }

    println!("  {} Checking Python dependencies (requirements.txt)... ", "→".bright_blue());
    
    let req_str = requirements_path.to_str().unwrap();
    
    // Attempt 1: Standard user install
    let status = Command::new("pip3")
        .args(["install", "--user", "-r", req_str])
        .status()
        .context("Failed to run pip3")?;

    if status.success() {
        return Ok(());
    }

    // Attempt 2: Handle PEP 668 (externally-managed-environment)
    // This is common on Debian 12+, Ubuntu 23.04+, etc.
    println!("  {} Environment is managed by OS. Retrying with --break-system-packages...", "⚠".yellow());
    let status = Command::new("pip3")
        .args(["install", "--user", "--break-system-packages", "-r", req_str])
        .status()
        .context("Failed to run pip3 with --break-system-packages")?;

    if !status.success() {
        anyhow::bail!("Failed to install Python dependencies. Please run 'pip3 install -r {}' manually.", requirements_path.display());
    }

    Ok(())
}

/// Setup all dependencies for a platform.
pub fn setup_platform(platform: &Platform) -> Result<()> {
    println!("{}", format!("🔧 Checking environment for {}...", platform.as_str()).bright_cyan().bold());

    // 1. Ensure uniffi-bindgen (Global)
    ensure_tool("uniffi-bindgen", &["cargo", "install", "uniffi_bindgen", "--version", "0.31.1"])?;

    match platform {
        Platform::Ios => {
            if Command::new("xcodebuild").arg("-version").output().is_err() {
                anyhow::bail!("Xcode (xcodebuild) is required for iOS builds. Please install it from the App Store.");
            }
            ensure_rust_targets(&["aarch64-apple-ios", "x86_64-apple-ios", "aarch64-apple-ios-sim"])?;
        }
        Platform::Macos => {
            if Command::new("xcodebuild").arg("-version").output().is_err() {
                anyhow::bail!("Xcode (xcodebuild) is required for macOS builds. Please install it from the App Store.");
            }
            ensure_rust_targets(&["aarch64-apple-darwin", "x86_64-apple-darwin"])?;
        }
        Platform::Android => {
            ensure_tool("cargo-ndk", &["cargo", "install", "cargo-ndk"])?;
            ensure_rust_targets(&["aarch64-linux-android", "armv7-linux-androideabi", "x86_64-linux-android"])?;
        }
        Platform::Linux => {
            if Command::new("cc").arg("--version").output().is_err() {
                anyhow::bail!("C compiler (cc) is missing. Install build-essential or equivalent.");
            }
            
            // Helper to install system packages
            let install_system_deps = |packages: Vec<&str>| -> Result<()> {
                if std::env::consts::OS == "linux" && Command::new("apt-get").arg("--version").output().is_ok() {
                    let has_sudo = Command::new("sudo").arg("-n").arg("true").status().map(|s| s.success()).unwrap_or(false);
                    let mut cmd = if has_sudo {
                        let mut c = Command::new("sudo");
                        c.arg("apt-get");
                        c
                    } else {
                        Command::new("apt-get")
                    };
                    
                    println!("  {} Installing Linux system dependencies: {}...", "→".bright_blue(), packages.join(", "));
                    let _ = cmd.args(["install", "-y"]).args(packages).status()?;
                    Ok(())
                } else if std::env::consts::OS == "macos" && Command::new("brew").arg("--version").output().is_ok() {
                    println!("  {} Installing macOS system dependencies for Linux support: {}...", "→".bright_blue(), packages.join(", "));
                    
                    // Map linux package names to brew package names if needed
                    let brew_packages: Vec<&str> = packages.iter().map(|&p| match p {
                        "libgtk-4-dev" => "gtk4",
                        "libadwaita-1-dev" => "libadwaita",
                        "python3-gi" => "pygobject3",
                        _ => p,
                    }).collect();
                    
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

            if Command::new("pkg-config").arg("--version").output().is_err() {
                install_system_deps(vec!["pkg-config"])?;
            }
            
            // Check for GTK 4 and Libadwaita
            let has_gtk4 = Command::new("pkg-config").args(["--exists", "gtk4"]).status().map(|s| s.success()).unwrap_or(false);
            let has_adwaita = Command::new("pkg-config").args(["--exists", "libadwaita-1"]).status().map(|s| s.success()).unwrap_or(false);
            
            if !has_gtk4 || !has_adwaita {
                install_system_deps(vec![
                    "libgtk-4-dev", 
                    "libadwaita-1-dev", 
                    "python3-gi", 
                    "python3-gi-cairo", 
                    "gir1.2-gtk-4.0", 
                    "gir1.2-adw-1"
                ])?;
            }

            // Install Python requirements
            ensure_python_requirements("linux")?;
        }
        Platform::Windows => {
            if std::env::consts::OS != "windows" {
                anyhow::bail!("Building for Windows requires a Windows host. Cross-compilation from {} is not yet fully supported.", std::env::consts::OS);
            }
            if Command::new("dotnet").arg("--version").output().is_err() {
                anyhow::bail!(".NET SDK is required for Windows builds. Please install it from https://dotnet.microsoft.com/");
            }
            ensure_rust_targets(&["x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"])?;
        }
        Platform::Web => {
            ensure_tool("wasm-pack", &["cargo", "install", "wasm-pack"])?;
            ensure_rust_targets(&["wasm32-unknown-unknown"])?;
        }
    }

    Ok(())
}
