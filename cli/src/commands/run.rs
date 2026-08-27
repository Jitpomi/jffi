use anyhow::{Context, Result};
use colored::*;
use std::process::Command;

use crate::platform::Platform;

pub fn run_project(platform_str: &str, device: bool, release: bool) -> Result<()> {
    let platform = Platform::from_str(platform_str)
        .ok_or_else(|| anyhow::anyhow!("Unknown platform: {}", platform_str))?;

    let target_desc = if device && platform == Platform::Ios {
        "iOS Device"
    } else {
        platform.as_str()
    };
    println!(
        "{}",
        format!("🚀 Running on {}...", target_desc)
            .bright_green()
            .bold()
    );
    println!();

    // Sync configurations to native projects before run
    let config = crate::config::load_config()?;
    crate::commands::build::sync_configs_to_platforms(&config)?;

    // Build first
    crate::commands::build::build_project(
        Some(platform_str.to_string()),
        false,
        release,
        device,
        false,
    )?;

    println!();
    println!(
        "{}",
        format!("▶️  Launching {}...", target_desc)
            .bright_cyan()
            .bold()
    );

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

    println!(
        "  {} Building and deploying to device...",
        "→".bright_blue()
    );
    println!();
    println!(
        "{}",
        "  Note: Make sure your device is connected and trusted.".yellow()
    );
    println!(
        "{}",
        "  You may need to configure code signing in Xcode first.".yellow()
    );
    println!();

    project
        .build("generic/platform=iOS", &[])
        .context("Build failed. Make sure code signing is configured in Xcode.")?;

    println!();
    println!("{}", "  ✅ Build complete!".green());
    println!();
    println!("{}", "  To deploy to your device:".bright_cyan());
    println!("  1. Open Xcode");
    println!("  2. Select your connected device");
    println!("  3. Press Cmd+R to run");
    println!();
    println!(
        "{}",
        "  Or use Xcode directly for automatic deployment.".bright_cyan()
    );

    Ok(())
}

