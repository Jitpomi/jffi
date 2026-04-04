use anyhow::{Context, Result};
use colored::*;
use std::process::Command;

pub fn run_project(platform: &str, device: bool) -> Result<()> {
    let target_desc = if device && platform == "ios" { "iOS Device" } else { platform };
    println!("{}", format!("🚀 Running on {}...", target_desc).bright_green().bold());
    println!();
    
    // Build first
    crate::commands::build::build_project(Some(platform.to_string()), false, false, device)?;
    
    println!();
    println!("{}", format!("▶️  Launching {}...", target_desc).bright_cyan().bold());
    
    run_platform_with_options(platform, device)
}

pub fn run_platform_with_options(platform: &str, device: bool) -> Result<()> {
    match platform {
        "ios" => {
            if device {
                run_ios_device()
            } else {
                run_ios()
            }
        },
        "android" => run_android(),
        "macos" | "macos-arm64" | "macos-x64" => run_macos(),
        "windows" | "windows-x64" | "windows-x86" => run_windows(),
        "linux" => run_linux(),
        "web" => run_web(),
        _ => anyhow::bail!("Unknown platform: {}", platform),
    }
}

fn run_ios_device() -> Result<()> {
    println!("  {} Finding Xcode project...", "→".bright_blue());
    
    // Find the xcodeproj
    let ios_dir = std::path::Path::new("platforms/ios");
    let xcodeproj = std::fs::read_dir(ios_dir)?
        .filter_map(|e| e.ok())
        .find(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s == "xcodeproj")
                .unwrap_or(false)
        })
        .map(|e| e.path())
        .context("Could not find .xcodeproj file")?;
    
    println!("  {} Building and deploying to device...", "→".bright_blue());
    println!();
    println!("{}", "  Note: Make sure your device is connected and trusted.".yellow());
    println!("{}", "  You may need to configure code signing in Xcode first.".yellow());
    println!();
    
    // Build and run on device using xcodebuild
    // This will use the first connected device
    let status = Command::new("xcodebuild")
        .args(&[
            "-project",
            xcodeproj.to_str().unwrap(),
            "-scheme",
            xcodeproj.file_stem().unwrap().to_str().unwrap(),
            "-destination",
            "generic/platform=iOS",
            "build",
        ])
        .status()
        .context("Failed to build with xcodebuild")?;
    
    if !status.success() {
        anyhow::bail!("Build failed. Make sure code signing is configured in Xcode.");
    }
    
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
    println!("  {} Finding Xcode project...", "→".bright_blue());
    
    // Find the xcodeproj
    let ios_dir = std::path::Path::new("platforms/ios");
    let xcodeproj = std::fs::read_dir(ios_dir)?
        .filter_map(|e| e.ok())
        .find(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s == "xcodeproj")
                .unwrap_or(false)
        })
        .map(|e| e.path())
        .context("Could not find .xcodeproj file")?;
    
    println!("  {} Building and launching in simulator...", "→".bright_blue());
    
    // Build and run in simulator using xcodebuild
    let status = Command::new("xcodebuild")
        .args(&[
            "-project",
            xcodeproj.to_str().unwrap(),
            "-scheme",
            xcodeproj.file_stem().unwrap().to_str().unwrap(),
            "-destination",
            "platform=iOS Simulator,name=iPhone 16 Pro",
            "build",
        ])
        .status()
        .context("Failed to build with xcodebuild")?;
    
    if !status.success() {
        anyhow::bail!("Build failed");
    }
    
    println!("  {} Launching app in simulator...", "→".bright_blue());
    
    // Get the app name and find the built .app bundle
    let app_name = xcodeproj.file_stem().unwrap().to_str().unwrap();
    
    // The app bundle is in the DerivedData directory
    // We need to find it by looking for the project-specific DerivedData folder
    let home = std::env::var("HOME").unwrap();
    let derived_data = format!("{}/Library/Developer/Xcode/DerivedData", home);
    
    // Find the app bundle
    let _app_bundle = format!(
        "{}/Build/Products/Debug-iphonesimulator/{}.app",
        derived_data, app_name
    );
    
    // Check if app bundle exists by searching DerivedData
    let app_path = std::fs::read_dir(&derived_data)
        .context("Could not read DerivedData directory")?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with(app_name)
        })
        .find_map(|project_dir| {
            let app_path = project_dir
                .path()
                .join("Build/Products/Debug-iphonesimulator")
                .join(format!("{}.app", app_name));
            if app_path.exists() {
                Some(app_path)
            } else {
                None
            }
        })
        .context("Could not find built .app bundle. Try running 'jffi build --platform ios' first.")?;
    
    // Boot simulator if needed
    println!("  {} Booting simulator...", "→".bright_blue());
    Command::new("xcrun")
        .args(&["simctl", "boot", "iPhone 16 Pro"])
        .output()
        .ok(); // Ignore error if already booted
    
    // Open Simulator app
    Command::new("open")
        .args(&["-a", "Simulator"])
        .status()
        .ok();
    
    // Give simulator time to boot
    std::thread::sleep(std::time::Duration::from_secs(3));
    
    // Install the app
    println!("  {} Installing app...", "→".bright_blue());
    let install_status = Command::new("xcrun")
        .args(&[
            "simctl",
            "install",
            "booted",
            app_path.to_str().unwrap(),
        ])
        .status()
        .context("Failed to install app")?;
    
    if !install_status.success() {
        anyhow::bail!("Failed to install app in simulator");
    }
    
    // Bring Simulator to foreground
    Command::new("open")
        .args(&["-a", "Simulator"])
        .status()
        .ok();
    
    // Get the bundle identifier from Info.plist
    let bundle_id = format!("com.example.{}", app_name.replace("-", ""));
    
    // Launch the app
    println!("  {} Launching app...", "→".bright_blue());
    let launch_status = Command::new("xcrun")
        .args(&["simctl", "launch", "booted", &bundle_id])
        .status()
        .context("Failed to launch app")?;
    
    if !launch_status.success() {
        anyhow::bail!("Failed to launch app in simulator");
    }
    
    println!();
    println!("{}", "  ✅ App launched in simulator!".green());
    
    Ok(())
}

