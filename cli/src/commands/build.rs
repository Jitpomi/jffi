use anyhow::{Context, Result};
use colored::*;
use std::process::Command;
use std::path::Path;
use std::fs;
use crate::platform::Platform;

fn validate_project_structure() -> Result<()> {
    if !Path::new("jffi.toml").exists() {
        anyhow::bail!(
            "{}\n\nThis command must be run from a project created with:\n  {} {}\n\nOr navigate to an existing JFFI project directory.",
            "Error: Not in a JFFI project directory.".red().bold(),
            "jffi new".bright_cyan(),
            "<project-name>".bright_yellow()
        );
    }
    
    if !Path::new("core").exists() {
        anyhow::bail!(
            "{}\n\n{}",
            "Error: Missing 'core' directory.".red().bold(),
            "Your project structure appears incomplete. Expected directories: core/, platforms/"
        );
    }
    
    // Auto-fix private modules in core/src/lib.rs that UniFFI silently drops
    let _ = ensure_public_modules();
    
    Ok(())
}

fn ensure_public_modules() -> Result<()> {
    let lib_rs_path = Path::new("core/src/lib.rs");
    if !lib_rs_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(lib_rs_path)?;
    let mut modified = false;
    let mut new_lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        // UniFFI ignores exported types in private modules. 
        // Automatically convert `mod name;` to `pub mod name;` (excluding internal android/tests).
        if trimmed.starts_with("mod ") && trimmed.ends_with(';') 
            && !trimmed.contains("android") && !trimmed.contains("tests") 
        {
            let new_line = line.replacen("mod ", "pub mod ", 1);
            new_lines.push(new_line);
            modified = true;
            println!("  {} Auto-fixed private module: changed `{}` to `pub {}` in core/src/lib.rs", 
                "ℹ".bright_blue(), trimmed, trimmed);
        } else {
            new_lines.push(line.to_string());
        }
    }

    if modified {
        fs::write(lib_rs_path, new_lines.join("\n") + "\n")?;
    }

    Ok(())
}


pub fn build_project(platform: Option<String>, all: bool, release: bool, device: bool, deploy: bool) -> Result<()> {
    validate_project_structure()?;
    
    let config = crate::config::load_config()?;
    sync_configs_to_platforms(&config)?;
    
    // Generate icons for the building platforms if configured
    if all {
        for p in &config.platforms.enabled {
            let _ = crate::commands::icons::generate_icons(&config, p);
        }
    } else if let Some(ref p) = platform {
        let _ = crate::commands::icons::generate_icons(&config, p);
    }
    
    if all {
        build_all_platforms(release)?;
    } else if let Some(platform) = platform {
        build_platform_with_options(&platform, release, device, deploy)?;
    } else {
        anyhow::bail!("Specify --platform <platform> or --all");
    }
    
    Ok(())
}

pub fn build_platform_with_options(platform: &str, release: bool, device: bool, deploy: bool) -> Result<()> {
    validate_project_structure()?;

    // Check if requirements are met for this platform
    if let Some(p_enum) = Platform::from_str(platform) {
        p_enum.check_requirements()?;
    }

    println!("{}", format!("🔨 Building for {}...", platform).bright_green().bold());
    
    match platform {
        "ios" => {
            let target_type = if device { "device" } else { "simulator" };
            build_ios_target(release, target_type)
        },
        "android" => build_android(release),
        "macos" => {
            let arch = if cfg!(target_arch = "aarch64") {
                "aarch64"
            } else {
                "x86_64"
            };
            build_macos(arch, release)
        }
        "macos-arm64" => build_macos("aarch64", release),
        "macos-x64" => build_macos("x86_64", release),
        "windows" => {
            if deploy {
                build_windows_all_archs(release)
            } else {
                build_windows_host_arch(release)
            }
        },
        "linux" => build_linux(release),
        "web" => build_web(release),
        _ => anyhow::bail!("Unknown platform: {}", platform),
    }
}

fn build_all_platforms(release: bool) -> Result<()> {
    println!("{}", "🔨 Building all platforms...".bright_green().bold());
    println!();
    
    // Read config to get enabled platforms
    let config = crate::config::load_config()?;
    
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
    
    // Check if requirements are met for this platform
    if let Some(p_enum) = Platform::from_str(platform) {
        p_enum.check_requirements()?;
    }

    println!("{}", format!("🔨 Building for {}...", platform).bright_green().bold());
    
    match platform {
        "ios" => build_ios(release),
        "android" => build_android(release),
        "macos" => {
            let arch = if cfg!(target_arch = "aarch64") {
                "aarch64"
            } else {
                "x86_64"
            };
            build_macos(arch, release)
        }
        "macos-arm64" => build_macos("aarch64", release),
        "macos-x64" => build_macos("x86_64", release),
        "windows" => build_windows_host_arch(release),
        "linux" => build_linux(release),
        "web" => build_web(release),
        _ => anyhow::bail!("Unknown platform: {}", platform),
    }
}

fn build_ios(release: bool) -> Result<()> {
    build_ios_xcframework(release)?;
    Ok(())
}

fn build_ios_target(release: bool, _target_type: &str) -> Result<()> {
    // Note: XCFramework includes both simulator and device, so target_type is ignored
    build_ios_xcframework(release)
}

fn build_ios_xcframework(release: bool) -> Result<()> {
    use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
    
    let profile = if release { "release" } else { "debug" };
    let verbose = std::env::var("JFFI_VERBOSE").is_ok();
    
    // Always build for all architectures to ensure Xcode compatibility
    // (Xcode may build for device even when running on simulator)
    let targets = vec![
        ("aarch64-apple-ios-sim", "iOS Simulator (ARM64)"),
        ("x86_64-apple-ios", "iOS Simulator (x86_64)"),
        ("aarch64-apple-ios", "iOS Device"),
    ];
    
    
    if verbose {
        // Verbose mode: no progress bars, just plain output
        for (target, target_name) in &targets {
            println!("  {} Building Rust library for {}...", "→".bright_blue(), target_name);
            
            let mut args = vec!["build"];
            if release {
                args.push("--release");
            }
            args.extend(&["--manifest-path", "core/Cargo.toml", "--target", target]);
            
            let status = Command::new("cargo")
                .env("CARGO_TARGET_DIR", "target")
                .env("IPHONEOS_DEPLOYMENT_TARGET", "16.0")
                .args(&args)
                .status()
                .context(format!("Failed to build Rust library for {}", target))?;
            
            if !status.success() {
                anyhow::bail!("Rust build failed for {}", target);
            }
        }
    } else {
        // Clean mode: use progress bars
        let multi = MultiProgress::new();
        let spinner_style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
            .unwrap()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");
        
        for (target, target_name) in &targets {
            let pb = multi.add(ProgressBar::new_spinner());
            pb.set_style(spinner_style.clone());
            pb.set_prefix("  →");
            pb.set_message(format!("Building {}", target_name));
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            
            let mut args = vec!["build"];
            if release {
                args.push("--release");
            }
            args.extend(&["--manifest-path", "core/Cargo.toml", "--target", target]);
            
            let status = Command::new("cargo")
                .env("CARGO_TARGET_DIR", "target")
                .env("IPHONEOS_DEPLOYMENT_TARGET", "16.0")
                .args(&args)
                .status()
                .context(format!("Failed to build Rust library for {}", target))?;
            
            if !status.success() {
                pb.finish_with_message(format!("{} {}", "✗".red(), target_name));
                anyhow::bail!("Rust build failed for {}", target);
            }
            
            pb.finish_with_message(format!("{} {}", "✓".green(), target_name));
        }
    }
    
    println!("  {} Generating Swift bindings...", "→".bright_blue());
    
    // Use the first target's library for binding generation
    let lib_dir = format!("target/aarch64-apple-ios-sim/{}", profile);
    let lib_path = std::fs::read_dir(&lib_dir)
        .context("Failed to read target directory")?
        .filter_map(|e| e.ok())
        .find(|e| {
            let name = e.file_name();
            let name_str = name.to_string_lossy();
            name_str.starts_with("lib") && name_str.ends_with("core.a")
        })
        .map(|e| e.path())
        .context("Could not find FFI library")?;
    
    // Generate Swift bindings using uniffi-bindgen
    let status = Command::new("uniffi-bindgen")
        .args([
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
    
    // Post-process Swift bindings for Swift 6 concurrency compatibility
    // This is a workaround for UniFFI issue #2818 until official support is added
    patch_swift_concurrency("platforms/ios")?;
    
    println!("  {} Creating XCFramework...", "→".bright_blue());
    
    // Find library name from the built file
    let lib_name = lib_path.file_stem()
        .and_then(|s| s.to_str())
        .and_then(|s| s.strip_prefix("lib"))
        .context("Could not determine library name")?;
    
    let xcframework_path = format!("platforms/ios/{}.xcframework", lib_name);
    
    // Create universal simulator library (combine arm64 and x86_64 simulators)
    let sim_arm64_lib = format!("target/aarch64-apple-ios-sim/{}/lib{}.a", profile, lib_name);
    let sim_x86_lib = format!("target/x86_64-apple-ios/{}/lib{}.a", profile, lib_name);
    let sim_universal_lib = format!("target/ios-simulator-universal/lib{}.a", lib_name);
    
    // Ensure clean directory for lipo output (prevents "can't move temporary file" errors)
    let universal_dir = std::path::Path::new("target/ios-simulator-universal");
    
    // Force cleanup with retry - handles locked files from previous builds
    for attempt in 0..3 {
        if universal_dir.exists() {
            if let Err(e) = std::fs::remove_dir_all(universal_dir) {
                if attempt < 2 {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                } else {
                    eprintln!("  {} Warning: Could not clean ios-simulator-universal directory: {}", "⚠".yellow(), e);
                }
            }
        }
        break;
    }
    
    std::fs::create_dir_all(universal_dir)?;
    
    let lipo_status = Command::new("lipo")
        .args([
            "-create",
            &sim_arm64_lib,
            &sim_x86_lib,
            "-output",
            &sim_universal_lib,
        ])
        .status()
        .context("Failed to create universal simulator library")?;
    
    if !lipo_status.success() {
        anyhow::bail!("lipo failed to create universal simulator library");
    }
    
    let device_lib = format!("target/aarch64-apple-ios/{}/lib{}.a", profile, lib_name);
    
    // Check if XCFramework exists - if so, try to update in-place (preserves Xcode reference)
    if std::path::Path::new(&xcframework_path).exists() {
        // Find actual subdirectories in XCFramework (don't hardcode names)
        let mut sim_dir = None;
        let mut device_dir = None;
        
        if let Ok(entries) = std::fs::read_dir(&xcframework_path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    // Security: skip if directory name contains path traversal attempts
                    if dir_name.contains("..") || dir_name.contains('/') || dir_name.contains('\\') {
                        continue;
                    }
                    if dir_name.contains("simulator") {
                        sim_dir = Some(dir_name.to_string());
                    } else if dir_name.starts_with("ios-arm64") && !dir_name.contains("simulator") {
                        device_dir = Some(dir_name.to_string());
                    }
                }
            }
        }
        
        // If we found both directories, update in-place
        if let (Some(sim), Some(dev)) = (sim_dir, device_dir) {
            let sim_dest = format!("{}/{}/lib{}.a", xcframework_path, sim, lib_name);
            let device_dest = format!("{}/{}/lib{}.a", xcframework_path, dev, lib_name);
            
            match (
                std::fs::copy(&sim_universal_lib, &sim_dest),
                std::fs::copy(&device_lib, &device_dest)
            ) {
                (Ok(_), Ok(_)) => {
                    // Touch Info.plist to update modification time (helps Xcode detect changes)
                    let info_plist = format!("{}/Info.plist", xcframework_path);
                    if Command::new("touch").arg(&info_plist).status().is_err() {
                        // Fallback: update timestamp by reading and writing
                        if let Ok(content) = std::fs::read(&info_plist) {
                            let _ = std::fs::write(&info_plist, content);
                        }
                    }
                    
                    println!("{}", "  ✅ iOS XCFramework updated in-place".green());
                    return Ok(());
                }
                _ => {
                    // Copy failed, fall through to recreate
                    println!("  {} XCFramework update failed, recreating...", "⚠".yellow());
                    let _ = std::fs::remove_dir_all(&xcframework_path);
                }
            }
        } else {
            // Structure mismatch or incomplete, recreate
            println!("  {} XCFramework structure incomplete, recreating...", "⚠".yellow());
            let _ = std::fs::remove_dir_all(&xcframework_path);
        }
    }
    
    // XCFramework doesn't exist - create it fresh
    let xcframework_status = Command::new("xcodebuild")
        .args([
            "-create-xcframework",
            "-library", &sim_universal_lib,
            "-library", &device_lib,
            "-output", &xcframework_path,
        ])
        .status()
        .context("Failed to create XCFramework")?;
    
    if !xcframework_status.success() {
        anyhow::bail!("xcodebuild failed to create XCFramework");
    }
    
    println!("{}", "  ✅ iOS XCFramework created successfully".green());
    Ok(())
}

