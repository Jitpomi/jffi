use anyhow::{Context, Result};
use colored::*;
use std::process::Command;

use crate::platform::Platform;

pub fn run_project(platform_str: &str, device: bool) -> Result<()> {
    let platform = Platform::from_str(platform_str)
        .ok_or_else(|| anyhow::anyhow!("Unknown platform: {}", platform_str))?;
    
    let target_desc = if device && platform == Platform::Ios { "iOS Device" } else { platform.as_str() };
    println!("{}", format!("🚀 Running on {}...", target_desc).bright_green().bold());
    println!();
    
    // Build first
    crate::commands::build::build_project(Some(platform_str.to_string()), false, false, device, false)?;
    
    println!();
    println!("{}", format!("▶️  Launching {}...", target_desc).bright_cyan().bold());
    
    match platform {
        Platform::Ios if device => run_ios_device(),
        Platform::Ios => run_ios(),
        Platform::Macos => run_macos(),
        Platform::Android => run_android(),
        Platform::Windows => run_windows(),
        Platform::Linux => run_linux(),
        Platform::Web => run_web(),
    }
}

fn run_ios_device() -> Result<()> {
    use crate::platform::XcodeProject;
    
    println!("  {} Finding Xcode project...", "→".bright_blue());
    let project = XcodeProject::find(crate::platform::Platform::Ios)?;
    
    println!("  {} Building and deploying to device...", "→".bright_blue());
    println!();
    println!("{}", "  Note: Make sure your device is connected and trusted.".yellow());
    println!("{}", "  You may need to configure code signing in Xcode first.".yellow());
    println!();
    
    project.build("generic/platform=iOS")
        .context("Build failed. Make sure code signing is configured in Xcode.")?;
    
    println!();
    println!("{}", "  ✅ Build complete!".green());
    println!();
    println!("{}", "  To deploy to your device:".bright_cyan());
    println!("  1. Open Xcode");
    println!("  2. Select your connected device");
    println!("  3. Press Cmd+R to run");
    println!();
    println!("{}", "  Or use Xcode directly for automatic deployment.".bright_cyan());
    
    Ok(())
}

