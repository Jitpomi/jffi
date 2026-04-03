use anyhow::{Context, Result};
use colored::*;
use std::process::Command;

pub fn run_project(platform: &str) -> Result<()> {
    println!("{}", format!("🚀 Running on {}...", platform).bright_green().bold());
    println!();
    
    // Build first
    crate::commands::build::build_project(Some(platform.to_string()), false, false)?;
    
    println!();
    println!("{}", format!("▶️  Launching {}...", platform).bright_cyan().bold());
    
    run_platform(platform)
}

pub fn run_platform(platform: &str) -> Result<()> {
    match platform {
        "ios" => run_ios(),
        "android" => run_android(),
        "macos" | "macos-arm64" | "macos-x64" => run_macos(),
        "windows" | "windows-x64" | "windows-x86" => run_windows(),
        "linux" => run_linux(),
        "web" => run_web(),
        _ => anyhow::bail!("Unknown platform: {}", platform),
    }
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
        .ok()
        .and_then(|entries| {
            entries
                .filter_map(|e| e.ok())
                .find(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .starts_with(app_name)
                })
                .and_then(|project_dir| {
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
        })
        .context("Could not find built .app bundle")?;
    
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

fn run_android() -> Result<()> {
    println!("  {} Opening Android Studio...", "→".bright_blue());
    
    Command::new("open")
        .arg("-a")
        .arg("Android Studio")
        .arg("platforms/android")
        .status()
        .context("Failed to open Android Studio")?;
    
    println!();
    println!("{}", "  ✅ Android Studio opened".green());
    println!("     Press ▶️ to run in emulator");
    
    Ok(())
}

fn run_macos() -> Result<()> {
    println!("  {} Opening Xcode...", "→".bright_blue());
    
    Command::new("open")
        .arg("platforms/macos/*.xcodeproj")
        .status()
        .context("Failed to open Xcode")?;
    
    println!();
    println!("{}", "  ✅ Xcode opened".green());
    println!("     Press ⌘R to run");
    
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