fn validate_android_manifest() -> Result<()> {
    // Check if AndroidManifest.xml has extractNativeLibs="true"
    // This is required for prebuilt AAR dependencies (JNA, ML Kit) that aren't 16 KB aligned
    let manifest_path = "platforms/android/app/src/main/AndroidManifest.xml";
    let manifest_content = std::fs::read_to_string(manifest_path)
        .context("Failed to read AndroidManifest.xml")?;
    
    if !manifest_content.contains("extractNativeLibs") {
        println!("  {} Warning: Add android:extractNativeLibs=\"true\" to <application> in AndroidManifest.xml", "⚠".bright_yellow());
        println!("     Required for prebuilt AAR dependencies (JNA, ML Kit) that aren't 16 KB aligned");
    }
    
    Ok(())
}

fn generate_android_cargo_config() -> Result<()> {
    // Generate .cargo/config.toml with 16 KB page alignment for Android 15+
    // This is required for modern Android devices to avoid runtime warnings and crashes
    // Must be at workspace root to apply to all dependencies (iroh, etc.)
    // See: https://developer.android.com/ndk/guides/16kb-page-size
    
    let cargo_dir = ".cargo";
    std::fs::create_dir_all(cargo_dir)
        .context("Failed to create .cargo directory")?;
    
    let config_content = r#"# Auto-generated by JFFI for Android 15+ compatibility
# Android 15+ requires native libraries to be 16 KB page-aligned
# See: https://developer.android.com/ndk/guides/16kb-page-size

[target.aarch64-linux-android]
rustflags = ["-C", "link-arg=-Wl,-z,max-page-size=16384"]

[target.armv7-linux-androideabi]
rustflags = ["-C", "link-arg=-Wl,-z,max-page-size=16384"]

[target.x86_64-linux-android]
rustflags = ["-C", "link-arg=-Wl,-z,max-page-size=16384"]

[target.i686-linux-android]
rustflags = ["-C", "link-arg=-Wl,-z,max-page-size=16384"]
"#;
    
    let config_path = format!("{}/config.toml", cargo_dir);
    std::fs::write(&config_path, config_content)
        .context("Failed to write .cargo/config.toml")?;
    
    Ok(())
}

fn check_ndk_context_needed() -> bool {
    // Check if any dependency uses ndk-context (e.g., hickory-resolver for DNS)
    let output = Command::new("cargo")
        .args(["tree", "-i", "ndk-context", "--target", "aarch64-linux-android", "--depth", "10"])
        .output();
    
    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // If ndk-context appears in the tree, we need to initialize it
        !stdout.contains("warning: nothing to print")
    } else {
        false
    }
}

