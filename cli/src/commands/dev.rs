use anyhow::Result;
use colored::*;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;

pub fn watch_project(platform: &str) -> Result<()> {
    println!("{}", format!("👀 Watch mode for {}...", platform).bright_green().bold());
    println!("   Watching for changes in core/ and ffi/");
    println!("   Press Ctrl+C to stop");
    println!();
    
    // Initial build and run
    println!("{}", "  → Initial build and launch...".bright_blue());
    initial_build_and_run(platform)?;
    
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
    
    println!("{}", "  ✓ Watching for changes...".green());
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
                        println!("{}", "  🔄 Changes detected, rebuilding...".yellow());
                        
                        match rebuild_and_run(platform) {
                            Ok(_) => {
                                println!();
                                println!("{}", "  ✓ Rebuild complete!".green());
                                println!();
                            }
                            Err(e) => {
                                println!();
                                println!("{}", format!("  ✗ Build failed: {}", e).red());
                                println!("{}", "  → Waiting for fixes...".yellow());
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

fn initial_build_and_run(platform: &str) -> Result<()> {
    // Build and launch the app for the first time
    crate::commands::build::build_platform(platform, false)?;
    
    // Launch the app in the simulator
    println!();
    println!("{}", "  → Launching app in simulator...".bright_blue());
    crate::commands::run::run_platform(platform)?;
    
    println!();
    println!("{}", "  ✓ App running with hot reload enabled".green());
    
    Ok(())
}

fn rebuild_and_run(platform: &str) -> Result<()> {
    // Build the project (this updates the dylib)
    crate::commands::build::build_platform(platform, false)?;
    
    // For iOS with hot reload, the HotReloadManager will automatically
    // detect the dylib change and reload it. No need to restart the app!
    println!("{}", "  → Dylib updated, hot reload will trigger automatically".bright_green());
    
    Ok(())
}
