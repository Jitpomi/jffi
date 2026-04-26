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
    
    if !Path::new("core").exists() {
        anyhow::bail!(
            "{}\n\n{}",
            "Error: Missing 'core' directory.".red().bold(),
            "Your project structure appears incomplete. Expected directories: core/, platforms/"
        );
    }
    
    Ok(())
}

fn ensure_rust_targets(targets: &[&str]) -> Result<()> {
    let output = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .context("Failed to run rustup. Please install Rust with rustup.")?;

    if !output.status.success() {
        anyhow::bail!("Failed to check installed Rust targets via rustup");
    }

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

    let status = Command::new("rustup")
        .arg("target")
        .arg("add")
        .args(&missing)
        .status()
        .context("Failed to install required Rust targets via rustup")?;

    if !status.success() {
        anyhow::bail!("Failed to install Rust targets: {}", missing.join(", "));
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
        "windows" | "windows-x64" => build_windows("x86_64", release),
        "windows-x86" => build_windows("i686", release),
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
    let profile = if release { "release" } else { "debug" };
    
    // Always build for all architectures to ensure Xcode compatibility
    // (Xcode may build for device even when running on simulator)
    let targets = vec![
        ("aarch64-apple-ios-sim", "iOS Simulator (ARM64)"),
        ("x86_64-apple-ios", "iOS Simulator (x86_64)"),
        ("aarch64-apple-ios", "iOS Device"),
    ];
    
    let target_names: Vec<&str> = targets.iter().map(|(t, _)| *t).collect();
    ensure_rust_targets(&target_names)?;
    
    // Build for each target
    for (target, target_name) in &targets {
        println!("  {} Building Rust library for {}...", "→".bright_blue(), target_name);
        let mut args = vec!["build"];
        if release {
            args.push("--release");
        }
        args.extend(&["--manifest-path", "core/Cargo.toml", "--target", target]);
        
        let status = Command::new("cargo")
            .env("CARGO_TARGET_DIR", "target")
            .args(&args)
            .status()
            .context(format!("Failed to build Rust library for {}", target))?;
        
        if !status.success() {
            anyhow::bail!("Rust build failed for {}", target);
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
    
    std::fs::create_dir_all("target/ios-simulator-universal")?;
    
    let lipo_status = Command::new("lipo")
        .args(&[
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
        .args(&[
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

fn build_android(release: bool) -> Result<()> {
    println!("  {} Building Rust library for Android...", "→".bright_blue());
    
    // Build for all Android architectures
    let architectures = vec![
        ("aarch64-linux-android", "arm64-v8a"),
        ("armv7-linux-androideabi", "armeabi-v7a"),
        ("x86_64-linux-android", "x86_64"),
    ];
    
    // Check if Android targets are installed
    let targets: Vec<&str> = architectures.iter().map(|(t, _)| *t).collect();
    ensure_android_targets(&targets)?;
    
    // Check if cargo-ndk is installed
    ensure_cargo_ndk()?;
    
    let profile = if release { "release" } else { "debug" };
    
    for (target, abi) in &architectures {
        println!("    Building for {} ({})", abi, target);
        println!("    Building {} ({})", abi, target);
        let mut args = vec!["ndk", "-t", target, "-o", "platforms/android/app/src/main/jniLibs", "build"];
        if release {
            args.push("--release");
        }
        args.extend(&["--manifest-path", "core/Cargo.toml"]);
        
        let status = Command::new("cargo")
            .env("CARGO_TARGET_DIR", "target")
            .args(&args)
            .status()
            .context(format!("Failed to build for {}", target))?;
        
        if !status.success() {
            anyhow::bail!("Rust build failed for {}", target);
        }
    }
    
    // Find the library file for binding generation
    let lib_dir = format!("target/aarch64-linux-android/{}", profile);
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
    
    // Generate Kotlin bindings using uniffi-bindgen
    let status = Command::new("uniffi-bindgen")
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

fn build_macos(_arch: &str, release: bool) -> Result<()> {
    build_macos_xcframework(release)
}

fn build_macos_xcframework(release: bool) -> Result<()> {
    let profile = if release { "release" } else { "debug" };
    
    // Always build for both architectures to ensure compatibility
    let targets = vec![
        ("aarch64-apple-darwin", "macOS Apple Silicon"),
        ("x86_64-apple-darwin", "macOS Intel"),
    ];
    
    let target_names: Vec<&str> = targets.iter().map(|(t, _)| *t).collect();
    ensure_rust_targets(&target_names)?;
    
    // Build for each target
    for (target, target_name) in &targets {
        println!("  {} Building Rust library for {}...", "→".bright_blue(), target_name);
        let mut args = vec!["build"];
        if release {
            args.push("--release");
        }
        args.extend(&["--manifest-path", "core/Cargo.toml", "--target", target]);
        
        let status = Command::new("cargo")
            .env("CARGO_TARGET_DIR", "target")
            .args(&args)
            .status()
            .context(format!("Failed to build Rust library for {}", target))?;
        
        if !status.success() {
            anyhow::bail!("Rust build failed for {}", target);
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
        .args(&[
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
        .args(&[
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

fn build_windows(arch: &str, release: bool) -> Result<()> {
    // Ensure uniffi-bindgen-cs is installed
    ensure_uniffi_bindgen_cs()?;
    
    let profile = if release { "release" } else { "debug" };
    let target = format!("{}-pc-windows-msvc", arch);
    
    println!("  {} Building Rust library for Windows ({})...", "→".bright_blue(), arch);
    
    let mut args = vec!["build"];
    if release {
        args.push("--release");
    }
    args.extend(&["--target", &target, "--manifest-path", "core/Cargo.toml"]);
    
    let status = Command::new("cargo")
        .env("CARGO_TARGET_DIR", "target")
        .args(&args)
        .status()
        .context("Failed to build Rust library")?;
    
    if !status.success() {
        anyhow::bail!("Rust build failed");
    }
    
    println!("  {} Generating C# bindings with uniffi-bindgen-cs...", "→".bright_blue());
    
    // Use library mode to generate bindings directly from the compiled DLL
    // On Windows, Rust builds cdylib as <name>.dll (not lib<name>.dll)
    let lib_name = std::env::current_dir()?
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("core")
        .replace("-", "_");
    let lib_path = format!("target/{}/{}/{}_core.dll", target, profile, lib_name);
    
    let status = Command::new("uniffi-bindgen-cs")
        .arg("--library")
        .arg(&lib_path)
        .arg("--out-dir")
        .arg("platforms/windows")
        .status()
        .context("Failed to run uniffi-bindgen-cs")?;
    
    if !status.success() {
        anyhow::bail!("C# bindings generation failed");
    }
    
    // Copy the .dll to platforms/windows for the build
    let lib_path = format!("target/{}/{}/{}_core.dll", target, profile, lib_name);
    let dll_name = format!("{}_core.dll", lib_name);
    if std::path::Path::new(&lib_path).exists() {
        std::fs::copy(&lib_path, format!("platforms/windows/{}", dll_name))
            .context("Failed to copy DLL to platforms/windows")?;
    }
    
    println!("  {} Building C# project with MSBuild...", "→".bright_blue());
    
    // Find the .csproj file
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
    
    // Prefer `dotnet build` (works with WinUI 3 projects).
    // If tools aren't on PATH, try common install locations / VS discovery.
    let dotnet_cmd = find_dotnet();
    let msbuild_cmd = find_msbuild();
    let build_cmd: &str = if dotnet_cmd.is_some() { "dotnet" } else { "msbuild" };
    
    let mut build_args: Vec<&str> = Vec::new();
    if build_cmd == "dotnet" {
        build_args.push("build");
        build_args.push(csproj_file.to_str().unwrap());
        // WinUI 3 requires a specific platform, not AnyCPU
        if release {
            build_args.extend(&["-c", "Release"]);
        }
    } else {
        // MSBuild: project file is the first positional argument (no 'build' subcommand)
        build_args.push(csproj_file.to_str().unwrap());
        if release {
            build_args.extend(&["/p:Configuration=Release"]);
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
        anyhow::bail!("C# build failed");
    }

    // Copy the FFI DLL to the output directory after build
    println!("  {} Copying FFI DLL to output directory...", "→".bright_blue());
    let config = if release { "Release" } else { "Debug" };
    let output_dir = crate::platform::windows::output_dir(
        crate::platform::windows::DEFAULT_PLATFORM,
        &config,
    );
    if std::path::Path::new(&output_dir).exists() {
        let dll_source = format!("platforms/windows/{}_core.dll", lib_name);
        let dll_dest = format!("{}/{}_core.dll", output_dir, lib_name);
        if std::path::Path::new(&dll_source).exists() {
            std::fs::copy(&dll_source, &dll_dest)
                .context("Failed to copy FFI DLL to output directory")?;
            println!("  {} Copied {} to output directory", "✓".green(), format!("{}_core.dll", lib_name));
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
        .args(&[
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

// Note: uniffi-bindgen is now run via 'cargo run --bin uniffi-bindgen' from the ffi directory
// This uses the project's own uniffi-bindgen binary defined in core/Cargo.toml

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

        // Prefer an explicit tag for repeatable installs; users can override to a fork/branch
        // that matches their UniFFI version.
        let tag = std::env::var("JFFI_UNIFFI_BINDGEN_CS_TAG").unwrap_or_else(|_| "v0.10.0+v0.29.4".to_string());
        let branch = std::env::var("JFFI_UNIFFI_BINDGEN_CS_BRANCH").ok();

        println!("    Installing uniffi-bindgen-cs ({} @ {})...", repo, branch.as_deref().unwrap_or(&tag));
        println!("    {} If you use UniFFI v0.31+, you may need a newer uniffi-bindgen-cs fork; set JFFI_UNIFFI_BINDGEN_CS_GIT/JFFI_UNIFFI_BINDGEN_CS_TAG.", "⚠".yellow());

        let mut cmd = Command::new("cargo");
        cmd.env("CARGO_NET_GIT_FETCH_WITH_CLI", "true")
            .args(&["install", "uniffi-bindgen-cs", "--git", &repo]);
        if let Some(branch) = branch.as_deref() {
            cmd.args(&["--branch", branch]);
        } else {
            cmd.args(&["--tag", &tag]);
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
    let profile = if release { "release" } else { "debug" };
    
    println!("  {} Building Rust library for Linux...", "→".bright_blue());
    
    let mut args = vec!["build"];
    if release {
        args.push("--release");
    }
    args.extend(&["--manifest-path", "core/Cargo.toml"]);
    
    let status = Command::new("cargo")
        .args(&args)
        .status()
        .context("Failed to build Rust library")?;
    
    if !status.success() {
        anyhow::bail!("Rust build failed");
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
    
    // Use cargo run with uniffi/cli feature from the ffi crate
    let status = Command::new("cargo")
        .args(&[
            "run",
            "--manifest-path",
            "core/Cargo.toml",
            "--features",
            "uniffi/cli",
            "--bin",
            "uniffi-bindgen",
            "--",
        ])
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
    // Ensure wasm32-unknown-unknown target is installed
    ensure_wasm_target()?;
    
    // Ensure wasm-bindgen-cli is installed
    ensure_wasm_bindgen_cli()?;
    
    println!("  {} Building Rust library for Web (WASM)...", "→".bright_blue());
    
    let profile = if release { "release" } else { "debug" };
    
    // Build the ffi-web crate for wasm32
    let mut args = vec!["build", "--target", "wasm32-unknown-unknown"];
    if release {
        args.push("--release");
    }
    args.extend(&["--manifest-path", "ffi-web/Cargo.toml"]);
    
    let status = Command::new("cargo")
        .args(&args)
        .status()
        .context("Failed to build Rust library for WASM")?;
    
    if !status.success() {
        anyhow::bail!("Rust WASM build failed");
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
    
    // Run wasm-bindgen to generate JS bindings
    let status = Command::new("wasm-bindgen")
        .arg(wasm_file.to_str().unwrap())
        .arg("--out-dir")
        .arg("platforms/web/pkg")
        .arg("--target")
        .arg("web")
        .arg("--out-name")
        .arg("wasm")
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
        .args(&["target", "list", "--installed"])
        .output()
        .context("Failed to check installed targets")?;
    
    let installed = String::from_utf8_lossy(&output.stdout);
    
    if !installed.contains("wasm32-unknown-unknown") {
        println!("    Installing wasm32-unknown-unknown...");
        let status = Command::new("rustup")
            .args(&["target", "add", "wasm32-unknown-unknown"])
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
    
    // Get the wasm-bindgen version from ffi-web/Cargo.toml
    let cargo_toml = std::fs::read_to_string("ffi-web/Cargo.toml")
        .context("Failed to read ffi-web/Cargo.toml")?;
    
    let required_version = cargo_toml
        .lines()
        .find(|line| line.contains("wasm-bindgen ="))
        .and_then(|line| {
            line.split('"').nth(1)
        })
        .unwrap_or("0.2");
    
    // Check installed version
    let check = Command::new("wasm-bindgen")
        .arg("--version")
        .output();
    
    let needs_install = if let Ok(output) = check {
        if output.status.success() {
            let installed_version = String::from_utf8_lossy(&output.stdout);
            let installed_version = installed_version.trim().split_whitespace().last().unwrap_or("");
            
            // Check if versions match
            !installed_version.starts_with(required_version)
        } else {
            true
        }
    } else {
        true
    };
    
    if needs_install {
        println!("    Installing wasm-bindgen-cli {} (this may take a few minutes)...", required_version);
        let status = Command::new("cargo")
            .args(&["install", "-f", "wasm-bindgen-cli", "--version", required_version])
            .status()
            .context("Failed to install wasm-bindgen-cli")?;
        
        if !status.success() {
            anyhow::bail!("Failed to install wasm-bindgen-cli");
        }
        println!("  {} wasm-bindgen-cli {} installed", "✓".green(), required_version);
    }
    
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
