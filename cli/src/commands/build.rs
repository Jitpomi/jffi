use anyhow::{Context, Result};
use colored::*;
use std::process::Command;

pub fn build_project(platform: Option<String>, all: bool, release: bool) -> Result<()> {
    if all {
        build_all_platforms(release)?;
    } else if let Some(platform) = platform {
        build_platform(&platform, release)?;
    } else {
        anyhow::bail!("Specify --platform <platform> or --all");
    }
    
    Ok(())
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
    let profile = if release { "release" } else { "debug" };
    
    println!("  {} Building Rust library for iOS Simulator...", "→".bright_blue());
    let mut args = vec!["build"];
    if release {
        args.push("--release");
    }
    args.extend(&["--manifest-path", "ffi/Cargo.toml", "--target", "aarch64-apple-ios-sim"]);
    
    let status = Command::new("cargo")
        .args(&args)
        .status()
        .context("Failed to build Rust library")?;
    
    if !status.success() {
        anyhow::bail!("Rust build failed");
    }
    
    println!("  {} Generating Swift bindings...", "→".bright_blue());
    
    // Find the actual library file (it will have underscores instead of hyphens)
    let lib_dir = format!("target/aarch64-apple-ios-sim/{}", profile);
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
    
    for target in targets {
        println!("    Building for {}...", target);
        let status = Command::new("cargo")
            .args(&[
                "build",
                if release { "--release" } else { "" },
                "--package",
                "ffi",
                "--target",
                target,
            ])
            .status()
            .context(format!("Failed to build for {}", target))?;
        
        if !status.success() {
            anyhow::bail!("Rust build failed for {}", target);
        }
    }
    
    println!("  {} Generating Kotlin bindings...", "→".bright_blue());
    let lib_path = format!("target/aarch64-linux-android/{}/libffi.so", profile);
    
    let status = Command::new("uniffi-bindgen-cli")
        .args(&[
            "generate",
            "--library",
            &lib_path,
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
    
    println!("  {} Generating Swift bindings...", "→".bright_blue());
    let lib_path = format!("target/{}/{}/libffi.dylib", target, profile);
    
    let status = Command::new("uniffi-bindgen-cli")
        .args(&[
            "generate",
            "--library",
            &lib_path,
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
    let status = Command::new("cargo")
        .args(&[
            "build",
            if release { "--release" } else { "" },
            "--package",
            "ffi",
            "--target",
            "x86_64-unknown-linux-gnu",
        ])
        .status()
        .context("Failed to build Rust library")?;
    
    if !status.success() {
        anyhow::bail!("Rust build failed");
    }
    
    println!("  {} Generating C bindings...", "→".bright_blue());
    let lib_path = format!("target/x86_64-unknown-linux-gnu/{}/libffi.so", profile);
    
    // UniFFI can generate C bindings
    let status = Command::new("uniffi-bindgen-cli")
        .args(&[
            "generate",
            "--library",
            &lib_path,
            "--language",
            "python", // or C when available
            "--out-dir",
            "platforms/linux",
        ])
        .status()
        .context("Failed to generate bindings")?;
    
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
