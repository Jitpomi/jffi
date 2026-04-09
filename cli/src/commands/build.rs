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
    build_ios_target(release, "simulator")
}

fn build_ios_target(release: bool, target_type: &str) -> Result<()> {
    let profile = if release { "release" } else { "debug" };
    
    // Choose target based on device vs simulator
    let (target, target_name) = match target_type {
        "device" => ("aarch64-apple-ios", "iOS Device"),
        "simulator" | _ => {
            let target = if cfg!(target_arch = "aarch64") {
                "aarch64-apple-ios-sim"
            } else {
                "x86_64-apple-ios"
            };
            (target, "iOS Simulator")
        }
    };

    ensure_rust_targets(&[target])?;
    
    println!("  {} Building Rust library for {}...", "→".bright_blue(), target_name);
    let mut args = vec!["build"];
    if release {
        args.push("--release");
    }
    args.extend(&["--manifest-path", "ffi/Cargo.toml", "--target", target]);
    
    let status = Command::new("cargo")
        .env("CARGO_TARGET_DIR", "target")
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
    
    let status = Command::new("cargo")
        .env("CARGO_TARGET_DIR", "target")
        .args(&[
            "run",
            "--manifest-path",
            "ffi/Cargo.toml",
            "--bin",
            "uniffi-bindgen",
            "--",
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
        args.extend(&["--manifest-path", "ffi/Cargo.toml"]);
        
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
            name_str.starts_with("lib") && name_str.ends_with("ffi.so")
        })
        .map(|e| e.path())
        .context("Could not find FFI library")?;
    
    let status = Command::new("cargo")
        .env("CARGO_TARGET_DIR", "target")
        .args(&[
            "run",
            "--manifest-path",
            "ffi/Cargo.toml",
            "--bin",
            "uniffi-bindgen",
            "--",
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

    ensure_rust_targets(&[&target])?;
    
    println!("  {} Building Rust library for macOS ({})...", "→".bright_blue(), arch);
    let mut args = vec!["build"];
    if release {
        args.push("--release");
    }
    args.extend(&["--manifest-path", "ffi/Cargo.toml", "--target", &target]);
    
    let status = Command::new("cargo")
        .env("CARGO_TARGET_DIR", "target")
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
    
    let status = Command::new("cargo")
        .env("CARGO_TARGET_DIR", "target")
        .args(&[
            "run",
            "--manifest-path",
            "ffi/Cargo.toml",
            "--bin",
            "uniffi-bindgen",
            "--",
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
    // Ensure uniffi-bindgen-cs is installed
    ensure_uniffi_bindgen_cs()?;
    
    let profile = if release { "release" } else { "debug" };
    let target = format!("{}-pc-windows-msvc", arch);
    
    println!("  {} Building Rust library for Windows ({})...", "→".bright_blue(), arch);
    
    let mut args = vec!["build"];
    if release {
        args.push("--release");
    }
    args.extend(&["--target", &target, "--manifest-path", "ffi/Cargo.toml"]);
    
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
        .unwrap_or("ffi")
        .replace("-", "_");
    let lib_path = format!("target/{}/{}/{}_ffi.dll", target, profile, lib_name);
    
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
    let lib_path = format!("target/{}/{}/{}_ffi.dll", target, profile, lib_name);
    let dll_name = format!("{}_ffi.dll", lib_name);
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
    
    // Build with MSBuild (or dotnet build)
    let build_cmd = if Command::new("dotnet").arg("--version").output().is_ok() {
        "dotnet"
    } else {
        "msbuild"
    };
    
    let mut build_args = vec!["build"];
    if build_cmd == "dotnet" {
        build_args.push(csproj_file.to_str().unwrap());
        // WinUI 3 requires a specific platform, not AnyCPU
        build_args.extend(&["-p:Platform=x64"]);
        // Make it self-contained to include WinUI 3 runtime
        build_args.extend(&["-p:WindowsAppSDKSelfContained=true"]);
        if release {
            build_args.extend(&["-c", "Release"]);
        }
    } else {
        build_args.push(csproj_file.to_str().unwrap());
        build_args.extend(&["/p:Platform=x64"]);
        build_args.extend(&["/p:WindowsAppSDKSelfContained=true"]);
        if release {
            build_args.extend(&["/p:Configuration=Release"]);
        }
    }
    
    let status = Command::new(build_cmd)
        .args(&build_args)
        .status()
        .context(format!("Failed to build with {}", build_cmd))?;
    
    if !status.success() {
        anyhow::bail!("C# build failed");
    }
    
    // Copy the FFI DLL to the output directory after build
    println!("  {} Copying FFI DLL to output directory...", "→".bright_blue());
    let output_dir = "platforms/windows/bin/x64/Debug/net8.0-windows10.0.19041.0";
    if std::path::Path::new(output_dir).exists() {
        let dll_source = format!("platforms/windows/{}_ffi.dll", lib_name);
        let dll_dest = format!("{}/{}_ffi.dll", output_dir, lib_name);
        if std::path::Path::new(&dll_source).exists() {
            std::fs::copy(&dll_source, &dll_dest)
                .context("Failed to copy FFI DLL to output directory")?;
            println!("  {} Copied {} to output directory", "✓".green(), format!("{}_ffi.dll", lib_name));
        }
    }
    
    println!("{}", "  ✅ Windows build complete".green());
    Ok(())
}

// Note: uniffi-bindgen is now run via 'cargo run --bin uniffi-bindgen' from the ffi directory
// This uses the project's own uniffi-bindgen binary defined in ffi/Cargo.toml

fn ensure_uniffi_bindgen_cs() -> Result<()> {
    println!("  {} Checking uniffi-bindgen-cs...", "→".bright_blue());
    
    let check = Command::new("uniffi-bindgen-cs")
        .arg("--version")
        .output();
    
    if check.is_err() || !check.unwrap().status.success() {
        println!("    Installing uniffi-bindgen-cs from main branch (this may take a few minutes)...");
        println!("    {} Note: uniffi-bindgen-cs targets UniFFI v0.29.4, but JFFI uses v0.31.0", "⚠".yellow());
        println!("    {} This may cause compatibility issues", "⚠".yellow());
        let status = Command::new("cargo")
            .args(&["install", "uniffi-bindgen-cs", "--git", "https://github.com/microsoft/uniffi-bindgen-cs", "--branch", "main"])
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
    args.extend(&["--manifest-path", "ffi/Cargo.toml"]);
    
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
            name_str.starts_with("lib") && name_str.ends_with("ffi.so")
        })
        .map(|e| e.path())
        .context("Could not find FFI library")?;
    
    // Use cargo run with uniffi/cli feature from the ffi crate
    let status = Command::new("cargo")
        .args(&[
            "run",
            "--manifest-path",
            "ffi/Cargo.toml",
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
