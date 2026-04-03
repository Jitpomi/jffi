use anyhow::Result;
use colored::*;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;

pub fn watch_project(platform: &str) -> Result<()> {
    println!("{}", format!("👀 Rust watch mode for {}...", platform).bright_green().bold());
    println!();
    println!("   This watches your Rust code (core/ and ffi/) and rebuilds on changes.");
    println!("   For Swift changes, use Xcode directly - it has native hot reload.");
    println!();
    println!("   To use:");
    println!("   1. Open platforms/ios/*.xcodeproj in Xcode");
    println!("   2. Run the app from Xcode (Cmd+R)");
    println!("   3. Edit Swift files - Xcode hot reloads automatically");
    println!("   4. Edit Rust files - this watcher rebuilds the dylib");
    println!("   5. In Xcode, press Cmd+B to rebuild with new Rust code");
    println!();
    println!("   Press Ctrl+C to stop watching");
    println!();
    
    // Initial build
    println!("{}", "  → Initial Rust build...".bright_blue());
    crate::commands::build::build_platform(platform, false)?;
    println!("{}", "  ✓ Ready! Open Xcode and run your app.".green());
    println!();
    
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
