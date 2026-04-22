use anyhow::{Context, Result};
use colored::*;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::process::Command;
use std::sync::mpsc::channel;
use std::time::Duration;

use crate::platform::{AndroidProject, Platform, XcodeProject};

pub fn watch_project(platform: &str) -> Result<()> {
    println!("{}", format!("👀 Rust watch mode for {}...", platform).bright_green().bold());
    println!();
    println!("   This watches your Rust code (core/) and rebuilds on changes.");
    println!("   For Swift changes, use Xcode directly - it has native hot reload.");
    println!();
    
    // Initial build (full build to ensure Xcode compatibility)
    println!("{}", "  → Initial Rust build...".bright_blue());
    crate::commands::build::build_platform(platform, false)?;
    println!("{}", "  ✓ Build complete!".green());
    println!();
    
    // Parse platform
    let platform_enum = Platform::from_str(platform)
        .ok_or_else(|| anyhow::anyhow!("Unknown platform: {}", platform))?;
    
    // Auto-open IDE or launch app
    if platform == "ios" {
        println!("{}", "  → Opening Xcode...".bright_blue());
        let project = XcodeProject::find(platform_enum)?;
        project.open()?;
        println!();
        println!("{}", "  ✓ Xcode opened!".green());
        println!();
        println!("{}", "   🚀 IMPORTANT: Press Cmd+R in Xcode to RUN the app!".bright_yellow().bold());
        println!("   (Select your simulator/device and run)");
        println!();
        println!("   Development workflow:");
        println!("   • Edit Swift files → Changes appear instantly (native hot reload)");
        println!("   • Edit Rust files → This watcher rebuilds → Press Cmd+B in Xcode");
        println!();
        println!("   Press Ctrl+C to stop watching");
        println!();
    } else if platform == "macos" {
        println!("{}", "  → Opening Xcode...".bright_blue());
        let project = XcodeProject::find(platform_enum)?;
        project.open()?;
        println!();
        println!("{}", "  ✓ Xcode opened!".green());
        println!();
        println!("{}", "   🚀 IMPORTANT: Press Cmd+R in Xcode to RUN the app!".bright_yellow().bold());
        println!("   (Don't just look at the preview - actually run it)");
        println!();
        println!("   Development workflow:");
        println!("   • Edit Swift files → Changes appear instantly (native hot reload)");
        println!("   • Edit Rust files → This watcher rebuilds → Press Cmd+B in Xcode");
        println!();
        println!("   Press Ctrl+C to stop watching");
        println!();
    } else if platform == "android" {
        println!("{}", "  → Opening Android Studio...".bright_blue());
        let android = AndroidProject::find()?;
        android.open()?;
        println!();
        println!("{}", "  ✓ Android Studio opened!".green());
        println!();
        println!("{}", "   🚀 IMPORTANT: Press ▶️ in Android Studio to RUN the app!".bright_yellow().bold());
        println!();
        println!("   Development workflow:");
        println!("   • Edit Kotlin files → Changes appear on rebuild (Compose hot reload)");
        println!("   • Edit Rust files → This watcher rebuilds → Rebuild in Android Studio");
        println!();
        println!("   Press Ctrl+C to stop watching");
        println!();
    } else if platform == "linux" {
        println!("{}", "  → Launching Linux app...".bright_blue());
        launch_linux_app_background()?;
        println!();
        println!("{}", "  ✓ App launched!".green());
        println!();
        println!("{}", "   🚀 Development mode active!".bright_yellow().bold());
        println!();
        println!("   Development workflow:");
        println!("   • Edit Python files → App auto-restarts");
        println!("   • Edit Rust files → This watcher rebuilds → App auto-restarts");
        println!();
        println!("   Press Ctrl+C to stop watching");
        println!();
    }
    
    // Set up file watcher
    let (tx, rx) = channel();
    
    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        },
        Config::default(),
    )?;
    
    // Watch entire core directory (catches src/, Cargo.toml, build.rs, .udl files, etc.)
    watcher.watch(Path::new("core"), RecursiveMode::Recursive)?;
    
    println!("{}", "  👀 Watching Rust project (core/)...".bright_cyan());
    println!();
    
    let mut last_rebuild = std::time::Instant::now();
    let debounce_duration = Duration::from_millis(500);
    
    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(event) => {
                // Check if this is any relevant file system event
                // Include: Modify, Create, Remove, and other events
                // Note: We process all events and filter by file type instead
                let should_process = !matches!(
                    event.kind,
                    notify::EventKind::Access(_) // Ignore file access events (too noisy)
                );
                
                if should_process {
                    // Filter out irrelevant files (target/, .git/, temp files, etc.)
                    let is_relevant_change = event.paths.iter().any(|p| {
                        let path_str = p.to_string_lossy();
                        // Ignore target directory, hidden files, and temp files
                        !path_str.contains("/target/") 
                        && !path_str.contains("/.") 
                        && !path_str.ends_with('~')
                        && !path_str.contains(".swp")
                        && !path_str.contains(".tmp")
                    });
                    
                    if !is_relevant_change {
                        continue;
                    }
                    
                    // Debug: show which files changed
                    for path in &event.paths {
                        let ext = path.extension().and_then(|s| s.to_str());
                        let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                        
                        // Show changes to Rust files, Cargo.toml, UDL files, build scripts
                        if ext == Some("rs") 
                            || ext == Some("toml") 
                            || ext == Some("udl")
                            || filename == "build.rs"
                        {
                            println!("  📝 Detected change: {}", path.display());
                        }
                    }
                    
                    // Debounce: only rebuild if enough time has passed
                    let now = std::time::Instant::now();
                    if now.duration_since(last_rebuild) > debounce_duration {
                        println!();
                        
                        println!("{}", "  🔄 Rust changes detected, rebuilding...".yellow());
                        
                        match crate::commands::build::build_platform(platform, false) {
                            Ok(_) => {
                                println!();
                                if platform == "linux" {
                                    println!("{}", "  ✓ Rust rebuild complete! Restarting app...".green());
                                    match restart_linux_app() {
                                        Ok(_) => {
                                            println!("{}", "  ✓ App restarted!".green());
                                        }
                                        Err(e) => {
                                            println!("{}", format!("  ✗ Failed to restart app: {}", e).red());
                                        }
                                    }
                                } else if platform == "ios" || platform == "macos" {
                                    // Touch XCFramework to update timestamp and notify Xcode
                                    let _ = touch_xcframework(platform);
                                    println!("{}", "  ✓ Rust rebuild complete! Press Cmd+B in Xcode to use new code.".green());
                                } else {
                                    println!("{}", "  ✓ Rust rebuild complete! Press Cmd+B in Xcode to use new code.".green());
                                }
                                println!();
                            }
                            Err(e) => {
                                println!();
                                println!("{}", format!("  ✗ Build failed: {}", e).red());
                                println!("{}", "  → Fix the error and save again...".yellow());
                                println!();
                            }
                        }
                        
                        last_rebuild = now;
                    }
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Normal timeout, continue watching
                continue;
            }
            Err(e) => {
                anyhow::bail!("Watch error: {}", e);
            }
        }
    }
}


