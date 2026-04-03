use anyhow::{Context, Result};
use colored::*;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::process::Command;
use std::sync::mpsc::channel;
use std::time::Duration;

pub fn watch_project(platform: &str) -> Result<()> {
    println!("{}", format!("👀 Rust watch mode for {}...", platform).bright_green().bold());
    println!();
    println!("   This watches your Rust code (core/ and ffi/) and rebuilds on changes.");
    println!("   For Swift changes, use Xcode directly - it has native hot reload.");
    println!();
    
    // Initial build
    println!("{}", "  → Initial Rust build...".bright_blue());
    crate::commands::build::build_platform(platform, false)?;
    println!("{}", "  ✓ Build complete!".green());
    println!();
    
    // Auto-open Xcode
    if platform == "ios" || platform == "macos" {
        println!("{}", "  → Opening Xcode...".bright_blue());
        open_xcode_project(platform)?;
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
    
    // Watch core and ffi directories
    watcher.watch(Path::new("core/src"), RecursiveMode::Recursive)?;
    watcher.watch(Path::new("ffi/src"), RecursiveMode::Recursive)?;
    
    println!("{}", "  👀 Watching Rust files...".bright_cyan());
    println!();
    
    let mut last_rebuild = std::time::Instant::now();
    let debounce_duration = Duration::from_millis(500);
    
    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(event) => {
                // Check if this is a modify or create event
                if matches!(
                    event.kind,
                    notify::EventKind::Modify(_) | notify::EventKind::Create(_)
                ) {
                    // Debounce: only rebuild if enough time has passed
                    let now = std::time::Instant::now();
                    if now.duration_since(last_rebuild) > debounce_duration {
                        println!();
                        println!("{}", "  🔄 Rust changes detected, rebuilding...".yellow());
                        
                        match crate::commands::build::build_platform(platform, false) {
                            Ok(_) => {
                                println!();
                                println!("{}", "  ✓ Rust rebuild complete! Press Cmd+B in Xcode to use new code.".green());
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

fn open_xcode_project(platform: &str) -> Result<()> {
    let platform_dir = match platform {
        "ios" => "platforms/ios",
        "macos" => "platforms/macos",
        _ => return Ok(()),
    };
    
    // Check if platform directory exists
    if !std::path::Path::new(platform_dir).exists() {
        anyhow::bail!(
            "Platform directory '{}' not found. Make sure you're in the project root directory.",
            platform_dir
        );
    }
    
    // Find the .xcodeproj file
    let xcodeproj = std::fs::read_dir(platform_dir)
        .context(format!("Failed to read directory '{}'", platform_dir))?
        .filter_map(|e| e.ok())
        .find(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s == "xcodeproj")
                .unwrap_or(false)
        })
        .map(|e| e.path())
        .context(format!("Could not find .xcodeproj file in '{}'", platform_dir))?;
    
    // Open in Xcode
    Command::new("open")
        .arg(xcodeproj)
        .status()
        .context("Failed to open Xcode")?;
    
    Ok(())
}
