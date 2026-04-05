use anyhow::{Context, Result};
use colored::*;
use std::process::Command;
use std::path::Path;

fn validate_project_structure() -> Result<()> {
    if !Path::new("jffi.toml").exists() {
        anyhow::bail!(
            "{}\n\n{}",
            "Error: Not in a JFFI project directory.".red().bold(),
            format!(
                "This command must be run from a project created with:\n  {} {}\n\nOr navigate to an existing JFFI project directory.",
                "jffi new".bright_cyan(),
                "<project-name>".bright_yellow()
            )
        );
    }
    
    if !Path::new("ffi").exists() {
        anyhow::bail!(
            "{}\n\n{}",
            "Error: Missing 'ffi' directory.".red().bold(),
            "Your project structure appears incomplete. Expected directories: core/, ffi/, platforms/"
        );
    }
    
    if !Path::new("core").exists() {
        anyhow::bail!(
            "{}\n\n{}",
            "Error: Missing 'core' directory.".red().bold(),
            "Your project structure appears incomplete. Expected directories: core/, ffi/, platforms/"
        );
    }
    
    Ok(())
}

pub fn build_project(platform: Option<String>, all: bool, release: bool, device: bool) -> Result<()> {
    validate_project_structure()?;
    
    if all {
        build_all_platforms(release)?;
    } else if let Some(platform) = platform {
        build_platform_with_options(&platform, release, device)?;
    } else {
        anyhow::bail!("Specify --platform <platform> or --all");
    }
    
    Ok(())
}

pub fn build_platform_with_options(platform: &str, release: bool, device: bool) -> Result<()> {
    validate_project_structure()?;
    
    println!("{}", format!("🔨 Building for {}...", platform).bright_green().bold());
    
    match platform {
        "ios" => {
            let target_type = if device { "device" } else { "simulator" };
            build_ios_target(release, target_type)
        },
        "android" => build_android(release),
        "macos" | "macos-arm64" => build_macos("aarch64", release),
        "macos-x64" => build_macos("x86_64", release),
        "windows" | "windows-x64" => build_windows("x86_64", release),
        "windows-x86" => build_windows("i686", release),
        "linux" => build_linux(release),
        "web" => build_web(release),
        _ => anyhow::bail!("Unknown platform: {}", platform),
    }
}

fn build_all_platforms(release: bool) -> Result<()> {
    println!("{}", "🔨 Building all platforms...".bright_green().bold());
    println!();
    
    // Read config to get enabled platforms
    let config = crate::platforms::config::load_config()?;
    
    for platform in &config.platforms.enabled {
        println!("{} Building {}...", "→".bright_blue(), platform.bright_cyan());
        if let Err(e) = build_platform(platform, release) {
            println!("{} Failed to build {}: {}", "✗".red(), platform, e);
        } else {
            println!("{} {} built successfully", "✓".green(), platform);
        }
        println!();
    }
    
    Ok(())
}

pub fn build_platform(platform: &str, release: bool) -> Result<()> {
    validate_project_structure()?;
    
    println!("{}", format!("🔨 Building for {}...", platform).bright_green().bold());
    
    match platform {
        "ios" => build_ios(release),
        "android" => build_android(release),
        "macos" | "macos-arm64" => build_macos("aarch64", release),
        "macos-x64" => build_macos("x86_64", release),
        "windows" | "windows-x64" => build_windows("x86_64", release),
        "windows-x86" => build_windows("i686", release),
        "linux" => build_linux(release),
        "web" => build_web(release),
        _ => anyhow::bail!("Unknown platform: {}", platform),
    }
}

fn build_ios(release: bool) -> Result<()> {
    build_ios_target(release, "simulator")
}

fn build_ios_target(release: bool, target_type: &str) -> Result<()> {
    let profile = if release { "release" } else { "debug" };
    
    // Choose target based on device vs simulator
    let (target, target_name) = match target_type {
        "device" => ("aarch64-apple-ios", "iOS Device"),
        "simulator" | _ => ("aarch64-apple-ios-sim", "iOS Simulator"),
    };
    
    println!("  {} Building Rust library for {}...", "→".bright_blue(), target_name);
    let mut args = vec!["build"];
    if release {
        args.push("--release");
    }
    args.extend(&["--manifest-path", "ffi/Cargo.toml", "--target", target]);
    
    let status = Command::new("cargo")
        .args(&args)
        .status()
        .context("Failed to build Rust library")?;
    
    if !status.success() {
        anyhow::bail!("Rust build failed");
    }
    
    println!("  {} Generating Swift bindings...", "→".bright_blue());
    
    // Find the actual library file (it will have underscores instead of hyphens)
    let lib_dir = format!("target/{}/{}", target, profile);
    let _lib_pattern = format!("{}/lib*ffi.dylib", lib_dir);
    
    let lib_path = std::fs::read_dir(&lib_dir)
        .context("Failed to read target directory")?
        .filter_map(|e| e.ok())
        .find(|e| {
            let name = e.file_name();
            let name_str = name.to_string_lossy();
            name_str.starts_with("lib") && name_str.ends_with("ffi.dylib")
        })
        .map(|e| e.path())
        .context("Could not find FFI library")?;
    
    let status = Command::new("uniffi-bindgen-cli")
        .args(&[
            "generate",
            "--library",
            lib_path.to_str().unwrap(),
            "--language",
            "swift",
            "--out-dir",
            "platforms/ios",
        ])
        .status()
        .context("Failed to generate Swift bindings")?;
    
    if !status.success() {
        anyhow::bail!("Binding generation failed");
    }
    
    println!("{}", "  ✅ iOS build complete".green());
    Ok(())
}

