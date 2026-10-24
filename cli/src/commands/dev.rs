use anyhow::{Context, Result};
use colored::*;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::channel;
use std::time::Duration;

use crate::platform::{AndroidProject, Platform, XcodeProject};

pub fn watch_project(platform: &str) -> Result<()> {
    println!(
        "{}",
        format!("👀 Rust watch mode for {}...", platform)
            .bright_green()
            .bold()
    );
    println!();
    println!("   This watches your Rust code (core/) and rebuilds on changes.");
    println!();

    // Initial build (full build to ensure Xcode compatibility)
    println!("{}", "  → Initial Rust build...".bright_blue());
    crate::commands::build::build_project(Some(platform.to_string()), false, false, false, false)?;
    println!("{}", "  ✓ Build complete!".green());
    println!();

    // Parse platform
    let platform_enum = Platform::from_str(platform)
        .ok_or_else(|| anyhow::anyhow!("Unknown platform: {}", platform))?;

    let mut linux_process = None;
    // Auto-open IDE or launch app
    if platform == "ios" {
        println!("{}", "  → Opening Xcode...".bright_blue());
        let project = XcodeProject::find(platform_enum)?;
        project.open()?;
        println!();
        println!("{}", "  ✓ Xcode opened!".green());
        println!();
        println!(
            "{}",
            "   🚀 IMPORTANT: Press Cmd+R in Xcode to RUN the app!"
                .bright_yellow()
                .bold()
        );
        println!("   (Select your simulator/device and run)");
        println!();
        println!("   Development workflow:");
        println!("   • Edit Swift files → Use Xcode previews or rebuild the app");
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
        println!(
            "{}",
            "   🚀 IMPORTANT: Press Cmd+R in Xcode to RUN the app!"
                .bright_yellow()
                .bold()
        );
        println!("   (Don't just look at the preview - actually run it)");
        println!();
        println!("   Development workflow:");
        println!("   • Edit Swift files → Use Xcode previews or rebuild the app");
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
        println!(
            "{}",
            "   🚀 IMPORTANT: Press ▶️ in Android Studio to RUN the app!"
                .bright_yellow()
                .bold()
        );
        println!();
        println!("   Development workflow:");
        println!("   • Edit Kotlin files → Changes appear on rebuild (Compose hot reload)");
        println!("   • Edit Rust files → This watcher rebuilds → Rebuild in Android Studio");
        println!();
        println!("   Press Ctrl+C to stop watching");
        println!();
    } else if platform == "linux" {
        println!("{}", "  → Launching Linux app...".bright_blue());
        linux_process = Some(launch_linux_app_background()?);
        println!();
        println!("{}", "  ✓ App launched!".green());
        println!();
        println!(
            "{}",
            "   🚀 Development mode active!".bright_yellow().bold()
        );
        println!();
        println!("   Development workflow:");
        println!("   • Edit Python files → App auto-restarts");
        println!("   • Edit Rust files → This watcher rebuilds → App auto-restarts");
        println!();
        println!("   Press Ctrl+C to stop watching");
        println!();
    } else if platform == "web" {
        println!("{}", "  → Starting web dev server...".bright_blue());
        start_web_dev_server()?;
        println!(
            "{}",
            "   🚀 Development mode active!".bright_yellow().bold()
        );
        println!();
        println!("   Development workflow:");
        println!("   • Edit JS/HTML/CSS files → Hot reload via Vite");
        println!("   • Edit Rust files → This watcher rebuilds WASM → Refresh browser");
        println!();
        println!("   Press Ctrl+C to stop watching");
        println!();
    } else if platform == "windows" {
        println!("{}", "  → Opening Visual Studio...".bright_blue());
        open_visual_studio()?;
        println!();
        println!("{}", "  ✓ Visual Studio opened!".green());
        println!();
        println!(
            "{}",
            "   🚀 IMPORTANT: Enable Deploy in VS before running!"
                .bright_yellow()
                .bold()
        );
        println!(
            "{}",
            "      Build → Configuration Manager → ☑ Deploy".bright_yellow()
        );
        println!(
            "{}",
            "      Then press ▶️ (F5) to RUN the app!"
                .bright_yellow()
                .bold()
        );
        println!();
        println!("   Development workflow:");
        println!("   • Edit C# files → Use Visual Studio's hot reload (Edit & Continue)");
        println!("   • Edit Rust files → This watcher rebuilds → Press F5 in Visual Studio");
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
    if platform == "linux" {
        watcher.watch(Path::new("platforms/linux"), RecursiveMode::Recursive)?;
    }

    println!("{}", "  👀 Watching Rust project (core/)...".bright_cyan());
    println!();

    let linux_ui_root = std::env::current_dir()?.join("platforms/linux");
    let mut pending_linux_ui_restart: Option<std::time::Instant> = None;
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
                            && !path_str.contains("core/src/android.rs")
                    });

                    if platform == "linux"
                        && event.paths.iter().all(|path| {
                            path.starts_with(&linux_ui_root) || path.starts_with("platforms/linux")
                        })
                    {
                        let ui_changed = event.paths.iter().any(|path| {
                            path.extension().is_some_and(|ext| ext == "py")
                                && path.file_name().is_some_and(|name| {
                                    !name.to_string_lossy().ends_with("_core.py")
                                })
                                && !path.to_string_lossy().contains("__pycache__")
                        });
                        if ui_changed {
                            pending_linux_ui_restart = Some(std::time::Instant::now());
                        }
                        continue;
                    }
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

                        // For Windows, only build the active platform architecture
                        let build_result = if platform == "windows" {
                            rebuild_windows_rust_only()
                        } else {
                            crate::commands::build::build_platform(platform, false)
                        };

                        match build_result {
                            Ok(_) => {
                                println!();
                                if platform == "linux" {
                                    println!(
                                        "{}",
                                        "  ✓ Rust rebuild complete! Restarting app...".green()
                                    );
                                    match restart_linux_app(&mut linux_process) {
                                        Ok(_) => {
                                            println!("{}", "  ✓ App restarted!".green());
                                        }
                                        Err(e) => {
                                            println!(
                                                "{}",
                                                format!("  ✗ Failed to restart app: {}", e).red()
                                            );
                                        }
                                    }
                                } else if platform == "ios" {
                                    // Touch XCFramework to update timestamp
                                    let _ = touch_xcframework(platform);

                                    println!("{}", "  ✓ Rust rebuild complete! Press Cmd+B in Xcode to use new code.".green());
                                } else if platform == "macos" {
                                    // Touch XCFramework to update timestamp and notify Xcode
                                    let _ = touch_xcframework(platform);
                                    println!("{}", "  ✓ Rust rebuild complete! Press Cmd+B in Xcode to use new code.".green());
                                } else if platform == "web" {
                                    println!("{}", "  ✓ Rust rebuild complete! Refresh your browser to use new code.".green());
                                } else if platform == "windows" {
                                    // Copy updated FFI DLL to output directory for Visual Studio to pick up
                                    if let Err(error) = copy_windows_ffi_dll() {
                                        eprintln!("Native DLL reload failed: {}", error);
                                    }
                                    println!("{}", "  ✓ Rust rebuild complete! Press F5 in Visual Studio to use new code.".green());
                                } else {
                                    println!("{}", "  ✓ Rust rebuild complete!".green());
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
                if pending_linux_ui_restart
                    .is_some_and(|changed| changed.elapsed() >= debounce_duration)
                {
                    pending_linux_ui_restart = None;
                    if let Err(error) = restart_linux_app(&mut linux_process) {
                        eprintln!(
                            "Python UI restart failed: {}. Fix the source and save again.",
                            error
                        );
                    }
                }
                continue;
            }
            Err(e) => {
                anyhow::bail!("Watch error: {}", e);
            }
        }
    }
}