fn run_ios() -> Result<()> {
    use crate::platform::{IOSSimulator, XcodeProject, find_ios_app_bundle};
    
    println!("  {} Finding Xcode project...", "→".bright_blue());
    let project = XcodeProject::find(crate::platform::Platform::Ios)?;
    
    println!("  {} Finding available simulator...", "→".bright_blue());
    let (simulator_name, simulator_id) = IOSSimulator::get_available()?;
    println!("  {} Using simulator: {}", "→".bright_blue(), simulator_name);
    
    println!("  {} Building and launching in simulator...", "→".bright_blue());
    
    let destination = format!("platform=iOS Simulator,id={}", simulator_id);
    project.build(&destination)?;
    
    let app_name = &project.scheme;
    
    println!("  {} Launching app in simulator...", "→".bright_blue());
    
    // Find the built app
    let app_path = find_ios_app_bundle(app_name)?;
    
    // Use simulator abstraction
    let sim = IOSSimulator;
    sim.boot(&simulator_id)?;
    sim.open_app()?;
    
    // Give simulator time to boot
    std::thread::sleep(std::time::Duration::from_secs(3));
    
    // Get bundle ID
    let bundle_id = format!("com.example.{}", app_name.replace("-", ""));
    
    // Install and launch with retry logic
    for attempt in 0..3 {
        sim.install_app(&app_path)?;
        sim.open_app()?; // Bring to foreground
        
        match sim.launch_app(&bundle_id) {
            Ok(_) => break,
            Err(e) if attempt < 2 => {
                eprintln!("  ⚠ Launch failed (attempt {}/3): {}", attempt + 1, e);
                eprintln!("  → Cleaning up and retrying...");
                let _ = Command::new("xcrun")
                    .args(&["simctl", "uninstall", "booted", &bundle_id])
                    .status();
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
            Err(e) => return Err(e),
        }
    }
    
    println!();
    println!("{}", "  ✅ App launched in simulator!".green());
    
    Ok(())
}

fn run_android() -> Result<()> {
    use crate::platform::AndroidProject;
    
    println!("  {} Preparing Android emulator...", "→".bright_blue());
    
    let android = AndroidProject::find()?;
    
    // Find emulator command
    let emulator_cmd = android.find_tool("emulator")
        .context("Android SDK emulator not found. Please install Android Studio and ensure the SDK is set up.")?;
    
    // Find adb command
    let adb_cmd = android.find_tool("adb")
        .context("adb not found. Please install Android SDK platform-tools.")?;
    
    // Check if an emulator is already running
    let devices_output = Command::new(&adb_cmd)
        .arg("devices")
        .output()
        .context("Failed to check for running devices")?;
    
    let devices_str = String::from_utf8_lossy(&devices_output.stdout);
    let running_emulator = devices_str.lines()
        .skip(1) // Skip "List of devices attached" header
        .find(|line| line.contains("emulator") && line.contains("device"));
    
    if running_emulator.is_some() {
        println!("  {} Using already-running emulator", "→".bright_blue());
    } else {
        // Check if emulator is available
        let emulator_output = Command::new(&emulator_cmd)
            .arg("-list-avds")
            .output()
            .context("Failed to list Android Virtual Devices")?;
        
        let avds = String::from_utf8_lossy(&emulator_output.stdout);
        let avd_list: Vec<&str> = avds.lines().filter(|l| !l.is_empty()).collect();
        
        if avd_list.is_empty() {
            anyhow::bail!(
                "No Android Virtual Devices (AVDs) found.\n\
                Create one using: Android Studio > Tools > Device Manager > Create Device"
            );
        }
        
        // Use the first available AVD
        let avd_name = avd_list[0];
        println!("  {} Available AVDs: {}", "→".bright_blue(), avd_list.join(", "));
        println!("  {} Starting emulator: {}...", "→".bright_blue(), avd_name.bright_cyan());
        
        // Start emulator in background
        Command::new(&emulator_cmd)
            .arg("-avd")
            .arg(avd_name)
            .arg("-no-snapshot-load")
            .spawn()
            .context("Failed to start emulator")?;
        
        // Wait a bit for emulator to start
        println!("  {} Waiting for emulator to boot...", "→".bright_blue());
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
    
    // Wait for device to be ready
    for i in 1..=30 {
        let status = Command::new(&adb_cmd)
            .args(&["shell", "getprop", "sys.boot_completed"])
            .output();
        
        if let Ok(output) = status {
            if String::from_utf8_lossy(&output.stdout).trim() == "1" {
                break;
            }
        }
        
        if i % 5 == 0 {
            println!("  {} Still waiting... ({}/30s)", "→".bright_blue(), i);
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    
    println!("  {} Building APK with Gradle...", "→".bright_blue());
    
    // Build APK using gradlew
    let android_dir = std::path::Path::new("platforms/android");
    
    let build_result = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "cd {} && export ANDROID_HOME=~/Library/Android/sdk && ./gradlew assembleDebug",
            android_dir.display()
        ))
        .status()
        .context("Failed to run Gradle build")?;
    
    if !build_result.success() {
        anyhow::bail!("Gradle build failed. Check the error messages above.");
    }
    
    println!("  {} Installing APK to emulator...", "→".bright_blue());
    
    // Find the built APK
    let apk_path = android_dir.join("app/build/outputs/apk/debug/app-debug.apk");
    
    if !apk_path.exists() {
        anyhow::bail!("APK not found at: {}", apk_path.display());
    }
    
    // Install APK using adb
    let install_status = Command::new(&adb_cmd)
        .args(&["install", "-r", apk_path.to_str().unwrap()])
        .status()
        .context("Failed to install APK")?;
    
    if !install_status.success() {
        anyhow::bail!("Failed to install APK on emulator");
    }
    
    println!("  {} Launching app...", "→".bright_blue());
    
    // Get package name from build.gradle.kts
    let build_gradle = std::fs::read_to_string("platforms/android/app/build.gradle.kts")
        .context("Failed to read build.gradle.kts")?;
    
    let package_name = build_gradle
        .lines()
        .find(|line| line.contains("applicationId"))
        .and_then(|line| line.split('"').nth(1))
        .context("Could not find applicationId in build.gradle.kts")?;
    
    // Launch the app
    let activity = format!("{}.MainActivity", package_name);
    Command::new(&adb_cmd)
        .args(&[
            "shell",
            "am",
            "start",
            "-n",
            &format!("{}/{}", package_name, activity),
        ])
        .status()
        .context("Failed to launch app")?;
    
    println!();
    println!("{}", "  ✅ App launched on emulator!".green());
    println!();
    
    Ok(())
}

fn run_macos() -> Result<()> {
    use crate::platform::XcodeProject;
    
    println!("  {} Finding Xcode project...", "→".bright_blue());
    let project = XcodeProject::find(crate::platform::Platform::Macos)?;
    
    println!("  {} Building and launching macOS app...", "→".bright_blue());
    project.build("")?; // No destination needed for macOS
    
    println!("  {} Launching app...", "→".bright_blue());
    let home = std::env::var("HOME").unwrap();
    let derived_data = format!("{}/Library/Developer/Xcode/DerivedData", home);
    
    let app_path = std::fs::read_dir(&derived_data)
        .context("Could not read DerivedData directory")?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with(&project.scheme))
        .find_map(|project_dir| {
            let app_path = project_dir
                .path()
                .join("Build/Products/Debug")
                .join(format!("{}.app", &project.scheme));
            if app_path.exists() { Some(app_path) } else { None }
        })
        .context("Could not find built .app bundle")?;
    
    Command::new("open").arg(&app_path).status().context("Failed to open app")?;
    
    println!();
    println!("{}", "  ✅ macOS app launched!".green());
    
    Ok(())
}

fn launch_windows_app() -> Result<()> {
    use std::process::Command;

    let windows_dir = std::env::current_dir()?.join("platforms/windows");

    // Deploy and launch the MSIX package via PowerShell
    // Detects host architecture and deploys only the matching build
    let status = Command::new("powershell")
        .args([
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            r#"
            $ErrorActionPreference = 'Stop'

            # Detect host architecture
            $arch = $env:PROCESSOR_ARCHITECTURE
            $platformDir = switch ($arch) {
                'AMD64' { 'x64' }
                'x86'   { 'x86' }
                'ARM64' { 'ARM64' }
                default { 'x64' }
            }

            Write-Host "Detected architecture: $arch → $platformDir"

            # Look for AppxManifest.xml in the matching platform directory
            $manifest = Get-ChildItem -Path "bin\$platformDir" -Recurse -Filter AppxManifest.xml | Select-Object -First 1
            if (-not $manifest) { 
                throw "AppxManifest.xml not found for $platformDir. Build the project first with: jffi build --platform windows" 
            }

            Write-Host "Found manifest: $($manifest.FullName)"
            Add-AppxPackage -Register $manifest.FullName -ForceApplicationShutdown

            # Get the Application ID from manifest
            $xml = [xml](Get-Content $manifest.FullName)
            $appId = $xml.Package.Applications.Application.Id
            
            # Get PackageFamilyName from the registered package (not from manifest GUID)
            $identity = $xml.Package.Identity.Name
            $package = Get-AppxPackage -Name $identity
            if (-not $package) {
                throw "Package not found after registration. Identity: $identity"
            }
            
            $packageFamilyName = $package.PackageFamilyName
            $aumid = "$packageFamilyName!$appId"

            Write-Host "Package Family Name: $packageFamilyName"
            Write-Host "Launching: $aumid"
            Start-Process "shell:AppsFolder\$aumid"
            "#
        ])
        .current_dir(&windows_dir)
        .status()?;

    if !status.success() {
        anyhow::bail!("Failed to deploy/launch Windows app");
    }

    Ok(())
}

fn run_windows() -> Result<()> {
    
    // Check if this is first build or rebuild
    let bin_dir = std::env::current_dir()?.join("platforms/windows/bin");
    let is_first_build = !bin_dir.exists() || std::fs::read_dir(&bin_dir)?.next().is_none();
    
    if is_first_build {
        // First build: build all architectures and C# project
        println!("  {} Building Windows app (all architectures)...", "→".bright_blue());
        crate::commands::build::build_project(Some("windows".to_string()), false, false, false, false)?;
    } else {
        // Rebuild: only build Rust for active architecture (faster)
        println!("  {} Rebuilding Rust core...", "→".bright_blue());
        crate::commands::dev::rebuild_windows_rust_only()?;
        
        // Copy updated DLL
        let _ = crate::commands::dev::copy_windows_ffi_dll();
    }

    println!();
    println!("{}", "  ✅ Build complete!".green());
    println!("{}", "  🚀 Deploying and launching Windows app...".bright_cyan());
    
    launch_windows_app()?;
    
    println!();
    println!("{}", "  ✅ App launched!".green());
    
    Ok(())
}

fn run_linux() -> Result<()> {
    println!("  {} Checking Linux dependencies...", "→".bright_blue());
    
    // Check for build tools (gcc)
    let needs_setup = !Command::new("gcc").arg("--version").output()?.status.success()
        || !Command::new("python3").arg("--version").output()?.status.success()
        || !Command::new("pkg-config").args(&["--exists", "gtk4"]).output()?.status.success();
    
    if needs_setup {
        println!("  {} Missing dependencies. Running setup script...", "→".bright_blue());
        
        let status = Command::new("bash")
            .arg("platforms/linux/setup.sh")
            .status()
            .context("Failed to run setup script")?;
        
        if !status.success() {
            anyhow::bail!("Dependency installation failed. Run: cd platforms/linux && ./setup.sh");
        }
    }
    
    // Build the Rust FFI library
    println!("  {} Building Rust FFI library...", "→".bright_blue());
    crate::commands::build::build_project(Some("linux".to_string()), false, false, false, false)?;
    
    // Copy the .so file to platforms/linux with the correct name
    let lib_path = std::fs::read_dir("target/debug")
        .context("Failed to read target directory")?
        .filter_map(|e| e.ok())
        .find(|e| {
            let name = e.file_name();
            let name_str = name.to_string_lossy();
            name_str.starts_with("lib") && name_str.ends_with("core.so")
        })
        .map(|e| e.path())
        .context("Could not find core library (lib*core.so). Make sure to build first.")?;
    
    let lib_filename = lib_path.file_name()
        .context("Could not get library filename")?;
    let dest_path = format!("platforms/linux/{}", lib_filename.to_string_lossy());
    
    std::fs::copy(&lib_path, &dest_path)
        .context("Failed to copy library to platforms/linux")?;
    
    // Launch the Python app
    println!("  {} Launching app...", "→".bright_blue());
    
    let status = Command::new("python3")
        .arg("main.py")
        .current_dir("platforms/linux")
        .status()
        .context("Failed to launch app")?;
    
    if !status.success() {
        anyhow::bail!("App failed to run");
    }
    
    println!("{}", "  ✅ App launched!".green());
    Ok(())
}

fn run_web() -> Result<()> {
    // Build the WASM first
    println!("  {} Building Rust FFI library...", "→".bright_blue());
    crate::commands::build::build_project(Some("web".to_string()), false, false, false, false)?;
    
    // Check if npm is installed
    let npm_check = Command::new("npm")
        .arg("--version")
        .output();
    
    if npm_check.is_err() || !npm_check.unwrap().status.success() {
        anyhow::bail!("npm is not installed. Please install Node.js and npm first.");
    }
    
    // Install npm dependencies if needed
    let node_modules = std::path::Path::new("platforms/web/node_modules");
    if !node_modules.exists() {
        println!("  {} Installing npm dependencies...", "→".bright_blue());
        let status = Command::new("npm")
            .arg("install")
            .current_dir("platforms/web")
            .status()
            .context("Failed to install npm dependencies")?;
        
        if !status.success() {
            anyhow::bail!("npm install failed");
        }
    }
    
    // Start Vite dev server
    println!("  {} Starting Vite dev server...", "→".bright_blue());
    println!("  {} Server will open at http://localhost:3000", "→".bright_blue());
    
    let status = Command::new("npm")
        .arg("run")
        .arg("dev")
        .current_dir("platforms/web")
        .status()
        .context("Failed to start Vite dev server")?;
    
    if !status.success() {
        anyhow::bail!("Vite dev server failed");
    }
    
    println!("{}", "  ✅ Web server stopped".green());
    Ok(())
}