fn build_android(release: bool) -> Result<()> {
    use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
    
    let verbose = std::env::var("JFFI_VERBOSE").is_ok();
    
    // Generate .cargo/config.toml with 16 KB page alignment for Android 15+
    generate_android_cargo_config()?;
    
    // Check if ndk-context initialization is needed and generate bridge BEFORE build
    let no_bridge = std::env::var("JFFI_NO_ANDROID_BRIDGE").is_ok();
    let needs_ndk_context = check_ndk_context_needed();
    
    if needs_ndk_context && no_bridge {
        println!("  {} Skipping ndk-context JNI bridge (--no-android-bridge)", "ℹ".bright_blue());
    } else if needs_ndk_context {
        println!("  {} Detected ndk-context dependency - generating JNI bridge...", "ℹ".bright_blue());
        generate_android_ndk_bridge()?;
        
        // Validate AndroidManifest.xml has extractNativeLibs for prebuilt AARs
        validate_android_manifest()?;
    }
    
    let config = crate::config::load_config()?;
    
    // Build for all Android architectures
    let mut architectures = vec![
        ("aarch64-linux-android", "arm64-v8a"),
        ("armv7-linux-androideabi", "armeabi-v7a"),
        ("x86_64-linux-android", "x86_64"),
    ];
    
    if let Some(abis) = &config.platforms.android.abis {
        architectures.retain(|(_, abi)| abis.contains(&abi.to_string()));
        if architectures.is_empty() {
            anyhow::bail!("No valid Android ABIs configured in jffi.toml. Enabled ABIs: {:?}", abis);
        }
    }
    
    let profile = if release { "release" } else { "debug" };
    
    if verbose {
        // Verbose mode: no progress bars, just plain output
        for (target, abi) in &architectures {
            println!("  {} Building Android {}...", "→".bright_blue(), abi);
            
            let mut args = vec!["ndk", "-t", target, "-o", "platforms/android/app/src/main/jniLibs", "build"];
            if release {
                args.push("--release");
            }
            args.extend(&["--manifest-path", "core/Cargo.toml"]);
            
            let ndk_platform = std::env::var("CARGO_NDK_PLATFORM")
                .unwrap_or_else(|_| config.platforms.android.min_sdk.to_string());
            
            let mut cmd = Command::new("cargo");
            cmd.env("CARGO_TARGET_DIR", "target")
                .env("CARGO_NDK_PLATFORM", ndk_platform)
                .args(&args);
            
            if let Some(rustflags) = &config.platforms.android.rustflags {
                let env_var_name = format!("CARGO_TARGET_{}_RUSTFLAGS", target.to_uppercase().replace("-", "_"));
                cmd.env(env_var_name, rustflags);
            }
            
            let status = cmd.status()
                .context(format!("Failed to build for {}", target))?;
            
            if !status.success() {
                anyhow::bail!("Rust build failed for {}", target);
            }
        }
    } else {
        // Clean mode: use progress bars
        let multi = MultiProgress::new();
        let spinner_style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
            .unwrap()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");
        
        for (target, abi) in &architectures {
            let pb = multi.add(ProgressBar::new_spinner());
            pb.set_style(spinner_style.clone());
            pb.set_prefix("  →");
            pb.set_message(format!("Building Android {}", abi));
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            
            let mut args = vec!["ndk", "-t", target, "-o", "platforms/android/app/src/main/jniLibs", "build"];
            if release {
                args.push("--release");
            }
            args.extend(&["--manifest-path", "core/Cargo.toml"]);
            
            let ndk_platform = std::env::var("CARGO_NDK_PLATFORM")
                .unwrap_or_else(|_| config.platforms.android.min_sdk.to_string());
            
            let mut cmd = Command::new("cargo");
            cmd.env("CARGO_TARGET_DIR", "target")
                .env("CARGO_NDK_PLATFORM", ndk_platform)
                .args(&args);
            
            if let Some(rustflags) = &config.platforms.android.rustflags {
                let env_var_name = format!("CARGO_TARGET_{}_RUSTFLAGS", target.to_uppercase().replace("-", "_"));
                cmd.env(env_var_name, rustflags);
            }
            
            let status = cmd.status()
                .context(format!("Failed to build for {}", target))?;
            
            if !status.success() {
                pb.finish_with_message(format!("{} Android {}", "✗".red(), abi));
                anyhow::bail!("Rust build failed for {}", target);
            }
            
            pb.finish_with_message(format!("{} Android {}", "✓".green(), abi));
        }
    }

    // Copy libc++_shared.so for each architecture if available in the NDK
    if let Some(ndk_dir) = find_ndk_dir() {
        for (target, abi) in &architectures {
            if let Some(libcxx_path) = find_libcxx_shared(&ndk_dir, target) {
                let dest_dir = format!("platforms/android/app/src/main/jniLibs/{}", abi);
                let dest_path = Path::new(&dest_dir).join("libc++_shared.so");
                if let Err(e) = std::fs::create_dir_all(&dest_dir) {
                    println!("  {} Warning: Failed to create jniLibs directory: {}", "⚠".yellow(), e);
                } else if let Err(e) = std::fs::copy(&libcxx_path, &dest_path) {
                    println!("  {} Warning: Failed to copy libc++_shared.so for {}: {}", "⚠".yellow(), abi, e);
                } else {
                    if verbose {
                        println!("  {} Copied libc++_shared.so to {}", "✓".green(), dest_path.display());
                    }
                }
            }
        }
    } else {
        println!("  {} Warning: Could not find Android NDK directory to bundle libc++_shared.so", "⚠".yellow());
    }
    
    // Find the library file for binding generation in the built target directories
    let mut lib_path = None;
    for (target, _) in &architectures {
        let lib_dir = format!("target/{}/{}", target, profile);
        if let Ok(entries) = std::fs::read_dir(&lib_dir) {
            if let Some(path) = entries
                .filter_map(|e| e.ok())
                .find(|e| {
                    let name = e.file_name();
                    let name_str = name.to_string_lossy();
                    name_str.starts_with("lib") && name_str.ends_with("core.so")
                })
                .map(|e| e.path())
            {
                lib_path = Some(path);
                break;
            }
        }
    }
    let lib_path = lib_path.context("Could not find FFI library in any target directory")?;
    
    // Generate Kotlin bindings using uniffi-bindgen
    let status = Command::new("uniffi-bindgen")
        .args([
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

fn generate_android_ndk_bridge() -> Result<()> {
    // Get package name from jffi.toml
    let config = crate::config::load_config()?;
    let package_name = &config.platforms.android.package;
    
    // Get project name for library name
    let cargo_toml = std::fs::read_to_string("core/Cargo.toml")
        .context("Failed to read core/Cargo.toml")?;
    let lib_name = cargo_toml
        .lines()
        .find(|line| line.starts_with("name"))
        .and_then(|line| line.split('"').nth(1))
        .context("Could not find project name in Cargo.toml")?
        .replace("-", "_");
    
    // 1. Generate Rust JNI bridge (core/src/android.rs)
    let jni_package = package_name.replace(".", "_");
    let android_rs = format!(r#"//! Android-specific JNI bridge for ndk-context initialization
//!
//! This module is auto-generated by JFFI to handle the UniFFI + JNA + ndk-context
//! incompatibility. UniFFI uses JNA which doesn't call JNI_OnLoad, so we need
//! a separate JNI function that Kotlin calls explicitly to initialize ndk-context.
//!
//! Background: Some Rust crates (like hickory-resolver used by iroh) need Android
//! context to access system DNS APIs. Since JNA doesn't provide JNI_OnLoad, we
//! expose this init function that Kotlin calls with the Application context.

use jni::JNIEnv;
use jni::objects::{{JClass, JObject}};
use std::ffi::c_void;

/// Initialize ndk-context with the Android application context.
/// 
/// This MUST be called from Kotlin before any Rust code that uses ndk-context
/// (e.g., DNS resolution via hickory-resolver).
///
/// # Safety
/// This function is called from JNI and must follow JNI safety rules.
/// The pointers passed to ndk-context must remain valid for the lifetime of the app.
#[no_mangle]
pub unsafe extern "C" fn Java_{jni_package}_JffiAndroidInit_initNdkContext(
    env: JNIEnv,
    _class: JClass,
    context: JObject,
) {{
    // Get the JavaVM pointer (required by ndk-context)
    let vm = match env.get_java_vm() {{
        Ok(vm) => vm,
        Err(e) => {{
            eprintln!("JFFI: Failed to get JavaVM: {{}}", e);
            return;
        }}
    }};
    
    // Create a global reference to the context so it persists
    let global_context = match env.new_global_ref(context) {{
        Ok(ctx) => ctx,
        Err(e) => {{
            eprintln!("JFFI: Failed to create global ref for Android context: {{}}", e);
            return;
        }}
    }};
    
    // Get raw pointers for ndk-context
    // ndk-context expects: (java_vm: *mut c_void, context_jobject: *mut c_void)
    let vm_ptr = vm.get_java_vm_pointer() as *mut c_void;
    let ctx_ptr = global_context.as_obj().as_raw() as *mut c_void;
    
    // Initialize ndk-context with raw pointers
    ndk_context::initialize_android_context(vm_ptr, ctx_ptr);
    
    // Keep the global reference alive (ndk-context stores the raw pointer)
    std::mem::forget(global_context);
    
    println!("JFFI: Android ndk-context initialized successfully");
}}
"#);
    
    std::fs::write("core/src/android.rs", android_rs)
        .context("Failed to write core/src/android.rs")?;
    
    // 2. Update core/src/lib.rs to include android module
    let lib_rs_path = "core/src/lib.rs";
    let mut lib_rs = std::fs::read_to_string(lib_rs_path)
        .context("Failed to read core/src/lib.rs")?;
    
    if !lib_rs.contains("mod android;") {
        // Add android module declaration after uniffi include
        if let Some(pos) = lib_rs.find("uniffi::include_scaffolding!") {
            let insert_pos = lib_rs[pos..].find('\n').map(|p| pos + p + 1).unwrap_or(lib_rs.len());
            lib_rs.insert_str(insert_pos, "\n#[cfg(target_os = \"android\")]\nmod android;\n");
            std::fs::write(lib_rs_path, lib_rs)
                .context("Failed to update core/src/lib.rs")?;
        }
    }
    
    // 3. Update core/Cargo.toml to add ndk-context and jni dependencies
    let cargo_toml_path = "core/Cargo.toml";
    let mut cargo_toml = std::fs::read_to_string(cargo_toml_path)
        .context("Failed to read core/Cargo.toml")?;
    
    if !cargo_toml.contains("ndk-context") {
        cargo_toml.push_str("\n# Android ndk-context support (auto-added by JFFI)\n");
        cargo_toml.push_str("[target.'cfg(target_os = \"android\")'.dependencies]\n");
        cargo_toml.push_str("ndk-context = \"0.1\"\n");
        cargo_toml.push_str("jni = \"0.21\"\n");
        std::fs::write(cargo_toml_path, cargo_toml)
            .context("Failed to update core/Cargo.toml")?;
    }
    
    // 4. Generate Kotlin JffiAndroidInit.kt
    let package_path = package_name.replace(".", "/");
    let kotlin_dir = format!("platforms/android/app/src/main/java/{}", package_path);
    std::fs::create_dir_all(&kotlin_dir)
        .context("Failed to create Kotlin package directory")?;
    
    let kotlin_init = format!(r#"package {package_name}

import android.content.Context

/**
 * JFFI Android initialization helper.
 * 
 * Auto-generated to handle ndk-context initialization for Rust dependencies
 * that need Android context (e.g., hickory-resolver for DNS).
 * 
 * This is necessary because UniFFI uses JNA which doesn't call JNI_OnLoad,
 * so we need explicit initialization before any Rust code runs.
 */
object JffiAndroidInit {{
    init {{
        System.loadLibrary("{lib_name}")
    }}

    /**
     * Initialize Android context for Rust code.
     * 
     * MUST be called before creating any Rust objects that use ndk-context.
     * Typically called in Application.onCreate() or Activity.onCreate().
     * 
     * @param context The Android application or activity context
     */
    @JvmStatic
    external fun initNdkContext(context: Context)
}}
"#);
    
    let kotlin_path = format!("{}/JffiAndroidInit.kt", kotlin_dir);
    std::fs::write(&kotlin_path, kotlin_init)
        .context("Failed to write JffiAndroidInit.kt")?;
    
    println!("  {} Generated JNI bridge and Kotlin helper", "✓".green());
    println!("  {} Add to MainActivity: JffiAndroidInit.initNdkContext(applicationContext)", "ℹ".bright_yellow());
    
    Ok(())
}

fn build_macos(_arch: &str, release: bool) -> Result<()> {
    build_macos_xcframework(release)
}

fn build_macos_xcframework(release: bool) -> Result<()> {
    use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
    
    let profile = if release { "release" } else { "debug" };
    let verbose = std::env::var("JFFI_VERBOSE").is_ok();
    let config = crate::config::load_config()?;
    
    // Get project name for library name from core/Cargo.toml
    let cargo_toml = std::fs::read_to_string("core/Cargo.toml")
        .context("Failed to read core/Cargo.toml")?;
    let lib_name = cargo_toml
        .lines()
        .find(|line| line.starts_with("name"))
        .and_then(|line| line.split('"').nth(1))
        .context("Could not find project name in Cargo.toml")?
        .replace("-", "_");
    
    // Always build for both architectures to ensure compatibility
    let targets = vec![
        ("aarch64-apple-darwin", "macOS Apple Silicon"),
        ("x86_64-apple-darwin", "macOS Intel"),
    ];
    
    
    if verbose {
        // Verbose mode: no progress bars, just plain output
        for (target, target_name) in &targets {
            println!("  {} Building Rust library for {}...", "→".bright_blue(), target_name);
            
            let mut args = vec!["build"];
            if release {
                args.push("--release");
            }
            args.extend(&["--manifest-path", "core/Cargo.toml", "--target", target]);
            
            let mut cmd = Command::new("cargo");
            cmd.env("CARGO_TARGET_DIR", "target")
                .env("MACOSX_DEPLOYMENT_TARGET", "13.0")
                .args(&args);
            
            let mut rustflags_val = config.platforms.macos.rustflags.clone().unwrap_or_default();
            if !rustflags_val.contains("-install_name") {
                if !rustflags_val.is_empty() {
                    rustflags_val.push(' ');
                }
                rustflags_val.push_str(&format!("-C link-arg=-Wl,-install_name,@rpath/lib{}.dylib", lib_name));
            }
            cmd.env("RUSTFLAGS", rustflags_val);
            
            let status = cmd.status()
                .context(format!("Failed to build Rust library for {}", target))?;
            
            if !status.success() {
                anyhow::bail!("Rust build failed for {}", target);
            }
        }
    } else {
        // Clean mode: use progress bars
        let multi = MultiProgress::new();
        let spinner_style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
            .unwrap()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");
        
        for (target, target_name) in &targets {
            let pb = multi.add(ProgressBar::new_spinner());
            pb.set_style(spinner_style.clone());
            pb.set_prefix("  →");
            pb.set_message(format!("Building {}", target_name));
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            
            let mut args = vec!["build"];
            if release {
                args.push("--release");
            }
            args.extend(&["--manifest-path", "core/Cargo.toml", "--target", target]);
            
            let mut cmd = Command::new("cargo");
            cmd.env("CARGO_TARGET_DIR", "target")
                .env("MACOSX_DEPLOYMENT_TARGET", "13.0")
                .args(&args);
            
            let mut rustflags_val = config.platforms.macos.rustflags.clone().unwrap_or_default();
            if !rustflags_val.contains("-install_name") {
                if !rustflags_val.is_empty() {
                    rustflags_val.push(' ');
                }
                rustflags_val.push_str(&format!("-C link-arg=-Wl,-install_name,@rpath/lib{}.dylib", lib_name));
            }
            cmd.env("RUSTFLAGS", rustflags_val);
            
            let status = cmd.status()
                .context(format!("Failed to build Rust library for {}", target))?;
            
            if !status.success() {
                pb.finish_with_message(format!("{} {}", "✗".red(), target_name));
                anyhow::bail!("Rust build failed for {}", target);
            }
            
            pb.finish_with_message(format!("{} {}", "✓".green(), target_name));
        }
    }
    
    println!("  {} Generating Swift bindings...", "→".bright_blue());
    
    // Use the first target's library for binding generation
    let lib_dir = format!("target/aarch64-apple-darwin/{}", profile);
    let lib_path = std::fs::read_dir(&lib_dir)
        .context("Failed to read target directory")?
        .filter_map(|e| e.ok())
        .find(|e| {
            let name = e.file_name();
            let name_str = name.to_string_lossy();
            name_str.starts_with("lib") && name_str.ends_with("core.dylib")
        })
        .map(|e| e.path())
        .context("Could not find FFI library")?;
    
    // Generate Swift bindings using uniffi-bindgen
    let status = Command::new("uniffi-bindgen")
        .args([
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
    
    // Post-process Swift bindings for Swift 6 concurrency compatibility
    // This is a workaround for UniFFI issue #2818 until official support is added
    patch_swift_concurrency("platforms/macos")?;
    
    println!("  {} Creating XCFramework...", "→".bright_blue());
    
    // Find library name from the built file
    let lib_name = lib_path.file_stem()
        .and_then(|s| s.to_str())
        .and_then(|s| s.strip_prefix("lib"))
        .context("Could not determine library name")?;
    
    let xcframework_path = format!("platforms/macos/{}.xcframework", lib_name);
    
    // Create universal library (combine arm64 and x86_64)
    // Use dylib for macOS because static libraries cannot be code signed when embedded
    let arm64_lib = format!("target/aarch64-apple-darwin/{}/lib{}.dylib", profile, lib_name);
    let x86_lib = format!("target/x86_64-apple-darwin/{}/lib{}.dylib", profile, lib_name);
    let universal_lib = format!("target/macos-universal/lib{}.dylib", lib_name);
    
    std::fs::create_dir_all("target/macos-universal")?;
    
    let lipo_status = Command::new("lipo")
        .args([
            "-create",
            &arm64_lib,
            &x86_lib,
            "-output",
            &universal_lib,
        ])
        .status()
        .context("Failed to create universal macOS library")?;
    
    if !lipo_status.success() {
        anyhow::bail!("lipo failed to create universal macOS library");
    }
    
    // Check if XCFramework exists - if so, try to update in-place (preserves Xcode reference)
    if std::path::Path::new(&xcframework_path).exists() {
        // Find actual subdirectory in XCFramework (don't hardcode name)
        let mut macos_dir = None;
        
        if let Ok(entries) = std::fs::read_dir(&xcframework_path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    // Security: skip if directory name contains path traversal attempts
                    if dir_name.contains("..") || dir_name.contains('/') || dir_name.contains('\\') {
                        continue;
                    }
                    if dir_name.starts_with("macos-") {
                        macos_dir = Some(dir_name.to_string());
                        break;
                    }
                }
            }
        }
        
        // If we found the directory, update in-place
        if let Some(dir) = macos_dir {
            let dest = format!("{}/{}/lib{}.dylib", xcframework_path, dir, lib_name);
            
            match std::fs::copy(&universal_lib, &dest) {
                Ok(_) => {
                    // Touch Info.plist to update modification time (helps Xcode detect changes)
                    let info_plist = format!("{}/Info.plist", xcframework_path);
                    if Command::new("touch").arg(&info_plist).status().is_err() {
                        // Fallback: update timestamp by reading and writing
                        if let Ok(content) = std::fs::read(&info_plist) {
                            let _ = std::fs::write(&info_plist, content);
                        }
                    }
                    
                    println!("{}", "  ✅ macOS XCFramework updated in-place".green());
                    return Ok(());
                }
                Err(_) => {
                    // Copy failed, fall through to recreate
                    println!("  {} XCFramework update failed, recreating...", "⚠".yellow());
                    let _ = std::fs::remove_dir_all(&xcframework_path);
                }
            }
        } else {
            // Structure mismatch, recreate
            println!("  {} XCFramework structure not found, recreating...", "⚠".yellow());
            let _ = std::fs::remove_dir_all(&xcframework_path);
        }
    }
    
    // XCFramework doesn't exist - create it fresh
    let xcframework_status = Command::new("xcodebuild")
        .args([
            "-create-xcframework",
            "-library", &universal_lib,
            "-output", &xcframework_path,
        ])
        .status()
        .context("Failed to create XCFramework")?;
    
    if !xcframework_status.success() {
        anyhow::bail!("xcodebuild failed to create XCFramework");
    }
    
    println!("{}", "  ✅ macOS XCFramework created successfully".green());
    Ok(())
}

fn build_windows_host_arch(release: bool) -> Result<()> {
    // Build only for the host architecture (faster for development)
    ensure_uniffi_bindgen_cs()?;
    
    // Detect host architecture
    let host_arch = if let Ok(arch) = std::env::var("PROCESSOR_ARCHITECTURE") {
        match arch.as_str() {
            "AMD64" => "x86_64",
            "x86" => "i686",
            "ARM64" => "aarch64",
            _ => "x86_64",
        }
    } else {
        "x86_64"
    };
    
    let host_platform = match host_arch {
        "x86_64" => "x64",
        "i686" => "x86",
        "aarch64" => "ARM64",
        _ => "x64",
    };
    
    println!("  {} Detected host architecture: {}", "→".bright_blue(), host_platform);
    
    let archs = [host_arch];
    let platforms = [host_platform];
    let profile = if release { "release" } else { "debug" };
    
    build_windows_archs(&archs, &platforms, release, profile)?;
    
    Ok(())
}

fn build_windows_all_archs(release: bool) -> Result<()> {
    // Build for all architectures (for deployment/distribution)
    ensure_uniffi_bindgen_cs()?;
    
    let archs = ["i686", "x86_64", "aarch64"];
    let platforms = ["x86", "x64", "ARM64"];
    let profile = if release { "release" } else { "debug" };
    
    build_windows_archs(&archs, &platforms, release, profile)?;
    
    Ok(())
}

fn check_windows_toolchain(arch: &str) -> Result<bool> {
    // Only aarch64 requires special toolchain detection.
    // x86_64 and i686 use MSVC which Rust's cc crate finds automatically
    // (via registry, environment variables, or VS installation paths).
    // cl.exe is typically NOT in global PATH but cargo build still works.
    match arch {
        "aarch64" => {
            // aarch64-pc-windows-msvc requires clang for C dependencies like ring
            let has_clang = Command::new("where")
                .arg("clang")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            Ok(has_clang)
        }
        "x86_64" | "i686" => {
            // MSVC is always assumed available for x64/x86 on Windows
            // Rust's cc crate handles MSVC discovery automatically
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn build_windows_archs(archs: &[&str], platforms: &[&str], release: bool, profile: &str) -> Result<()> {
    use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
    
    let verbose = std::env::var("JFFI_VERBOSE").is_ok();
    
    // Filter architectures by available toolchains
    let mut available_archs = Vec::new();
    let mut available_platforms = Vec::new();
    let mut skipped = Vec::new();
    
    for (i, arch) in archs.iter().enumerate() {
        match check_windows_toolchain(arch) {
            Ok(true) => {
                available_archs.push(*arch);
                available_platforms.push(platforms[i]);
            }
            Ok(false) => {
                let platform = platforms[i];
                let msg = match *arch {
                    "aarch64" => {
                        "Install LLVM/Clang: winget install LLVM.LLVM"
                    }
                    "x86_64" | "i686" => {
                        "Install Visual Studio Build Tools with C++ workload"
                    }
                    _ => "Install the required C compiler",
                };
                skipped.push(format!("{} ({})", platform, msg));
            }
            Err(_) => {
                skipped.push(format!("{} (toolchain check failed)", platforms[i]));
            }
        }
    }
    
    if !skipped.is_empty() {
        println!("  {} Skipping unsupported architectures:", "⚠".bright_yellow());
        for s in &skipped {
            println!("     • {}", s);
        }
    }
    
    if available_archs.is_empty() {
        anyhow::bail!("No Windows toolchains available. Install Visual Studio Build Tools (x64/x86) or LLVM/Clang (ARM64).");
    }
    
    // Step 1: Build Rust libraries for available architectures
    if verbose {
        // Verbose mode: no progress bars, just plain output
        for arch in available_archs.iter() {
            let target = format!("{}-pc-windows-msvc", arch);
            
            // Ensure the Rust target is installed
            let target_installed = Command::new("rustup")
                .args(["target", "list", "--installed"])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).contains(&target))
                .unwrap_or(false);
            if !target_installed {
                println!("  {} Installing Rust target {}...", "→".bright_blue(), target);
                let status = Command::new("rustup")
                    .args(["target", "add", &target])
                    .status()
                    .context("Failed to install Rust target via rustup")?;
                if !status.success() {
                    anyhow::bail!("rustup target add {} failed", target);
                }
            }

            println!("  {} Building Windows {}...", "→".bright_blue(), arch);
            
            let mut args = vec!["build"];
            if release {
                args.push("--release");
            }
            // Limit to 1 job on Windows to reduce memory pressure from large deps (iroh, etc.)
            args.push("--jobs");
            args.push("1");
            args.extend(&["--target", &target, "--manifest-path", "core/Cargo.toml"]);
            
            let status = Command::new("cargo")
                .env("CARGO_TARGET_DIR", "target")
                .args(&args)
                .status()
                .context("Failed to build Rust library")?;
            
            if !status.success() {
                anyhow::bail!("Rust build failed for {}", arch);
            }
        }
    } else {
        // Clean mode: use progress bars
        let multi = MultiProgress::new();
        let spinner_style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
            .unwrap()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");
        
        for arch in available_archs.iter() {
            let target = format!("{}-pc-windows-msvc", arch);
            
            // Ensure the Rust target is installed
            let target_installed = Command::new("rustup")
                .args(["target", "list", "--installed"])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).contains(&target))
                .unwrap_or(false);
            if !target_installed {
                println!("  {} Installing Rust target {}...", "→".bright_blue(), target);
                let status = Command::new("rustup")
                    .args(["target", "add", &target])
                    .status()
                    .context("Failed to install Rust target via rustup")?;
                if !status.success() {
                    anyhow::bail!("rustup target add {} failed", target);
                }
            }

            let pb = multi.add(ProgressBar::new_spinner());
            pb.set_style(spinner_style.clone());
            pb.set_prefix("  →");
            pb.set_message(format!("Building Windows {}", arch));
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            
            let mut args = vec!["build"];
            if release {
                args.push("--release");
            }
            // Limit to 1 job on Windows to reduce memory pressure from large deps (iroh, etc.)
            args.push("--jobs");
            args.push("1");
            args.extend(&["--target", &target, "--manifest-path", "core/Cargo.toml"]);
            
            let status = Command::new("cargo")
                .env("CARGO_TARGET_DIR", "target")
                .args(&args)
                .status()
                .context("Failed to build Rust library")?;
            
            if !status.success() {
                pb.finish_with_message(format!("{} Windows {}", "✗".red(), arch));
                anyhow::bail!("Rust build failed for {}", arch);
            }
            
            pb.finish_with_message(format!("{} Windows {}", "✓".green(), arch));
        }
    }
    
    // Step 2: Generate C# bindings once (using x64 DLL)
    let lib_name = std::env::current_dir()?
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("core")
        .replace("-", "_");
    let lib_path = format!("target/x86_64-pc-windows-msvc/{}/{}_core.dll", profile, lib_name);
    
    println!("  {} Generating C# bindings with uniffi-bindgen-cs...", "→".bright_blue());
    
    std::fs::create_dir_all("platforms/windows/Generated")?;
    
    let status = Command::new("uniffi-bindgen-cs")
        .arg("--library")
        .arg(&lib_path)
        .arg("--config")
        .arg("core/uniffi.toml")
        .arg("--out-dir")
        .arg("platforms/windows/Generated")
        .status()
        .context("Failed to run uniffi-bindgen-cs")?;
    if !status.success() {
        anyhow::bail!("uniffi-bindgen-cs failed to generate C# bindings");
    }
    
    // Auto-fix the uniffi-bindgen-cs v0.10.x interface prefix bug
    let cs_file_path = format!("platforms/windows/Generated/{}_core.cs", lib_name);
    if let Ok(content) = std::fs::read_to_string(&cs_file_path) {
        // uniffi-bindgen-cs v0.10.x generates `internal interface IFoo` for callback interfaces
        // but expects `Foo` everywhere else.
        let mut fixed_content = content.clone();
        
        let marker = "class UniffiCallbackInterface";
        let mut start_idx = 0;
        let mut patched = false;
        
        while let Some(idx) = fixed_content[start_idx..].find(marker) {
            let actual_idx = start_idx + idx;
            let name_start = actual_idx + marker.len();
            
            let mut name_end = name_start;
            for (i, c) in fixed_content[name_start..].char_indices() {
                if !c.is_alphanumeric() && c != '_' {
                    name_end = name_start + i;
                    break;
                }
            }
            
            let callback_name = fixed_content[name_start..name_end].to_string();
            if !callback_name.is_empty() {
                let bad_internal = format!("internal interface I{}", callback_name);
                let good_internal = format!("internal interface {}", callback_name);
                if fixed_content.contains(&bad_internal) {
                    fixed_content = fixed_content.replace(&bad_internal, &good_internal);
                    patched = true;
                }
                
                let bad_public = format!("public interface I{}", callback_name);
                let good_public = format!("public interface {}", callback_name);
                if fixed_content.contains(&bad_public) {
                    fixed_content = fixed_content.replace(&bad_public, &good_public);
                    patched = true;
                }
            }
            start_idx = name_end;
        }
        
        if patched {
            let _ = std::fs::write(&cs_file_path, fixed_content);
            println!("  {} Auto-fixed uniffi-bindgen-cs interface prefix bug", "ℹ".bright_blue());
        }
    }
    
    // Step 3: Copy DLL to Windows project root (BEFORE C# build - PostBuild target needs it)
    println!("  {} Copying DLL to project directory...", "→".bright_blue());
    let target = "x86_64-pc-windows-msvc"; // Default to x64
    let lib_path = format!("target/{}/{}/{}_core.dll", target, profile, lib_name);
    let dll_dest = format!("platforms/windows/{}_core.dll", lib_name);
    
    if std::path::Path::new(&lib_path).exists() {
        std::fs::copy(&lib_path, &dll_dest)
            .with_context(|| format!("Failed to copy DLL to {}", dll_dest))?;
        println!("  {} Copied DLL to platforms/windows/", "✓".green());
    } else {
        anyhow::bail!("Rust DLL not found at {}", lib_path);
    }
    
    // Step 4: Build C# project for specified platforms
    println!("  {} Building C# project with MSBuild...", "→".bright_blue());
    
    let csproj_file = std::fs::read_dir("platforms/windows")
        .context("Failed to read platforms/windows directory")?
        .filter_map(|e| e.ok())
        .find(|e| {
            let name = e.file_name();
            let name_str = name.to_string_lossy();
            name_str.ends_with(".csproj")
        })
        .map(|e| e.path())
        .context("Could not find .csproj file")?;
    
    let dotnet_cmd = find_dotnet();
    let msbuild_cmd = find_msbuild();
    let build_cmd: &str = if dotnet_cmd.is_some() { "dotnet" } else { "msbuild" };
    
    for platform in platforms.iter() {
        println!("  {} Building for {}...", "→".bright_blue(), platform);
        
        let mut build_args: Vec<String> = Vec::new();
        if build_cmd == "dotnet" {
            build_args.push("build".to_string());
            build_args.push(csproj_file.to_string_lossy().into_owned());
            build_args.push(format!("-p:Platform={}", platform));
            build_args.push("-p:PublishReadyToRun=false".to_string());
            build_args.push("-p:PublishTrimmed=false".to_string());
            if release {
                build_args.extend(["-c".to_string(), "Release".to_string()]);
            }
        } else {
            build_args.push(csproj_file.to_string_lossy().into_owned());
            build_args.push(format!("/p:Platform={}", platform));
            build_args.push("/p:PublishReadyToRun=false".to_string());
            build_args.push("/p:PublishTrimmed=false".to_string());
            if release {
                build_args.extend(["/p:Configuration=Release".to_string()]);
            }
        }
        
        let mut cmd = if build_cmd == "dotnet" {
            Command::new(dotnet_cmd.as_deref().unwrap_or("dotnet"))
        } else {
            Command::new(msbuild_cmd.as_deref().unwrap_or("msbuild"))
        };

        let status = cmd
            .args(&build_args)
            .status()
            .with_context(|| {
                let dotnet_hint = if dotnet_cmd.is_some() {
                    "dotnet was found but failed to run."
                } else {
                    "dotnet was not found (checked PATH and `C:\\Program Files\\dotnet\\dotnet.exe`). Install .NET 8 SDK from https://dotnet.microsoft.com/download"
                };
                let msbuild_hint = if msbuild_cmd.is_some() {
                    "msbuild was found but failed to run. Note: MSBuild requires the .NET SDK to resolve Microsoft.NET.Sdk-style projects."
                } else {
                    "msbuild was not found (checked PATH and Visual Studio via vswhere)."
                };
                format!(
                    "Failed to build Windows app. {dotnet_hint} {msbuild_hint} Install the .NET SDK (recommended) or Visual Studio Build Tools."
                )
            })?;
        
        if !status.success() {
            anyhow::bail!("C# build failed for platform {}", platform);
        }
    }
    
    println!("{}", "  ✅ Windows build complete".green());
    Ok(())
}

fn find_dotnet() -> Option<String> {
    // 1) PATH
    if Command::new("dotnet").arg("--version").output().is_ok() {
        return Some("dotnet".to_string());
    }
    // 2) Default install location
    let candidates = [
        r"C:\Program Files\dotnet\dotnet.exe",
        r"C:\Program Files (x86)\dotnet\dotnet.exe",
    ];
    for c in candidates {
        if std::path::Path::new(c).exists() {
            return Some(c.to_string());
        }
    }
    None
}

fn find_msbuild() -> Option<String> {
    // 1) PATH
    if Command::new("msbuild").arg("-version").output().is_ok() {
        return Some("msbuild".to_string());
    }

    // 2) Visual Studio discovery via vswhere (installed with VS / Build Tools)
    let vswhere = r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe";
    if !std::path::Path::new(vswhere).exists() {
        return None;
    }

    // Ask vswhere to locate MSBuild.exe. Example output:
    // C:\Program Files\Microsoft Visual Studio\2022\BuildTools\MSBuild\Current\Bin\MSBuild.exe
    let out = Command::new(vswhere)
        .args([
            "-latest",
            "-products",
            "*",
            "-requires",
            "Microsoft.Component.MSBuild",
            "-find",
            r"MSBuild\**\Bin\MSBuild.exe",
        ])
        .output()
        .ok()?;

    if !out.status.success() {
        return None;
    }

    let s = String::from_utf8_lossy(&out.stdout);
    let first = s.lines().find(|l| !l.trim().is_empty())?.trim().to_string();
    if std::path::Path::new(&first).exists() {
        Some(first)
    } else {
        None
    }
}

// Note: uniffi-bindgen is installed as a standalone cargo tool for binding generation.
// This avoids compiling uniffi_bindgen as a library dependency, which can OOM on small VMs.

fn ensure_uniffi_bindgen() -> Result<()> {
    println!("  {} Checking uniffi-bindgen...", "→".bright_blue());

    let check = Command::new("uniffi-bindgen")
        .arg("--version")
        .output();

    if check.is_err() || !check.unwrap().status.success() {
        println!("    Installing uniffi-bindgen...");
        let status = Command::new("cargo")
            .args(["install", "uniffi", "--features", "cli", "--bin", "uniffi-bindgen", "--version", "0.31.1"])
            .status()
            .context("Failed to install uniffi-bindgen")?;

        if !status.success() {
            anyhow::bail!("Failed to install uniffi-bindgen");
        }
        println!("    {} uniffi-bindgen installed successfully", "✓".green());
    } else {
        println!("    {} uniffi-bindgen is already installed", "✓".green());
    }

    Ok(())
}

fn ensure_uniffi_bindgen_cs() -> Result<()> {
    println!("  {} Checking uniffi-bindgen-cs...", "→".bright_blue());
    
    let check = Command::new("uniffi-bindgen-cs")
        .arg("--version")
        .output();
    
    if check.is_err() || !check.unwrap().status.success() {
        // Use git CLI for fetching (more reliable than libgit2 on Windows).
        // See Cargo docs: https://doc.rust-lang.org/cargo/reference/config.html#netgit-fetch-with-cli
        let repo = std::env::var("JFFI_UNIFFI_BINDGEN_CS_GIT").unwrap_or_else(|_| {
            // The previously-used microsoft/uniffi-bindgen-cs URL is no longer available.
            // NordSecurity maintains the widely-used fork.
            "https://github.com/NordSecurity/uniffi-bindgen-cs".to_string()
        });

        // NOTE: uniffi-bindgen-cs v0.10 targets UniFFI 0.29; JFFI uses UniFFI 0.31.
        // The generated C# bindings work in practice for our use case.
        // If you encounter compatibility issues with a specific UniFFI feature,
        // override with JFFI_UNIFFI_BINDGEN_CS_GIT / JFFI_UNIFFI_BINDGEN_CS_TAG env vars.
        
        // Prefer an explicit tag for repeatable installs; users can override to a fork/branch
        // that matches their UniFFI version.
        let tag = std::env::var("JFFI_UNIFFI_BINDGEN_CS_TAG").unwrap_or_else(|_| "v0.10.0+v0.29.4".to_string());
        let branch = std::env::var("JFFI_UNIFFI_BINDGEN_CS_BRANCH").ok();

        println!("    Installing uniffi-bindgen-cs ({} @ {})...", repo, branch.as_deref().unwrap_or(&tag));
        println!("    {} If you use UniFFI v0.31+, you may need a newer uniffi-bindgen-cs fork; set JFFI_UNIFFI_BINDGEN_CS_GIT/JFFI_UNIFFI_BINDGEN_CS_TAG.", "⚠".yellow());

        let mut cmd = Command::new("cargo");
        cmd.env("CARGO_NET_GIT_FETCH_WITH_CLI", "true")
            .args(["install", "uniffi-bindgen-cs", "--git", &repo]);
        if let Some(branch) = branch.as_deref() {
            cmd.args(["--branch", branch]);
        } else {
            cmd.args(["--tag", &tag]);
        }

        let status = cmd
            .status()
            .context("Failed to install uniffi-bindgen-cs")?;
        
        if !status.success() {
            anyhow::bail!("Failed to install uniffi-bindgen-cs");
        }
        println!("    {} uniffi-bindgen-cs installed successfully", "✓".green());
    } else {
        println!("    {} uniffi-bindgen-cs is already installed", "✓".green());
    }
    
    Ok(())
}

fn build_linux(release: bool) -> Result<()> {
    println!("{} Building Linux project...", "→".bright_blue());

    let profile = if release { "release" } else { "debug" };

    // Check for Python dependencies
    println!("  {} Checking dependencies...", "→".bright_blue());
    let python_check = Command::new("python3")
        .args(["-c", "import gi; gi.require_version('Gtk', '4.0')"])
        .output();

    if python_check.is_err() || !python_check.unwrap().status.success() {
        println!("{}", "  ⚠️  Python GTK4 dependencies not found.".yellow());
        println!("  Run setup.sh to install: sudo ./platforms/linux/setup.sh");
    }

    // Build Rust library for Linux
    let verbose = std::env::var("JFFI_VERBOSE").is_ok();
    
    let mut args = vec!["build"];
    if release {
        args.push("--release");
    }
    args.extend(&["--manifest-path", "core/Cargo.toml"]);

    if verbose {
        // Verbose mode: no progress bar, just plain output
        println!("  {} Building Rust library...", "→".bright_blue());
        
        let status = Command::new("cargo")
            .args(&args)
            .status()
            .context("Failed to build Rust library")?;

        if !status.success() {
            anyhow::bail!("Rust build failed");
        }
    } else {
        // Clean mode: use progress bar
        use indicatif::{ProgressBar, ProgressStyle};
        let pb = ProgressBar::new_spinner();
        let spinner_style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
            .unwrap()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");
        pb.set_style(spinner_style);
        pb.set_prefix("  →");
        pb.set_message("Building Rust library");
        pb.enable_steady_tick(std::time::Duration::from_millis(100));

        let status = Command::new("cargo")
            .args(&args)
            .status()
            .context("Failed to build Rust library")?;

        if !status.success() {
            pb.finish_with_message(format!("{} Build failed", "✗".red()));
            anyhow::bail!("Rust build failed");
        }

        pb.finish_with_message(format!("{} Rust library", "✓".green()));
    }

    println!("  {} Generating Python bindings...", "→".bright_blue());

    // Find the library file
    let lib_dir = format!("target/{}", profile);
    let lib_path = std::fs::read_dir(&lib_dir)
        .context("Failed to read target directory")?
        .filter_map(|e| e.ok())
        .find(|e| {
            let name = e.file_name();
            let name_str = name.to_string_lossy();
            name_str.starts_with("lib") && name_str.ends_with("core.so")
        })
        .map(|e| e.path())
        .context("Could not find FFI library")?;

    ensure_uniffi_bindgen()?;

    let status = Command::new("uniffi-bindgen")
        .args([
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
    use indicatif::{ProgressBar, ProgressStyle};
    
    let verbose = std::env::var("JFFI_VERBOSE").is_ok();
    
    // Ensure wasm32-unknown-unknown target is installed
    ensure_wasm_target()?;
    
    let profile = if release { "release" } else { "debug" };
    
    // Build the ffi-web crate for wasm32
    let mut args = vec!["build", "--target", "wasm32-unknown-unknown"];
    if release {
        args.push("--release");
    }
    args.extend(&["--manifest-path", "ffi-web/Cargo.toml"]);
    
    if verbose {
        // Verbose mode: no progress bar, just plain output
        println!("  {} Building WASM...", "→".bright_blue());
        
        let mut cmd = Command::new("cargo");
        
        // Auto-fix for macOS Apple Clang lacking WASM support
        if std::env::consts::OS == "macos" && std::env::var("CC_wasm32_unknown_unknown").is_err() {
            if let Ok(output) = Command::new("brew").args(["--prefix", "llvm"]).output() {
                if output.status.success() {
                    let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    let clang = format!("{}/bin/clang", prefix);
                    let ar = format!("{}/bin/llvm-ar", prefix);
                    if std::path::Path::new(&clang).exists() {
                        cmd.env("CC_wasm32_unknown_unknown", clang);
                        cmd.env("AR_wasm32_unknown_unknown", ar);
                    }
                }
            }
        }
        
        let status = cmd
            .args(&args)
            .status()
            .context("Failed to build Rust library for WASM")?;
        
        if !status.success() {
            anyhow::bail!("Rust WASM build failed");
        }
    } else {
        // Clean mode: use progress bar
        let pb = ProgressBar::new_spinner();
        let spinner_style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
            .unwrap()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");
        pb.set_style(spinner_style);
        pb.set_prefix("  →");
        pb.set_message("Building WASM");
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        
        let mut cmd = Command::new("cargo");
        
        // Auto-fix for macOS Apple Clang lacking WASM support
        if std::env::consts::OS == "macos" && std::env::var("CC_wasm32_unknown_unknown").is_err() {
            if let Ok(output) = Command::new("brew").args(["--prefix", "llvm"]).output() {
                if output.status.success() {
                    let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    let clang = format!("{}/bin/clang", prefix);
                    let ar = format!("{}/bin/llvm-ar", prefix);
                    if std::path::Path::new(&clang).exists() {
                        cmd.env("CC_wasm32_unknown_unknown", clang);
                        cmd.env("AR_wasm32_unknown_unknown", ar);
                    }
                }
            }
        }
        
        let status = cmd
            .args(&args)
            .status()
            .context("Failed to build Rust library for WASM")?;
        
        if !status.success() {
            pb.finish_with_message(format!("{} WASM", "✗".red()));
            anyhow::bail!("Rust WASM build failed");
        }
        
        pb.finish_with_message(format!("{} WASM", "✓".green()));
    }
    
    println!("  {} Generating JavaScript bindings with wasm-bindgen...", "→".bright_blue());
    
    // Find the .wasm file - need to get the actual project name
    let wasm_dir = format!("target/wasm32-unknown-unknown/{}", profile);
    let wasm_file = std::fs::read_dir(&wasm_dir)
        .context("Failed to read wasm target directory")?
        .filter_map(|e| e.ok())
        .find(|e| {
            let name = e.file_name();
            let name_str = name.to_string_lossy();
            name_str.ends_with("_ffi_web.wasm")
        })
        .map(|e| e.path())
        .context("Could not find WASM file")?;
    
    // Derive the out-name from the .wasm filename to match wasm-pack conventions
    // e.g. "bena_ffi_web.wasm" -> "bena_ffi_web"
    let out_name = wasm_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("wasm")
        .to_string();
    
    // Ensure wasm-bindgen-cli is installed with exact version matching the resolved library
    ensure_wasm_bindgen_cli()?;
    
    // Run wasm-bindgen to generate JS bindings
    let status = Command::new("wasm-bindgen")
        .arg(wasm_file.to_str().unwrap())
        .arg("--out-dir")
        .arg("platforms/web/pkg")
        .arg("--target")
        .arg("web")
        .arg("--out-name")
        .arg(&out_name)
        .status()
        .context("Failed to run wasm-bindgen")?;
    
    if !status.success() {
        anyhow::bail!("wasm-bindgen failed");
    }
    
    println!("{}", "  ✅ Web build complete".green());
    Ok(())
}

fn ensure_wasm_target() -> Result<()> {
    println!("  {} Checking wasm32-unknown-unknown target...", "→".bright_blue());
    
    let output = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .context("Failed to check installed targets")?;
    
    let installed = String::from_utf8_lossy(&output.stdout);
    
    if !installed.contains("wasm32-unknown-unknown") {
        println!("    Installing wasm32-unknown-unknown...");
        let status = Command::new("rustup")
            .args(["target", "add", "wasm32-unknown-unknown"])
            .status()
            .context("Failed to install wasm32-unknown-unknown target")?;
        
        if !status.success() {
            anyhow::bail!("Failed to install wasm32-unknown-unknown target");
        }
    }
    
    Ok(())
}

fn ensure_wasm_bindgen_cli() -> Result<()> {
    println!("  {} Checking wasm-bindgen-cli...", "→".bright_blue());
    
    // Read exact resolved wasm-bindgen version from Cargo.lock (generated by cargo build)
    let lock_content = std::fs::read_to_string("Cargo.lock")
        .context("Failed to read Cargo.lock. Run cargo build first.")?;
    
    let mut in_wasm_bindgen = false;
    let mut required_version = None;
    
    for line in lock_content.lines() {
        let trimmed = line.trim();
        if trimmed == "[[package]]" {
            in_wasm_bindgen = false;
        } else if trimmed == r#"name = "wasm-bindgen""# {
            in_wasm_bindgen = true;
        } else if in_wasm_bindgen && trimmed.starts_with(r#"version = ""#) {
            required_version = trimmed.split('"').nth(1).map(|s| s.to_string());
            break;
        }
    }
    
    let required_version = required_version
        .context("Could not find wasm-bindgen version in Cargo.lock")?;
    
    // Check installed version
    let check = Command::new("wasm-bindgen")
        .arg("--version")
        .output();
    
    let needs_install = if let Ok(output) = check {
        if output.status.success() {
            let installed_version = String::from_utf8_lossy(&output.stdout);
            let installed_version = installed_version.split_whitespace().last().unwrap_or("");
            installed_version != required_version
        } else {
            true
        }
    } else {
        true
    };
    
    if needs_install {
        println!("    Installing wasm-bindgen-cli {} (this may take a few minutes)...", required_version);
        let status = Command::new("cargo")
            .args(["install", "-f", "wasm-bindgen-cli", "--version", &required_version])
            .status()
            .context("Failed to install wasm-bindgen-cli")?;
        
        if !status.success() {
            anyhow::bail!("Failed to install wasm-bindgen-cli");
        }
        println!("  {} wasm-bindgen-cli {} installed", "✓".green(), required_version);
    } else {
        println!("  {} wasm-bindgen-cli {} already installed", "✓".green(), required_version);
    }
    
    Ok(())
}


/// Post-process UniFFI-generated Swift files to fix Swift 6 concurrency issues
/// This is a workaround for UniFFI issue #2818 until official support is added
/// See: https://github.com/mozilla/uniffi-rs/issues/2818
fn patch_swift_concurrency(platform_dir: &str) -> Result<()> {
    let dir = Path::new(platform_dir);
    if !dir.exists() {
        return Ok(());
    }
    
    // Find all Swift files generated by UniFFI
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if let Some(ext) = path.extension() {
            if ext == "swift" {
                let filename = path.file_name().unwrap().to_string_lossy();
                
                // Only patch UniFFI-generated files (skip user Swift files)
                // UniFFI generates files with patterns like: <crate_name>_ffi.swift or <crate_name>_core.swift
                if filename.ends_with("_ffi.swift") || filename.ends_with("_core.swift") {
                    patch_swift_file(&path)?;
                }
            }
        }
    }
    
    Ok(())
}

fn patch_swift_file(path: &Path) -> Result<()> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read Swift file: {}", path.display()))?;
    
    // Pattern: static let vtablePtr: UnsafePointer<...> = {
    // Replace with: nonisolated(unsafe) static let vtablePtr: UnsafePointer<...> = {
    let patched = content.replace(
        "static let vtablePtr: UnsafePointer<",
        "nonisolated(unsafe) static let vtablePtr: UnsafePointer<"
    );
    
    // Only write if changes were made
    if patched != content {
        fs::write(path, patched)
            .with_context(|| format!("Failed to write patched Swift file: {}", path.display()))?;
        
        println!("    {} Patched Swift 6 concurrency in {}", "✓".green(), path.file_name().unwrap().to_string_lossy());
    }
    
    Ok(())
}

fn resolve_version_code(
    configured_build_number: Option<u32>,
    package_version_code: Option<u32>,
    ci_run_number: Option<&str>,
) -> (u32, bool) {
    if let Some(number) = configured_build_number {
        return (number, false);
    }
    if let Some(number) = ci_run_number.and_then(|value| value.parse::<u32>().ok()) {
        return (number, true);
    }
    (package_version_code.unwrap_or(1), false)
}

fn resolve_android_target_sdk(
    configured_target_sdk: Option<u32>,
    bundle_compile_sdk: Option<u32>,
) -> u32 {
    configured_target_sdk.or(bundle_compile_sdk).unwrap_or(36)
}

pub fn sync_configs_to_platforms(config: &crate::config::Config) -> Result<()> {
    println!("  {} Syncing jffi.toml configurations to native platform builds...", "→".bright_blue());

    let version_name = config.bundle.as_ref()
        .and_then(|b| b.version.as_ref())
        .unwrap_or(&config.package.version);

    let configured_build_number = config.bundle.as_ref().and_then(|b| b.build_number);
    let ci_run_number = std::env::var("GITHUB_RUN_NUMBER").ok();
    let (version_code, used_ci_run_number) = resolve_version_code(
        configured_build_number,
        config.package.version_code,
        ci_run_number.as_deref(),
    );

    if used_ci_run_number {
        println!("    {} No build_number configured. Using CI run number: {}", "★".bright_magenta(), version_code);
    }

    // 1. Android
    let gradle_path = Path::new("platforms/android/app/build.gradle.kts");
    if gradle_path.exists() {
        let content = fs::read_to_string(gradle_path)?;
        
        let package = &config.platforms.android.package;
        
        // Parse the old package name from build.gradle.kts BEFORE rewriting it
        let old_package = content.lines()
            .find(|line| line.trim().starts_with("namespace ="))
            .and_then(|line| {
                let parts: Vec<&str> = line.split('=').collect();
                if parts.len() == 2 {
                    let val = parts[1].trim();
                    let trimmed_val = val.trim_matches(|c| c == '"' || c == '\'' || c == ' ' || c == ';' || c == ',');
                    Some(trimmed_val.to_string())
                } else {
                    None
                }
            });

        if let Some(ref old_pkg) = old_package {
            if old_pkg != package {
                rename_android_package(old_pkg, package)?;
            }
        }
        let min_sdk = config.platforms.android.min_sdk;
        let bundle_compile_sdk = config.bundle.as_ref()
            .and_then(|bundle| bundle.android.as_ref())
            .map(|android| android.compile_sdk);
        let target_sdk = resolve_android_target_sdk(
            config.platforms.android.target_sdk,
            bundle_compile_sdk,
        );
        let obfuscate = config.platforms.android.obfuscate;
        let shrink_resources = config.platforms.android.shrink_resources;
        
        let lines: Vec<String> = content.lines().map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("applicationId =") {
                let indent = line.len() - line.trim_start().len();
                format!("{:width$}applicationId = \"{}\"", "", package, width = indent)
            } else if trimmed.starts_with("namespace =") {
                let indent = line.len() - line.trim_start().len();
                format!("{:width$}namespace = \"{}\"", "", package, width = indent)
            } else if trimmed.starts_with("minSdk =") {
                let indent = line.len() - line.trim_start().len();
                format!("{:width$}minSdk = {}", "", min_sdk, width = indent)
            } else if trimmed.starts_with("targetSdk =") {
                let indent = line.len() - line.trim_start().len();
                format!("{:width$}targetSdk = {}", "", target_sdk, width = indent)
            } else if trimmed.starts_with("versionName =") {
                let indent = line.len() - line.trim_start().len();
                format!("{:width$}versionName = \"{}\"", "", version_name, width = indent)
            } else if trimmed.starts_with("versionCode =") {
                let indent = line.len() - line.trim_start().len();
                format!("{:width$}versionCode = {}", "", version_code, width = indent)
            } else if trimmed.starts_with("isMinifyEnabled =") {
                let indent = line.len() - line.trim_start().len();
                format!("{:width$}isMinifyEnabled = {}", "", obfuscate, width = indent)
            } else if trimmed.starts_with("isShrinkResources =") {
                let indent = line.len() - line.trim_start().len();
                format!("{:width$}isShrinkResources = {}", "", shrink_resources, width = indent)
            } else {
                line.to_string()
            }
        }).collect();
        
        let new_content = lines.join("\n");
        if new_content != content {
            fs::write(gradle_path, new_content)?;
            println!("    {} Synced Android gradle configurations (package: {})", "✓".green(), package);
        }
    }

    // 2. iOS/macOS Xcode Projects
    let bundle_id = config.bundle.as_ref()
        .and_then(|b| b.identifier.clone())
        .unwrap_or_else(|| config.platforms.ios.bundle_id.clone());

    let platforms = vec!["ios", "macos"];
    for platform_str in platforms {
        let platform_dir = format!("platforms/{}", platform_str);
        if !Path::new(&platform_dir).exists() {
            continue;
        }
        
        if let Ok(entries) = fs::read_dir(&platform_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("xcodeproj") {
                    let pbxproj_path = path.join("project.pbxproj");
                    if pbxproj_path.exists() {
                        let content = fs::read_to_string(&pbxproj_path)?;
                        
                        // Extract base bundle identifier to preserve extension suffixes
                        let mut all_ids = Vec::new();
                        for line in content.lines() {
                            let trimmed = line.trim();
                            if trimmed.starts_with("PRODUCT_BUNDLE_IDENTIFIER = ") {
                                if let Some(val) = trimmed.split('=').nth(1) {
                                    let id = val.trim().trim_end_matches(';').trim_matches('"').to_string();
                                    if !id.is_empty() {
                                        all_ids.push(id);
                                    }
                                }
                            }
                        }
                        all_ids.sort_by_key(|id| id.len());
                        let base_id = all_ids.first().cloned();

                        let lines: Vec<String> = content.lines().map(|line| {
                            let trimmed = line.trim_start();
                            if trimmed.starts_with("PRODUCT_BUNDLE_IDENTIFIER = ") {
                                let parts: Vec<&str> = line.split('=').collect();
                                if parts.len() == 2 {
                                    let indent = line.len() - trimmed.len();
                                    let existing_id = parts[1].trim().trim_end_matches(';').trim_matches('"');
                                    let final_id = if let Some(ref base) = base_id {
                                        if existing_id.starts_with(base) && existing_id.len() > base.len() {
                                            let suffix = &existing_id[base.len()..];
                                            format!("{}{}", bundle_id, suffix)
                                        } else {
                                            bundle_id.clone()
                                        }
                                    } else {
                                        bundle_id.clone()
                                    };
                                    format!("{:width$}PRODUCT_BUNDLE_IDENTIFIER = {};", "", final_id, width = indent)
                                } else {
                                    line.to_string()
                                }
                            } else if trimmed.starts_with("MARKETING_VERSION = ") {
                                let parts: Vec<&str> = line.split('=').collect();
                                if parts.len() == 2 {
                                    let indent = line.len() - trimmed.len();
                                    format!("{:width$}MARKETING_VERSION = {};", "", version_name, width = indent)
                                } else {
                                    line.to_string()
                                }
                            } else if trimmed.starts_with("CURRENT_PROJECT_VERSION = ") {
                                let parts: Vec<&str> = line.split('=').collect();
                                if parts.len() == 2 {
                                    let indent = line.len() - trimmed.len();
                                    format!("{:width$}CURRENT_PROJECT_VERSION = {};", "", version_code, width = indent)
                                } else {
                                    line.to_string()
                                }
                            } else {
                                line.to_string()
                            }
                        }).collect();
                        
                        let new_content = lines.join("\n");
                        if new_content != content {
                            fs::write(&pbxproj_path, new_content)?;
                            println!("    {} Synced {} Xcode bundle settings (id: {}, version: {}, build: {})", "✓".green(), platform_str, bundle_id, version_name, version_code);
                        }
                    }
                }
            }
        }
        if platform_str == "ios" {
            if let Some(ref groups) = config.platforms.ios.app_groups {
                let _ = sync_entitlements(Path::new("platforms/ios"), groups);
            }
        }
        if platform_str == "macos" {
            if let Some(ref groups) = config.platforms.macos.app_groups {
                let _ = sync_entitlements(Path::new("platforms/macos"), groups);
            }
        }
    }

    // 3. Web - sync SEO & meta tags to index.html
    let index_path = Path::new("platforms/web/index.html");
    if index_path.exists() {
        let web = &config.platforms.web;
        let name_pascal = config.package.name
            .split('-')
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<String>();
        
        let title = web.title.as_deref().unwrap_or(&name_pascal);
        let lang = &web.lang;
        
        let content = fs::read_to_string(index_path)?;
        let mut new_content = content.clone();
        
        // Sync lang attribute
        if new_content.contains("<html lang=\"") {
            let re_lang = format!("<html lang=\"{}\"", lang);
            // Replace the existing lang value
            if let Some(start) = new_content.find("<html lang=\"") {
                if let Some(end) = new_content[start + 12..].find('"') {
                    let old = &new_content[start..start + 12 + end + 1];
                    new_content = new_content.replace(old, &re_lang);
                }
            }
        }
        
        // Sync <title>
        if let (Some(start), Some(end)) = (new_content.find("<title>"), new_content.find("</title>")) {
            let old_title = &new_content[start..end + 8];
            new_content = new_content.replace(old_title, &format!("<title>{}</title>", title));
        }
        
        // Build meta tags to inject/replace inside <head>
        let mut meta_tags = Vec::new();
        
        if let Some(desc) = &web.description {
            meta_tags.push(format!("    <meta name=\"description\" content=\"{}\">", desc));
        }
        if let Some(keywords) = &web.keywords {
            meta_tags.push(format!("    <meta name=\"keywords\" content=\"{}\">", keywords));
        }
        if let Some(author) = &web.author {
            meta_tags.push(format!("    <meta name=\"author\" content=\"{}\">", author));
        }
        if let Some(theme_color) = &web.theme_color {
            meta_tags.push(format!("    <meta name=\"theme-color\" content=\"{}\">", theme_color));
        }
        if let Some(favicon) = &web.favicon {
            meta_tags.push(format!("    <link rel=\"icon\" href=\"{}\">", favicon));
        }
        
        // Open Graph tags
        meta_tags.push(format!("    <meta property=\"og:title\" content=\"{}\">", title));
        if let Some(desc) = &web.description {
            meta_tags.push(format!("    <meta property=\"og:description\" content=\"{}\">", desc));
        }
        if let Some(og_type) = &web.og_type {
            meta_tags.push(format!("    <meta property=\"og:type\" content=\"{}\">", og_type));
        }
        if let Some(og_url) = &web.og_url {
            meta_tags.push(format!("    <meta property=\"og:url\" content=\"{}\">", og_url));
        }
        if let Some(og_image) = &web.og_image {
            meta_tags.push(format!("    <meta property=\"og:image\" content=\"{}\">", og_image));
        }
        
        // Twitter Card tags
        if let Some(twitter_card) = &web.twitter_card {
            meta_tags.push(format!("    <meta name=\"twitter:card\" content=\"{}\">", twitter_card));
            meta_tags.push(format!("    <meta name=\"twitter:title\" content=\"{}\">", title));
            if let Some(desc) = &web.description {
                meta_tags.push(format!("    <meta name=\"twitter:description\" content=\"{}\">", desc));
            }
            if let Some(og_image) = &web.og_image {
                meta_tags.push(format!("    <meta name=\"twitter:image\" content=\"{}\">", og_image));
            }
        }
        
        // Remove existing JFFI-managed meta block (if present) and inject new one
        let marker_start = "    <!-- jffi:meta:start -->";
        let marker_end = "    <!-- jffi:meta:end -->";
        
        if let (Some(ms), Some(me)) = (new_content.find(marker_start), new_content.find(marker_end)) {
            // Replace existing block
            let old_block = &new_content[ms..me + marker_end.len()];
            let new_block = format!("{}\n{}\n{}", marker_start, meta_tags.join("\n"), marker_end);
            new_content = new_content.replace(old_block, &new_block);
        } else if !meta_tags.is_empty() {
            // Inject after the last <meta> tag in <head>, or before </head>
            if let Some(head_end) = new_content.find("</head>") {
                let new_block = format!("{}\n{}\n{}\n", marker_start, meta_tags.join("\n"), marker_end);
                new_content.insert_str(head_end, &new_block);
            }
        }
        
        if new_content != content {
            fs::write(index_path, new_content)?;
            println!("    {} Synced web SEO & meta tags (title: {})", "✓".green(), title);
        }
    }

    // 4. Rust Core Cargo.toml
    let cargo_path = Path::new("core/Cargo.toml");
    if cargo_path.exists() {
        let content = fs::read_to_string(cargo_path)?;
        let lines: Vec<String> = content.lines().map(|line| {
            if line.starts_with("version = \"") {
                format!("version = \"{}\"", version_name)
            } else {
                line.to_string()
            }
        }).collect();
        let new_content = lines.join("\n");
        if new_content != content {
            fs::write(cargo_path, new_content)?;
            println!("    {} Synced Rust core Cargo.toml version", "✓".green());
        }
    }

    // 5. Windows Package.appxmanifest
    let windows_path = Path::new("platforms/windows/Package.appxmanifest");
    if windows_path.exists() {
        let content = fs::read_to_string(windows_path)?;
        let parts: Vec<&str> = version_name.split('.').collect();
        let windows_version = if parts.len() == 3 {
            // Microsoft Store requires the revision (4th component) to always be 0.
            // Use version_code (GITHUB_RUN_NUMBER in CI) as the Build (3rd) component
            // so each CI run produces a unique, accepted version: Major.Minor.Build.0
            // e.g. version "1.6.1", CI run 97 → "1.6.97.0"
            format!("{}.{}.{}.0", parts[0], parts[1], version_code)
        } else {
            version_name.to_string()
        };
        let windows_app_id = get_windows_app_id(config);
        let mut inside_identity = false;
        let lines: Vec<String> = content.lines().map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("<Identity") {
                inside_identity = true;
            }
            
            let mut result = line.to_string();
            
            if inside_identity {
                if trimmed.starts_with("Name=\"") {
                    if let Some(start) = line.find("Name=\"") {
                        if let Some(end) = line[start + 6..].find('"') {
                            let prefix = &line[..start + 6];
                            let suffix = &line[start + 6 + end + 1..];
                            result = format!("{}{}\"{}", prefix, windows_app_id, suffix);
                        }
                    }
                }
                if trimmed.ends_with("/>") || trimmed.contains("</Identity>") {
                    inside_identity = false;
                }
            }
            
            if result.trim_start().starts_with("Version=\"") {
                if let Some(start) = result.find("Version=\"") {
                    if let Some(end) = result[start + 9..].find('"') {
                        let prefix = &result[..start + 9];
                        let suffix = &result[start + 9 + end + 1..];
                        return format!("{}{}\"{}", prefix, windows_version, suffix);
                    }
                }
            }
            result
        }).collect();
        let new_content = lines.join("\n");
        if new_content != content {
            fs::write(windows_path, new_content)?;
            println!("    {} Synced Windows Package.appxmanifest identity & version (id: {})", "✓".green(), windows_app_id);
        }
    }

    // 6. Linux GTK Application ID
    let linux_app_py = Path::new("platforms/linux/app.py");
    if linux_app_py.exists() {
        let content = fs::read_to_string(linux_app_py)?;
        let app_id = get_linux_app_id(config);
        
        let lines: Vec<String> = content.lines().map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("application_id=") || trimmed.starts_with("application_id =") {
                if let Some(start) = line.find("application_id") {
                    if let Some(_eq_pos) = line[start..].find('=') {
                        let indent = line.len() - line.trim_start().len();
                        return format!("{:width$}application_id='{}',", "", app_id, width = indent);
                    }
                }
            }
            line.to_string()
        }).collect();
        
        let new_content = lines.join("\n");
        if new_content != content {
            fs::write(linux_app_py, new_content)?;
            println!("    {} Synced Linux application ID (id: {})", "✓".green(), app_id);
        }
    }

    Ok(())
}