fn find_windows_solution() -> Result<PathBuf> {
    let windows_dir = std::env::current_dir()?.join("platforms").join("windows");

    find_file_with_extension(&windows_dir, "slnx")
        .or_else(|| find_file_with_extension(&windows_dir, "csproj"))
        .with_context(|| {
            format!(
                "Could not find .slnx or .csproj file in {}",
                windows_dir.display()
            )
        })
}

fn find_file_with_extension(dir: &Path, extension: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|ext| ext == extension))
}

fn open_visual_studio() -> Result<()> {
    let solution_path = find_windows_solution()?;
    let solution_path_str = solution_path.to_str().context("Invalid path")?;

    // Try to find devenv.exe in known VS 2022 install locations
    let devenv_paths = [
        r"C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\IDE\devenv.exe",
        r"C:\Program Files\Microsoft Visual Studio\2022\Professional\Common7\IDE\devenv.exe",
        r"C:\Program Files\Microsoft Visual Studio\2022\Enterprise\Common7\IDE\devenv.exe",
        r"C:\Program Files (x86)\Microsoft Visual Studio\2022\Community\Common7\IDE\devenv.exe",
        r"C:\Program Files (x86)\Microsoft Visual Studio\2022\Professional\Common7\IDE\devenv.exe",
        r"C:\Program Files (x86)\Microsoft Visual Studio\2022\Enterprise\Common7\IDE\devenv.exe",
    ];

    for path in &devenv_paths {
        if Path::new(path).exists() {
            Command::new(path)
                .arg(solution_path_str)
                .spawn()
                .context("Failed to launch devenv.exe")?;
            return Ok(());
        }
    }

    // Fallback: try devenv.exe on PATH (e.g. via Developer Command Prompt)
    if Command::new("devenv.exe")
        .arg(solution_path_str)
        .spawn()
        .is_ok()
    {
        return Ok(());
    }

    // Last resort: use explorer to open via file association
    Command::new("explorer")
        .arg(solution_path_str)
        .spawn()
        .with_context(|| {
            format!(
                "Failed to open Visual Studio. Make sure Visual Studio 2022 is installed. \
                 You can also open the project manually at: {}",
                solution_path_str,
            )
        })?;

    Ok(())
}