fn build_android(release: bool) -> Result<()> {
    let profile = if release { "release" } else { "debug" };
    
    println!("  {} Building Rust library for Android...", "→".bright_blue());
    
    // Build for multiple Android architectures
    let targets = vec![
        "aarch64-linux-android",
        "armv7-linux-androideabi",
        "x86_64-linux-android",
    ];
    
    // Ensure Android targets are installed
    ensure_android_targets(&targets)?;
    
    // Ensure cargo-ndk is installed for proper linking
    ensure_cargo_ndk()?;
    
    for target in targets {
        println!("    Building for {}...", target);
        
        let mut args = vec!["ndk", "build"];
        if release {
            args.push("--release");
        }
        args.extend(&["--manifest-path", "ffi/Cargo.toml", "--target", target]);
        
        let status = Command::new("cargo")
            .args(&args)
            .status()
            .context(format!("Failed to build for {}", target))?;
        
        if !status.success() {
            anyhow::bail!("Rust build failed for {}", target);
        }
    }
    
    println!("  {} Generating Kotlin bindings...", "→".bright_blue());
    
    // Find the actual library file (it will have underscores instead of hyphens)
    let lib_dir = format!("target/aarch64-linux-android/{}", profile);
    
    let lib_path = std::fs::read_dir(&lib_dir)
        .context("Failed to read target directory")?
        .filter_map(|e| e.ok())
        .find(|e| {
            let name = e.file_name();
            let name_str = name.to_string_lossy();
            name_str.starts_with("lib") && name_str.ends_with("ffi.so")
        })
        .map(|e| e.path())
        .context("Could not find FFI library")?;
    
    let status = Command::new("uniffi-bindgen-cli")
        .args(&[
            "generate",
            "--library",
            lib_path.to_str().unwrap(),
            "--language",
            "kotlin",
            "--out-dir",
            "platforms/android/app/src/main/java",
        ])
        .status()
        .context("Failed to generate Kotlin bindings")?;
    
    if !status.success() {
        anyhow::bail!("Binding generation failed");
    }
    
    println!("{}", "  ✅ Android build complete".green());
    Ok(())
}

fn build_macos(arch: &str, release: bool) -> Result<()> {
    let profile = if release { "release" } else { "debug" };
    let target = format!("{}-apple-darwin", arch);
    
    println!("  {} Building Rust library for macOS ({})...", "→".bright_blue(), arch);
    let mut args = vec!["build"];
    if release {
        args.push("--release");
    }
    args.extend(&["--manifest-path", "ffi/Cargo.toml", "--target", &target]);
    
    let status = Command::new("cargo")
        .args(&args)
        .status()
        .context("Failed to build Rust library")?;
    
    if !status.success() {
        anyhow::bail!("Rust build failed");
    }
    
    println!("  {} Generating Swift bindings...", "→".bright_blue());
    
    // Find the actual library file (it will have underscores instead of hyphens)
    let lib_dir = format!("target/{}/{}", target, profile);
    
    let lib_path = std::fs::read_dir(&lib_dir)
        .context("Failed to read target directory")?
        .filter_map(|e| e.ok())
        .find(|e| {
            let name = e.file_name();
            let name_str = name.to_string_lossy();
            name_str.starts_with("lib") && name_str.ends_with("ffi.dylib")
        })
        .map(|e| e.path())
        .context("Could not find FFI library")?;
    
    let status = Command::new("uniffi-bindgen-cli")
        .args(&[
            "generate",
            "--library",
            lib_path.to_str().unwrap(),
            "--language",
            "swift",
            "--out-dir",
            "platforms/macos",
        ])
        .status()
        .context("Failed to generate Swift bindings")?;
    
    if !status.success() {
        anyhow::bail!("Binding generation failed");
    }
    
    println!("{}", "  ✅ macOS build complete".green());
    Ok(())
}