fn find_ndk_dir() -> Option<std::path::PathBuf> {
    if let Ok(ndk_home) = std::env::var("ANDROID_NDK_HOME") {
        let path = std::path::PathBuf::from(ndk_home);
        if path.exists() { return Some(path); }
    }
    
    // Fallback to checking SDK paths
    let sdk_roots = vec![
        std::env::var("ANDROID_HOME").ok(),
        std::env::var("ANDROID_SDK_ROOT").ok(),
        std::env::var("HOME").ok().map(|h| format!("{}/Library/Android/sdk", h)),
        std::env::var("HOME").ok().map(|h| format!("{}/Android/Sdk", h)),
    ];
    
    for sdk_root in sdk_roots.into_iter().flatten() {
        let ndk_base = std::path::PathBuf::from(&sdk_root).join("ndk");
        if ndk_base.exists() {
            if let Ok(entries) = std::fs::read_dir(&ndk_base) {
                let mut versions: Vec<std::path::PathBuf> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| p.is_dir())
                    .collect();
                versions.sort();
                if let Some(latest) = versions.last() {
                    return Some(latest.clone());
                }
            }
        }
        
        let ndk_bundle = std::path::PathBuf::from(&sdk_root).join("ndk-bundle");
        if ndk_bundle.exists() { return Some(ndk_bundle); }
    }
    None
}