pub fn rebuild_windows_rust_only() -> Result<()> {
    use std::process::Command;

    // Detect which platform is active (or use host architecture as default)
    let bin_dir = std::env::current_dir()?.join("platforms/windows/bin");
    let is_first_build = !bin_dir.exists();

    let target = detect_host_windows_architecture();

    let platform_name = match target {
        "x86_64-pc-windows-msvc" => "x64",
        "i686-pc-windows-msvc" => "x86",
        "aarch64-pc-windows-msvc" => "ARM64",
        _ => "x64",
    };

    if is_first_build {
        println!(
            "  {} Detected host architecture: {}",
            "→".bright_blue(),
            platform_name
        );
    }
    println!(
        "  {} Building Rust for {} only...",
        "→".bright_blue(),
        platform_name
    );

    // Build only the detected target
    let status = Command::new("cargo")
        .args([
            "build",
            "--target",
            target,
            "--manifest-path",
            "core/Cargo.toml",
        ])
        .status()
        .context("Failed to run cargo build")?;

    if !status.success() {
        anyhow::bail!("Rust build failed");
    }

    Ok(())
}

pub fn detect_host_windows_architecture() -> &'static str {
    // Detect the host CPU architecture using environment variable
    if let Ok(arch) = std::env::var("PROCESSOR_ARCHITECTURE") {
        match arch.as_str() {
            "AMD64" => "x86_64-pc-windows-msvc",
            "x86" => "i686-pc-windows-msvc",
            "ARM64" => "aarch64-pc-windows-msvc",
            _ => "x86_64-pc-windows-msvc", // Default to x64
        }
    } else {
        "x86_64-pc-windows-msvc" // Default to x64 if env var not found
    }
}