fn find_android_tool(tool_name: &str) -> Option<String> {
    // Check if tool is in PATH first
    if Command::new(tool_name).arg("--version").output().is_ok() {
        return Some(tool_name.to_string());
    }
    
    // Try default Android SDK locations
    let home = std::env::var("HOME").unwrap_or_default();
    let tool_subdir = if tool_name == "emulator" { "emulator" } else { "platform-tools" };
    
    let possible_paths = vec![
        format!("{}/Library/Android/sdk/{}/{}", home, tool_subdir, tool_name),
        format!("{}/Android/Sdk/{}/{}", home, tool_subdir, tool_name),
        format!("{}/.android/sdk/{}/{}", home, tool_subdir, tool_name),
    ];
    
    possible_paths.into_iter()
        .find(|path| std::path::Path::new(path).exists())
}

fn run_android() -> Result<()> {
    println!("  {} Preparing Android emulator...", "→".bright_blue());
    
    // Find emulator command
    let emulator_cmd = find_android_tool("emulator")
        .context("Android SDK emulator not found. Please install Android Studio and ensure the SDK is set up.")?;
    
    // Find adb command
    let adb_cmd = find_android_tool("adb")
        .context("adb not found. Please install Android SDK platform-tools.")?;
    
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
    println!("  {} Finding Xcode project...", "→".bright_blue());
    
    // Find the xcodeproj
    let macos_dir = std::path::Path::new("platforms/macos");
    let xcodeproj = std::fs::read_dir(macos_dir)?
        .filter_map(|e| e.ok())
        .find(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s == "xcodeproj")
                .unwrap_or(false)
        })
        .map(|e| e.path())
        .context("Could not find .xcodeproj file")?;
    
    println!("  {} Building and launching macOS app...", "→".bright_blue());
    
    // Build and run using xcodebuild
    let status = Command::new("xcodebuild")
        .args(&[
            "-project",
            xcodeproj.to_str().unwrap(),
            "-scheme",
            xcodeproj.file_stem().unwrap().to_str().unwrap(),
            "build",
        ])
        .status()
        .context("Failed to build with xcodebuild")?;
    
    if !status.success() {
        anyhow::bail!("Build failed");
    }
    
    println!("  {} Launching app...", "→".bright_blue());
    
    // Get the app name
    let app_name = xcodeproj.file_stem().unwrap().to_str().unwrap();
    
    // Find the built app in DerivedData
    let home = std::env::var("HOME").unwrap();
    let derived_data = format!("{}/Library/Developer/Xcode/DerivedData", home);
    
    let app_path = std::fs::read_dir(&derived_data)
        .context("Could not read DerivedData directory")?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with(app_name)
        })
        .find_map(|project_dir| {
            let app_path = project_dir
                .path()
                .join("Build/Products/Debug")
                .join(format!("{}.app", app_name));
            if app_path.exists() {
                Some(app_path)
            } else {
                None
            }
        })
        .context("Could not find built .app bundle. Try running 'jffi build --platform macos' first.")?;
    
    // Launch the app
    Command::new("open")
        .arg(app_path)
        .status()
        .context("Failed to launch app")?;
    
    println!();
    println!("{}", "  ✅ macOS app launched!".green());
    
    Ok(())
}

fn run_windows() -> Result<()> {
    println!("  {} Opening Visual Studio...", "→".bright_blue());
    
    println!();
    println!("{}", "  ℹ️  Open platforms/windows/*.sln in Visual Studio".bright_cyan());
    println!("     Press F5 to run");
    
    Ok(())
}

fn run_linux() -> Result<()> {
    println!("  {} Building and running GTK app...", "→".bright_blue());
    
    let status = Command::new("cargo")
        .args(&["run", "--manifest-path", "platforms/linux/Cargo.toml"])
        .status()
        .context("Failed to run Linux app")?;
    
    if !status.success() {
        anyhow::bail!("Failed to run app");
    }
    
    Ok(())
}

fn run_web() -> Result<()> {
    println!("  {} Starting web server...", "→".bright_blue());
    
    // Check if http-server is installed
    if Command::new("which").arg("http-server").output()?.status.success() {
        println!("  {} Serving on http://localhost:8080", "→".bright_blue());
        
        Command::new("http-server")
            .arg("platforms/web")
            .arg("-p")
            .arg("8080")
            .status()
            .context("Failed to start web server")?;
    } else {
        println!();
        println!("{}", "  ℹ️  Install http-server: npm install -g http-server".bright_cyan());
        println!("     Then run: cd platforms/web && http-server");
    }
    
    Ok(())
}