fn find_libcxx_shared(ndk_dir: &Path, target: &str) -> Option<std::path::PathBuf> {
    let target_sub = match target {
        "aarch64-linux-android" => "aarch64-linux-android",
        "armv7-linux-androideabi" => "arm-linux-androideabi",
        "x86_64-linux-android" => "x86_64-linux-android",
        "i686-linux-android" => "i686-linux-android",
        _ => target,
    };
    
    let hosts = vec!["darwin-x86_64", "linux-x86_64", "windows-x86_64", "windows"];
    for host in hosts {
        let path = ndk_dir
            .join("toolchains/llvm/prebuilt")
            .join(host)
            .join("sysroot/usr/lib")
            .join(target_sub)
            .join("libc++_shared.so");
        if path.exists() {
            return Some(path);
        }
    }
    None
}

fn get_linux_app_id(config: &crate::config::Config) -> String {
    if let Some(ref bundle) = config.bundle {
        if let Some(ref linux) = bundle.linux {
            if let Some(ref app_id) = linux.app_id {
                return app_id.clone();
            }
        }
        if let Some(ref id) = bundle.identifier {
            return id.clone();
        }
    }
    if !config.platforms.android.package.is_empty() && config.platforms.android.package != "com.example.app" {
        return config.platforms.android.package.clone();
    }
    if !config.platforms.ios.bundle_id.is_empty() && config.platforms.ios.bundle_id != "com.example.app" {
        return config.platforms.ios.bundle_id.clone();
    }
    format!("com.example.{}", config.package.name.replace("-", ""))
}