pub fn copy_windows_ffi_dll() -> Result<()> {
    let name = crate::commands::build::core_library_name(Path::new("core/Cargo.toml"))?;
    let target = detect_host_windows_architecture();
    let platform = match target {
        "i686-pc-windows-msvc" => "x86",
        "aarch64-pc-windows-msvc" => "ARM64",
        _ => "x64",
    };
    let source = Path::new("target")
        .join(target)
        .join("debug")
        .join(format!("{}.dll", name));
    let destination = crate::platform::windows_output_dir(platform, false);
    if !source.is_file() {
        anyhow::bail!("Fresh Windows debug library missing: {}", source.display());
    }
    fs_copy_windows_library(&source, &destination.join(format!("{}.dll", name)))?;
    fs_copy_windows_library(
        &source,
        &Path::new("platforms/windows").join(format!("{}.dll", name)),
    )?;
    Ok(())
}

fn fs_copy_windows_library(source: &Path, destination: &Path) -> Result<()> {
    std::fs::copy(source, destination).with_context(|| format!("Could not update {}. Stop the app/debugger in Visual Studio before reloading native code.", destination.display()))?;
    Ok(())
}

/// Own the process so watcher errors and normal exits cannot abandon the app.
struct LinuxApp {
    child: std::process::Child,
}

impl LinuxApp {
    fn stop(&mut self) -> Result<()> {
        if self.child.try_wait()?.is_some() {
            return Ok(());
        }
        // xvfb-run owns the Python and Xvfb processes directly. Stop only children
        // of the process we launched; never match arbitrary Python command lines.
        if cfg!(target_os = "linux") {
            let _ = Command::new("pkill")
                .args(["-TERM", "-P", &self.child.id().to_string()])
                .status();
        }
        if let Err(error) = self.child.kill() {
            // The xvfb wrapper may have exited after its child received TERM.
            if self.child.try_wait()?.is_none() {
                return Err(error.into());
            }
        }
        self.child.wait()?;
        Ok(())
    }
}

impl Drop for LinuxApp {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn launch_linux_app_background() -> Result<LinuxApp> {
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

    let lib_filename = lib_path
        .file_name()
        .context("Could not get library filename")?;
    let dest_path = format!("platforms/linux/{}", lib_filename.to_string_lossy());

    std::fs::copy(&lib_path, &dest_path).context("Failed to copy library to platforms/linux")?;

    // Check display environment for headless fallback
    let display_var = std::env::var("DISPLAY").unwrap_or_default();
    let wayland_var = std::env::var("WAYLAND_DISPLAY").unwrap_or_default();
    let display_set = !display_var.trim().is_empty() || !wayland_var.trim().is_empty();

    let has_xvfb = Command::new("which")
        .arg("xvfb-run")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    // Allow user to disable headless mode via env var
    let force_no_headless = std::env::var("JFFI_HEADLESS")
        .map(|v| v == "0")
        .unwrap_or(false);

    if !display_set && !has_xvfb {
        anyhow::bail!(
            "No display server found. Either:\n\
             • SSH with X11 forwarding: ssh -X <host>\n\
             • Install xvfb: sudo apt install xvfb"
        );
    }

    // Launch Python app in background
    let mut cmd = if !display_set && !force_no_headless {
        println!(
            "  {} No display detected, using xvfb-run for headless mode...",
            "→".bright_blue()
        );
        println!("     (Set JFFI_HEADLESS=0 to disable this behavior)");
        let mut c = Command::new("xvfb-run");
        c.args([
            "--auto-servernum",
            "--server-args=-screen 0 1024x768x24",
            "python3",
            "main.py",
        ]);
        c
    } else {
        if !display_set && force_no_headless {
            println!(
                "  {} Headless mode disabled, attempting regular launch...",
                "→".bright_blue()
            );
        }
        let mut c = Command::new("python3");
        c.arg("main.py");
        c
    };

    let verbose = std::env::var("JFFI_VERBOSE").is_ok();

    cmd.current_dir("platforms/linux")
        .env("GSK_RENDERER", "cairo")
        .env("GDK_DEBUG", "gl-disable")
        .env("NO_AT_BRIDGE", "1")
        .env("GTK_A11Y", "none")
        .env("LC_ALL", "C.UTF-8");

