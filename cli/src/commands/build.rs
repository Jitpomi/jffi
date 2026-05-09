use anyhow::{Context, Result};
use colored::*;
use std::process::Command;
use std::path::Path;
use std::fs;

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

pub fn build_project(platform: Option<String>, all: bool, release: bool, device: bool, deploy: bool) -> Result<()> {
    validate_project_structure()?;
    
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
    
    let target_names: Vec<&str> = targets.iter().map(|(t, _)| *t).collect();
    ensure_rust_targets(&target_names)?;
    
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
        .args(&["tree", "-i", "ndk-context", "--target", "aarch64-linux-android", "--depth", "10"])
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
    
    if verbose {
        // Verbose mode: no progress bars, just plain output
        for (target, abi) in &architectures {
            println!("  {} Building Android {}...", "→".bright_blue(), abi);
            
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
            
            let status = Command::new("cargo")
                .env("CARGO_TARGET_DIR", "target")
                .args(&args)
                .status()
                .context(format!("Failed to build for {}", target))?;
            
            if !status.success() {
                pb.finish_with_message(format!("{} Android {}", "✗".red(), abi));
                anyhow::bail!("Rust build failed for {}", target);
            }
            
            pb.finish_with_message(format!("{} Android {}", "✓".green(), abi));
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

fn generate_android_ndk_bridge() -> Result<()> {
    // Get package name from AndroidManifest.xml
    let manifest_path = "platforms/android/app/src/main/AndroidManifest.xml";
    let manifest_content = std::fs::read_to_string(manifest_path)
        .context("Failed to read AndroidManifest.xml")?;
    
    let package_name = manifest_content
        .lines()
        .find(|line| line.contains("package="))
        .and_then(|line| {
            line.split("package=\"")
                .nth(1)?
                .split('"')
                .next()
        })
        .context("Could not find package name in AndroidManifest.xml")?;
    
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
    mut env: JNIEnv,
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
    
    // Always build for both architectures to ensure compatibility
    let targets = vec![
        ("aarch64-apple-darwin", "macOS Apple Silicon"),
        ("x86_64-apple-darwin", "macOS Intel"),
    ];
    
    let target_names: Vec<&str> = targets.iter().map(|(t, _)| *t).collect();
    ensure_rust_targets(&target_names)?;
    
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
                .env("MACOSX_DEPLOYMENT_TARGET", "13.0")
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
                .env("MACOSX_DEPLOYMENT_TARGET", "13.0")
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
    // Check if required C compiler is available for the target architecture
    match arch {
        "aarch64" => {
            // aarch64-pc-windows-msvc requires clang for C dependencies like ring
            let has_clang = Command::new("clang")
                .arg("--version")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            Ok(has_clang)
        }
        "x86_64" | "i686" => {
            // x86/x64 use MSVC's cl.exe which is typically available
            let has_msvc = Command::new("cl")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            Ok(has_msvc)
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
            if release {
                build_args.extend(["-c".to_string(), "Release".to_string()]);
            }
        } else {
            build_args.push(csproj_file.to_string_lossy().into_owned());
            build_args.push(format!("/p:Platform={}", platform));
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
            .args(&["install", "uniffi", "--features", "cli", "--bin", "uniffi-bindgen", "--version", "0.31.1"])
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
    println!("{} Building Linux project...", "→".bright_blue());

    let profile = if release { "release" } else { "debug" };

    // Check for Python dependencies
    println!("  {} Checking dependencies...", "→".bright_blue());
    let python_check = Command::new("python3")
        .args(&["-c", "import gi; gi.require_version('Gtk', '4.0')"])
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
        
        let status = Command::new("cargo")
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
        
        let status = Command::new("cargo")
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
            let installed_version = installed_version.trim().split_whitespace().last().unwrap_or("");
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
            .args(&["install", "-f", "wasm-bindgen-cli", "--version", &required_version])
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