fn get_windows_app_id(config: &crate::config::Config) -> String {
    if let Some(ref bundle) = config.bundle {
        if let Some(ref windows) = bundle.windows {
            if let Some(ref identity_name) = windows.identity_name {
                return identity_name.clone();
            }
        }
        if let Some(ref id) = bundle.identifier {
            return id.clone();
        }
    }
    if !config.platforms.android.package.is_empty() && config.platforms.android.package != "com.example.app" {
        return config.platforms.android.package.clone();
    }
    if !config.platforms.ios.bundle_id.is_empty() && config.platforms.ios.bundle_id != "com.example.app" {
        return config.platforms.ios.bundle_id.clone();
    }
    format!("com.example.{}", config.package.name.replace("-", ""))
}

fn rename_android_package(old_pkg: &str, new_pkg: &str) -> Result<()> {
    use std::path::PathBuf;
    let src_dir = Path::new("platforms/android/app/src");
    if !src_dir.exists() {
        return Ok(());
    }

    println!("    {} Android package name change detected: {} → {}", "→".bright_blue(), old_pkg, new_pkg);

    // 1. Walk the directory and find all .kt and .java and .xml files
    let walk_dir = |dir: &Path| -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let mut queue = vec![dir.to_path_buf()];
        while let Some(path) = queue.pop() {
            if path.is_dir() {
                for entry in fs::read_dir(path)? {
                    let entry = entry?;
                    queue.push(entry.path());
                }
            } else {
                let ext = path.extension().and_then(|s| s.to_str());
                if ext == Some("kt") || ext == Some("java") || ext == Some("xml") {
                    files.push(path);
                }
            }
        }
        Ok(files)
    };

    let files = walk_dir(src_dir)?;
    for file in &files {
        let content = fs::read_to_string(file)?;
        if content.contains(old_pkg) {
            let new_content = content.replace(old_pkg, new_pkg);
            fs::write(file, new_content)?;
        }
    }

    // 2. Relocate files.
    if let Ok(entries) = fs::read_dir(src_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                for lang_dir_name in &["java", "kotlin"] {
                    let lang_dir = path.join(lang_dir_name);
                    if lang_dir.is_dir() {
                        let old_pkg_dir = lang_dir.join(old_pkg.replace('.', "/"));
                        let new_pkg_dir = lang_dir.join(new_pkg.replace('.', "/"));
                        
                        if old_pkg_dir.exists() && old_pkg_dir != new_pkg_dir {
                            fs::create_dir_all(&new_pkg_dir)?;
                            move_dir_contents(&old_pkg_dir, &new_pkg_dir)?;
                            clean_empty_parent_directories(&old_pkg_dir, &lang_dir)?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn move_dir_contents(src: &Path, dst: &Path) -> Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let target = dst.join(name);
        
        if path.is_dir() {
            fs::create_dir_all(&target)?;
            move_dir_contents(&path, &target)?;
            fs::remove_dir(&path)?;
        } else {
            if target.exists() {
                fs::remove_file(&target)?;
            }
            fs::rename(&path, &target)?;
        }
    }
    Ok(())
}

fn clean_empty_parent_directories(leaf: &Path, stop_at: &Path) -> Result<()> {
    let mut current = leaf.to_path_buf();
    while current != stop_at {
        if current.exists() && current.is_dir() {
            if fs::read_dir(&current)?.next().is_none() {
                fs::remove_dir(&current)?;
            } else {
                break;
            }
        }
        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            break;
        }
    }
    Ok(())
}

fn sync_entitlements(dir: &Path, groups: &[String]) -> Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                sync_entitlements(&path, groups)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("entitlements") {
                update_entitlements_file(&path, groups)?;
            }
        }
    }
    Ok(())
}