fn launch_linux_app_background() -> Result<()> {
    use std::process::Stdio;
    
    // Copy the .so file to platforms/linux
    let lib_path = std::fs::read_dir("target/debug")
        .context("Failed to read target directory")?
        .filter_map(|e| e.ok())
        .find(|e| {
            let name = e.file_name();
            let name_str = name.to_string_lossy();
            name_str.starts_with("lib") && name_str.ends_with("core.so")
        })
        .map(|e| e.path())
        .context("Could not find FFI library")?;
    
    std::fs::copy(&lib_path, "platforms/linux/libcore.so")
        .context("Failed to copy library to platforms/linux")?;
    
    // Launch Python app in background
    Command::new("python3")
        .arg("main.py")
        .current_dir("platforms/linux")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Failed to launch app")?;
    
    // Give it a moment to start
    std::thread::sleep(Duration::from_millis(500));
    
    Ok(())
}

fn restart_linux_app() -> Result<()> {
    // Kill existing Python processes running main.py
    let _ = Command::new("pkill")
        .args(&["-f", "python3.*main.py"])
        .status();
    
    // Wait a moment for cleanup
    std::thread::sleep(Duration::from_millis(200));
    
    // Relaunch
    launch_linux_app_background()
}

fn touch_xcframework(platform: &str) -> Result<()> {
    // Touch the XCFramework to update its timestamp, signaling to Xcode it changed
    let platform_dir = if platform == "ios" {
        "platforms/ios"
    } else {
        "platforms/macos"
    };
    
    if let Ok(entries) = std::fs::read_dir(platform_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("xcframework") {
                // Touch the XCFramework directory and its Info.plist
                let _ = Command::new("touch")
                    .arg(&path)
                    .status();
                let info_plist = path.join("Info.plist");
                if info_plist.exists() {
                    let _ = Command::new("touch")
                        .arg(&info_plist)
                        .status();
                }
                return Ok(());
            }
        }
    }
    
    Ok(())
}