    if !verbose {
        cmd.stdout(Stdio::null());
    }

    let mut app = LinuxApp {
        child: cmd.spawn().context("Failed to launch app")?,
    };
    std::thread::sleep(Duration::from_millis(500));
    if let Some(status) = app.child.try_wait()? {
        anyhow::bail!("Linux app exited during startup: {}", status);
    }
    Ok(app)
}

fn restart_linux_app(app: &mut Option<LinuxApp>) -> Result<()> {
    if let Some(mut previous) = app.take() {
        previous.stop()?;
    }
    *app = Some(launch_linux_app_background()?);
    Ok(())
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
                let _ = Command::new("touch").arg(&path).status();

                let info_plist = path.join("Info.plist");
                if info_plist.exists() {
                    let _ = Command::new("touch").arg(&info_plist).status();
                }
            }
        }
    }

    Ok(())
}

fn start_web_dev_server() -> Result<()> {
    use std::process::Stdio;

    let config = crate::config::load_config()?;
    let web_config = config.platforms.web;
    let port = web_config.port;

    let verbose = std::env::var("JFFI_VERBOSE").is_ok();

    // Install npm dependencies if needed
    let node_modules = std::path::Path::new("platforms/web/node_modules");
    if !node_modules.exists() {
        println!("  {} Installing npm dependencies...", "→".bright_blue());
        let mut npm_install = Command::new("npm");
        npm_install.arg("install").current_dir("platforms/web");
        if !verbose {
            npm_install.stdout(Stdio::null()).stderr(Stdio::null());
        }
        let status = npm_install
            .status()
            .context("Failed to install npm dependencies")?;

        if !status.success() {
            anyhow::bail!("npm install failed");
        }
    }

    // Start Vite dev server in background
    let mut vite_cmd = Command::new("npm");
    let mut args = vec![
        "run".to_string(),
        "dev".to_string(),
        "--".to_string(),
        "--port".to_string(),
        port.to_string(),
    ];

    if web_config.host {
        args.push("--host".to_string());
    }
    if web_config.https {
        args.push("--https".to_string());
    }
    if web_config.open {
        args.push("--open".to_string());
    }
    if web_config.cors {
        args.push("--cors".to_string());
    }

    vite_cmd.args(&args).current_dir("platforms/web");
    if !verbose {
        vite_cmd.stdout(Stdio::null()).stderr(Stdio::null());
    }
    vite_cmd
        .spawn()
        .context("Failed to start Vite dev server")?;

    // Give it a moment to start
    std::thread::sleep(Duration::from_millis(1500));

    let protocol = if web_config.https { "https" } else { "http" };
    let host_str = if web_config.host {
        "0.0.0.0"
    } else {
        "localhost"
    };

    println!();
    println!(
        "  {} Vite dev server running at {}",
        "✓".green(),
        format!("{}://{}:{}", protocol, host_str, port)
            .bright_cyan()
            .underline()
    );
    println!();

    Ok(())
}

#[cfg(all(test, unix))]
mod lifecycle_review_tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn stopping_owned_app_is_idempotent_and_leaves_other_processes_alive() {
        let mut unrelated = LinuxApp {
            child: Command::new("sleep").arg("30").spawn().unwrap(),
        };
        let mut owned = LinuxApp {
            child: Command::new("sleep").arg("30").spawn().unwrap(),
        };
        owned.stop().unwrap();
        owned.stop().unwrap();
        assert!(owned.child.try_wait().unwrap().is_some());
        assert!(unrelated.child.try_wait().unwrap().is_none());
        unrelated.stop().unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn dropping_owned_app_reaps_it_after_a_watcher_error() {
        let child = Command::new("sleep").arg("30").spawn().unwrap();
        let id = child.id();
        {
            let _app = LinuxApp { child };
        }
        let status = Command::new("kill")
            .args(["-0", &id.to_string()])
            .output()
            .unwrap();
        assert!(
            !status.status.success(),
            "the app should be stopped and reaped"
        );
    }
}