fn update_entitlements_file(path: &Path, app_groups: &[String]) -> Result<()> {
    let content = fs::read_to_string(path)?;
    let mut groups_xml = String::new();
    groups_xml.push_str("\t<key>com.apple.security.application-groups</key>\n\t<array>\n");
    for group in app_groups {
        groups_xml.push_str(&format!("\t\t<string>{}</string>\n", group));
    }
    groups_xml.push_str("\t</array>\n");

    let new_content = if content.contains("com.apple.security.application-groups") {
        if let Some(key_idx) = content.find("<key>com.apple.security.application-groups</key>") {
            if let Some(array_start) = content[key_idx..].find("<array>") {
                if let Some(array_end) = content[key_idx + array_start..].find("</array>") {
                    let end_pos = key_idx + array_start + array_end + "</array>".len();
                    let mut res = content[..key_idx].to_string();
                    res.push_str(&groups_xml);
                    res.push_str(&content[end_pos..]);
                    res
                } else {
                    content.clone()
                }
            } else {
                content.clone()
            }
        } else {
            content.clone()
        }
    } else {
        if let Some(dict_end) = content.rfind("</dict>") {
            let mut res = content[..dict_end].to_string();
            res.push_str(&groups_xml);
            res.push_str(&content[dict_end..]);
            res
        } else {
            content.clone()
        }
    };

    if new_content != content {
        fs::write(path, new_content)?;
    }
    Ok(())
}

#[cfg(test)]
mod config_sync_tests {
    use super::{resolve_android_target_sdk, resolve_version_code};

    #[test]
    fn explicit_build_number_wins_over_ci_run_number() {
        assert_eq!(resolve_version_code(Some(83), Some(7), Some("91")), (83, false));
    }

    #[test]
    fn ci_run_number_is_a_fallback_when_build_number_is_absent() {
        assert_eq!(resolve_version_code(None, Some(7), Some("91")), (91, true));
    }

    #[test]
    fn invalid_ci_run_number_falls_back_to_package_version_code() {
        assert_eq!(resolve_version_code(None, Some(7), Some("not-a-number")), (7, false));
    }

    #[test]
    fn android_target_prefers_explicit_target_then_compile_sdk() {
        assert_eq!(resolve_android_target_sdk(Some(34), Some(36)), 34);
        assert_eq!(resolve_android_target_sdk(None, Some(36)), 36);
        assert_eq!(resolve_android_target_sdk(None, None), 36);
    }
}