fn run_ios() -> Result<()> {
    use crate::platform::{find_ios_app_bundle, IOSSimulator, XcodeProject};

    println!("  {} Finding Xcode project...", "→".bright_blue());
    let project = XcodeProject::find(crate::platform::Platform::Ios)?;

    println!("  {} Finding available simulator...", "→".bright_blue());
    let (simulator_name, simulator_id) = IOSSimulator::get_available()?;
    println!(
        "  {} Using simulator: {}",
        "→".bright_blue(),
        simulator_name
    );

    println!(
        "  {} Building and launching in simulator...",
        "→".bright_blue()
    );

    let destination = format!("platform=iOS Simulator,id={}", simulator_id);
    project.build(&destination, &["CODE_SIGN_IDENTITY=-"])?;

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

    // Get bundle ID from config
    let config = crate::config::load_config()?;
    let bundle_id = config
        .bundle
        .as_ref()
        .and_then(|b| b.identifier.clone())
        .unwrap_or_else(|| config.platforms.ios.bundle_id.clone());

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
                    .args(["simctl", "uninstall", "booted", &bundle_id])
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
    let adb_cmd = android
        .find_tool("adb")
        .context("adb not found. Please install Android SDK platform-tools.")?;

    let get_emulator_serial = |adb_path: &str| -> Option<String> {
        let output = Command::new(adb_path).arg("devices").output().ok()?;
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        stdout_str
            .lines()
            .skip(1) // Skip header
            .find(|line| line.contains("emulator") && line.contains("device"))
            .and_then(|line| line.split_whitespace().next().map(|s| s.to_string()))
    };

    // Check if an emulator is already running
    let mut serial = get_emulator_serial(&adb_cmd);

    if let Some(ref s) = serial {
        println!(
            "  {} Using already-running emulator: {}",
            "→".bright_blue(),
            s.bright_cyan()
        );
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
        println!(
            "  {} Available AVDs: {}",
            "→".bright_blue(),
            avd_list.join(", ")
        );
        println!(
            "  {} Starting emulator: {}...",
            "→".bright_blue(),
            avd_name.bright_cyan()
        );

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

        serial = get_emulator_serial(&adb_cmd);
    }

    // Wait for device to be ready
    for i in 1..=30 {
        if serial.is_none() {
            serial = get_emulator_serial(&adb_cmd);
        }

        let mut check_cmd = Command::new(&adb_cmd);
        if let Some(ref s) = serial {
            check_cmd.args(["-s", s]);
        }
        check_cmd.args(["shell", "getprop", "sys.boot_completed"]);

        if let Ok(output) = check_cmd.output() {
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

    let gradlew_path = android_dir.join(if cfg!(windows) {
        "gradlew.bat"
    } else {
        "gradlew"
    });
    if !gradlew_path.exists() {
        println!("  {} Generating Gradle wrapper...", "→".bright_blue());
        let wrapper_jar = "gradle/wrapper/gradle-wrapper.jar";
        if android_dir.join(wrapper_jar).exists() {
            let wrapper_status = Command::new("java")
                .args([
                    "-classpath",
                    wrapper_jar,
                    "org.gradle.wrapper.GradleWrapperMain",
                    "wrapper",
                ])
                .current_dir(android_dir)
                .status()
                .context("Failed to generate Gradle wrapper using Java")?;
            if !wrapper_status.success() {
                println!(
                    "  {} Warning: Failed to generate Gradle wrapper",
                    "⚠".yellow()
                );
            } else if !cfg!(windows) {
                let _ = Command::new("chmod")
                    .args(["+x", "gradlew"])
                    .current_dir(android_dir)
                    .status();
            }
        } else {
            println!(
                "  {} Warning: gradle-wrapper.jar not found at {}",
                "⚠".yellow(),
                android_dir.join(wrapper_jar).display()
            );
        }
    }

    use std::process::Stdio;
    let verbose = std::env::var("JFFI_VERBOSE").is_ok();

    let mut gradle_cmd = Command::new("bash");
    gradle_cmd.arg("-c").arg(format!(
        "cd {} && export ANDROID_HOME=~/Library/Android/sdk && ./gradlew assembleDebug",
        android_dir.display()
    ));
    if !verbose {
        gradle_cmd.stdout(Stdio::null()).stderr(Stdio::null());
    }

    let build_result = gradle_cmd.status().context("Failed to run Gradle build")?;

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
    let mut install_cmd = Command::new(&adb_cmd);
    if let Some(ref s) = serial {
        install_cmd.args(["-s", s]);
    }
    install_cmd.args(["install", "-r", apk_path.to_str().unwrap()]);
    if !verbose {
        install_cmd.stdout(Stdio::null()).stderr(Stdio::null());
    }
    let install_status = install_cmd.status().context("Failed to install APK")?;

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
    let mut launch_cmd = Command::new(&adb_cmd);
    if let Some(ref s) = serial {
        launch_cmd.args(["-s", s]);
    }
    launch_cmd
        .args([
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

    println!(
        "  {} Building and launching macOS app...",
        "→".bright_blue()
    );
    project.build("", &["CODE_SIGN_IDENTITY=-"])?; // No destination needed for macOS

    println!("  {} Launching app...", "→".bright_blue());
    let home = std::env::var("HOME").context("HOME is not set; cannot locate Xcode DerivedData")?;
    let derived_data = format!("{}/Library/Developer/Xcode/DerivedData", home);

    let app_path = std::fs::read_dir(&derived_data)
        .context("Could not read DerivedData directory")?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with(&project.scheme))
        .find_map(|project_dir| {
            let app_path = project_dir
                .path()
                .join("Build/Products/Debug")
                .join(format!("{}.app", project.scheme));
            if app_path.exists() {
                Some(app_path)
            } else {
                None
            }
        })
        .context("Could not find built .app bundle")?;

    Command::new("open")
        .arg(&app_path)
        .status()
        .context("Failed to open app")?;

    println!();
    println!("{}", "  ✅ macOS app launched!".green());

    Ok(())
}

fn launch_windows_app() -> Result<()> {
    use std::process::{Command, Stdio};

    let windows_dir = std::env::current_dir()?.join("platforms/windows");
    let verbose = std::env::var("JFFI_VERBOSE").is_ok();

    // Deploy and launch the MSIX package via PowerShell
    // Detects host architecture and deploys only the matching build
    let mut ps_cmd = Command::new("powershell");
    ps_cmd.args([
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
            
            # Get the Package Name from the manifest
            $xml = [xml](Get-Content $manifest.FullName)
            $identity = $xml.Package.Identity.Name
            
            # ALWAYS remove any existing package to clear Windows package resource caches (icons, resources.pri)
            $pkg = Get-AppxPackage -Name $identity
            if ($pkg) {
                Write-Host "Removing existing package to clear resource caches..."
                Remove-AppxPackage $pkg.PackageFullName -ErrorAction SilentlyContinue
            }

            # Register the new build
            Write-Host "Registering package from manifest..."
            Add-AppxPackage -Register $manifest.FullName -ForceApplicationShutdown

            # Get the Application ID from manifest
            $appId = $xml.Package.Applications.Application.Id
            
            # Get PackageFamilyName from the registered package (not from manifest GUID)
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
        .current_dir(&windows_dir);

    if !verbose {
        ps_cmd.stdout(Stdio::null()).stderr(Stdio::null());
    }

    let status = ps_cmd.status()?;

    if !status.success() {
        anyhow::bail!("Failed to deploy/launch Windows app");
    }

    Ok(())
}

fn run_windows() -> Result<()> {
    // Build project for host/active architecture (handles icons generation, uniffi cs, and msbuild)
    println!(
        "  {} Building Windows app (host architecture)...",
        "→".bright_blue()
    );
    crate::commands::build::build_project(Some("windows".to_string()), false, false, false, false)?;

    println!();
    println!(
        "{}",
        "  🚀 Deploying and launching Windows app...".bright_cyan()
    );

    launch_windows_app()?;

    println!();
    println!("{}", "  ✅ App launched!".green());

    Ok(())
}

fn run_linux() -> Result<()> {
    println!("  {} Checking Linux dependencies...", "→".bright_blue());

    // Check for build tools (gcc)
    let needs_setup = !Command::new("gcc")
        .arg("--version")
        .output()?
        .status
        .success()
        || !Command::new("python3")
            .arg("--version")
            .output()?
            .status
            .success()
        || !Command::new("pkg-config")
            .args(["--exists", "gtk4"])
            .output()?
            .status
            .success();

    if needs_setup {
        use std::process::Stdio;
        let verbose = std::env::var("JFFI_VERBOSE").is_ok();

        println!(
            "  {} Missing dependencies. Running setup script...",
            "→".bright_blue()
        );

        let mut setup_cmd = Command::new("bash");
        setup_cmd.arg("platforms/linux/setup.sh");
        if !verbose {
            setup_cmd.stdout(Stdio::null()).stderr(Stdio::null());
        }

        let status = setup_cmd.status().context("Failed to run setup script")?;

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

    // Launch the Python app
    println!("  {} Launching app...", "→".bright_blue());

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

    let mut child = cmd
        .current_dir("platforms/linux")
        .env("GSK_RENDERER", "cairo")
        .env("GDK_DEBUG", "gl-disable")
        .env("NO_AT_BRIDGE", "1")
        .env("GTK_A11Y", "none")
        .env("LC_ALL", "C.UTF-8")
        .spawn()
        .context("Failed to launch app")?;

    let status = child.wait().context("App process error")?;

    if !status.success() {
        anyhow::bail!("App failed to run. If using remote X11, ensure XQuartz is running and SSH -X is enabled.");
    }

    println!("{}", "  ✅ App launched!".green());
    Ok(())
}

fn run_web() -> Result<()> {
    // Build the WASM first
    println!("  {} Building Rust FFI library...", "→".bright_blue());
    crate::commands::build::build_project(Some("web".to_string()), false, false, false, false)?;

    // Check if npm is installed
    let npm_check = Command::new("npm").arg("--version").output();

    if npm_check.is_err() || !npm_check.unwrap().status.success() {
        anyhow::bail!("npm is not installed. Please install Node.js and npm first.");
    }

    let config = crate::config::load_config()?;
    let web_config = config.platforms.web;
    let port = web_config.port;

    use std::process::Stdio;
    let verbose = std::env::var("JFFI_VERBOSE").is_ok();

    // Install npm dependencies if needed
    let node_modules = std::path::Path::new("platforms/web/node_modules");
    if !node_modules.exists() {
        println!("  {} Installing npm dependencies...", "→".bright_blue());
        let mut npm_cmd = Command::new("npm");
        npm_cmd.arg("install").current_dir("platforms/web");
        if !verbose {
            npm_cmd.stdout(Stdio::null()).stderr(Stdio::null());
        }
        let status = npm_cmd
            .status()
            .context("Failed to install npm dependencies")?;

        if !status.success() {
            anyhow::bail!("npm install failed");
        }
    }

    // Start Vite dev server
    let protocol = if web_config.https { "https" } else { "http" };
    let host_str = if web_config.host {
        "0.0.0.0"
    } else {
        "localhost"
    };

    println!("  {} Starting Vite dev server...", "→".bright_blue());
    println!(
        "  {} Server will open at {}://{}:{}",
        "→".bright_blue(),
        protocol,
        host_str,
        port
    );

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
    let status = vite_cmd
        .status()
        .context("Failed to start Vite dev server")?;

    if !status.success() {
        anyhow::bail!("Vite dev server failed");
    }

    println!("{}", "  ✅ Web server stopped".green());
    Ok(())
}