fn build_windows(arch: &str, release: bool) -> Result<()> {
    let profile = if release { "release" } else { "debug" };
    let target = format!("{}-pc-windows-msvc", arch);
    
    println!("  {} Building Rust library for Windows ({})...", "→".bright_blue(), arch);
    let status = Command::new("cargo")
        .args(&[
            "build",
            if release { "--release" } else { "" },
            "--package",
            "ffi",
            "--target",
            &target,
        ])
        .status()
        .context("Failed to build Rust library")?;
    
    if !status.success() {
        anyhow::bail!("Rust build failed");
    }
    
    println!("  {} Generating C# bindings...", "→".bright_blue());
    let _lib_path = format!("target/{}/{}/ffi.dll", target, profile);
    
    // Note: UniFFI doesn't natively support C#, we'll need a custom generator
    println!("  {} C# bindings generation (coming soon)", "○".yellow());
    
    println!("{}", "  ✅ Windows build complete".green());
    Ok(())
}

fn build_linux(release: bool) -> Result<()> {
    let profile = if release { "release" } else { "debug" };
    
    println!("  {} Building Rust library for Linux...", "→".bright_blue());
    
    let mut args = vec!["build"];
    if release {
        args.push("--release");
    }
    args.extend(&["--manifest-path", "ffi/Cargo.toml"]);
    
    let status = Command::new("cargo")
        .args(&args)
        .status()
        .context("Failed to build Rust library")?;
    
    if !status.success() {
        anyhow::bail!("Rust build failed");
    }
    
    println!("  {} Generating Python bindings...", "→".bright_blue());
    
    // Check if uniffi-bindgen-cli is installed, install if not
    if Command::new("uniffi-bindgen-cli").arg("--version").output().is_err() {
        println!("  {} Installing uniffi-bindgen-cli...", "→".bright_blue());
        let install_status = Command::new("cargo")
            .args(&["install", "uniffi-bindgen-cli", "--version", "0.31.0"])
            .status()
            .context("Failed to install uniffi-bindgen-cli")?;
        
        if !install_status.success() {
            anyhow::bail!("Failed to install uniffi-bindgen-cli");
        }
    }
    
    // Find the library file
    let lib_dir = format!("target/{}", profile);
    let lib_path = std::fs::read_dir(&lib_dir)
        .context("Failed to read target directory")?
        .filter_map(|e| e.ok())
        .find(|e| {
            let name = e.file_name();
            let name_str = name.to_string_lossy();
            name_str.starts_with("lib") && name_str.ends_with("ffi.so")
        })
        .map(|e| e.path())
        .context("Could not find FFI library")?;
    
    let status = Command::new("uniffi-bindgen-cli")
        .args(&[
            "generate",
            "--library",
            lib_path.to_str().unwrap(),
            "--language",
            "python",
            "--out-dir",
            "platforms/linux",
        ])
        .status()
        .context("Failed to generate Python bindings")?;
    
    if !status.success() {
        anyhow::bail!("Binding generation failed");
    }
    
    println!("{}", "  ✅ Linux build complete".green());
    Ok(())
}

fn build_web(release: bool) -> Result<()> {
    println!("  {} Building Rust library for Web (WASM)...", "→".bright_blue());
    let status = Command::new("cargo")
        .args(&[
            "build",
            if release { "--release" } else { "" },
            "--package",
            "ffi",
            "--target",
            "wasm32-unknown-unknown",
        ])
        .status()
        .context("Failed to build Rust library")?;
    
    if !status.success() {
        anyhow::bail!("Rust build failed");
    }
    
    println!("  {} Generating JavaScript bindings with wasm-bindgen...", "→".bright_blue());
    // Note: This requires wasm-bindgen, not UniFFI
    println!("  {} WASM bindings (coming soon - needs wasm-bindgen integration)", "○".yellow());
    
    println!("{}", "  ✅ Web build complete".green());
    Ok(())
}

fn ensure_android_targets(targets: &[&str]) -> Result<()> {
    println!("  {} Checking Android targets...", "→".bright_blue());
    
    // Check which targets are installed
    let output = Command::new("rustup")
        .args(&["target", "list", "--installed"])
        .output()
        .context("Failed to check installed targets")?;
    
    let installed = String::from_utf8_lossy(&output.stdout);
    
    for target in targets {
        if !installed.contains(target) {
            println!("    Installing {}...", target.bright_yellow());
            let status = Command::new("rustup")
                .args(&["target", "add", target])
                .status()
                .context(format!("Failed to install target {}", target))?;
            
            if !status.success() {
                anyhow::bail!("Failed to install Android target: {}", target);
            }
        }
    }
    
    println!("  {} Android targets ready", "✓".green());
    Ok(())
}

fn ensure_cargo_ndk() -> Result<()> {
    println!("  {} Checking cargo-ndk...", "→".bright_blue());
    
    // Check if cargo-ndk is installed
    let check = Command::new("cargo")
        .args(&["ndk", "--version"])
        .output();
    
    if check.is_err() || !check.unwrap().status.success() {
        println!("    Installing cargo-ndk...");
        let status = Command::new("cargo")
            .args(&["install", "cargo-ndk"])
            .status()
            .context("Failed to install cargo-ndk")?;
        
        if !status.success() {
            anyhow::bail!("Failed to install cargo-ndk");
        }
        println!("  {} cargo-ndk installed", "✓".green());
    }
    
    Ok(())
}
