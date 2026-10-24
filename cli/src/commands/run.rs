use anyhow::{Context, Result};
use colored::*;
use std::process::Command;

use crate::platform::Platform;

pub fn run_project(
    platform_str: &str,
    device: bool,
    release: bool,
    device_id: Option<&str>,
) -> Result<()> {
    let platform = Platform::from_str(platform_str)
        .ok_or_else(|| anyhow::anyhow!("Unknown platform: {}", platform_str))?;

    if !platform.is_host_supported() {
        anyhow::bail!("{} cannot be run on this host", platform.as_str());
    }
    if (device || device_id.is_some()) && !matches!(platform, Platform::Ios | Platform::Android) {
        anyhow::bail!("--device and --device-id apply only to iOS and Android");
    }
    if device && platform == Platform::Ios && device_id.is_none() {
        anyhow::bail!(
            "Physical iOS runs require --device-id <UDID>; use xcrun devicectl list devices"
        );
    }
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
    if !config
        .platforms
        .enabled
        .iter()
        .any(|enabled| enabled == platform.as_str())
    {
        anyhow::bail!(
            "Platform '{}' is not enabled in jffi.toml",
            platform.as_str()
        );
    }
    crate::commands::build::sync_configs_to_platforms(&config)?;

    // Build first
    crate::commands::build::build_project(
        Some(platform_str.to_string()),
        false,
        release,
        device && platform == Platform::Ios,
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
        Platform::Ios if device => run_ios_device(release, device_id),
        Platform::Ios => run_ios(release, device_id),
        Platform::Macos => run_macos(release),
        Platform::Android => run_android(release, device, device_id),
        Platform::Windows => run_windows(release),
        Platform::Linux => run_linux(release),
        Platform::Web => run_web(),
    }
}

fn run_ios_device(release: bool, device_id: Option<&str>) -> Result<()> {
    use crate::platform::XcodeProject;

    println!("  {} Finding Xcode project...", "→".bright_blue());
    let project = XcodeProject::find(crate::platform::Platform::Ios)?;

    let id = device_id.context("Pass --device-id <identifier> from `xcrun devicectl list devices` for a physical iOS device")?;
    let destination = format!("id={}", id);
    project.build(
        &destination,
        &[
            "-configuration",
            native_configuration(release),
            "-derivedDataPath",
            "target/jffi/xcode/ios-device",
            "-allowProvisioningUpdates",
        ],
    )?;
    let app = std::path::PathBuf::from("target/jffi/xcode/ios-device/Build/Products")
        .join(format!("{}-iphoneos", native_configuration(release)))
        .join(format!("{}.app", project.scheme));
    let config = crate::config::load_config()?;
    let bundle_id = config
        .bundle
        .as_ref()
        .and_then(|b| b.identifier.as_deref())
        .unwrap_or(&config.platforms.ios.bundle_id);
    checked_status(
        Command::new("xcrun")
            .args(["devicectl", "device", "install", "app", "--device", id])
            .arg(&app),
        "iOS device installation",
    )?;
    checked_status(
        Command::new("xcrun").args([
            "devicectl",
            "device",
            "process",
            "launch",
            "--device",
            id,
            bundle_id,
        ]),
        "iOS device launch",
    )?;
    Ok(())
}

fn run_ios(release: bool, device_id: Option<&str>) -> Result<()> {
    use crate::platform::{IOSSimulator, XcodeProject};

    println!("  {} Finding Xcode project...", "→".bright_blue());
    let project = XcodeProject::find(crate::platform::Platform::Ios)?;

    println!("  {} Finding available simulator...", "→".bright_blue());
    let (simulator_name, simulator_id) = IOSSimulator::get_available(device_id)?;
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
    project.build(
        &destination,
        &[
            "CODE_SIGN_IDENTITY=-",
            "-configuration",
            native_configuration(release),
            "-derivedDataPath",
            "target/jffi/xcode/ios",
        ],
    )?;

    let app_name = &project.scheme;

    println!("  {} Launching app in simulator...", "→".bright_blue());

    // Find the built app
    let app_path = apple_app_path("ios", app_name, release);

    // Use simulator abstraction
    let sim = IOSSimulator;
    sim.boot(&simulator_id)?;
    sim.open_app()?;

    // Get bundle ID from config
    let config = crate::config::load_config()?;
    let bundle_id = config
        .bundle
        .as_ref()
        .and_then(|b| b.identifier.clone())
        .unwrap_or_else(|| config.platforms.ios.bundle_id.clone());

    // Install and launch with retry logic
    for attempt in 0..3 {
        sim.install_app(&simulator_id, &app_path)?;
        sim.open_app()?; // Bring to foreground

        match sim.launch_app(&simulator_id, &bundle_id) {
            Ok(_) => break,
            Err(e) if attempt < 2 => {
                eprintln!("  ⚠ Launch failed (attempt {}/3): {}", attempt + 1, e);
                eprintln!("  → Cleaning up and retrying...");
                let _ = Command::new("xcrun")
                    .args(["simctl", "uninstall", &simulator_id, &bundle_id])
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

fn run_android(release: bool, physical: bool, device_id: Option<&str>) -> Result<()> {
    use crate::platform::AndroidProject;

    println!("  {} Preparing Android device...", "→".bright_blue());

    let android = AndroidProject::find()?;

    let adb_cmd = android
        .find_tool("adb")
        .context("adb not found. Install Android SDK platform-tools.")?;
    let get_serial = |adb_path: &str| -> Result<Option<String>> {
        let output = Command::new(adb_path).arg("devices").output()?;
        if !output.status.success() {
            anyhow::bail!("adb devices failed");
        }
        select_android_device(
            &String::from_utf8_lossy(&output.stdout),
            physical,
            device_id,
        )
    };
    let mut serial = get_serial(&adb_cmd)?;
    if serial.is_none() && (physical || device_id.is_some()) {
        anyhow::bail!(
            "Requested Android device is not connected and authorized; check `adb devices`"
        );
    }
    if let Some(ref s) = serial {
        println!(
            "  {} Using Android device: {}",
            "→".bright_blue(),
            s.bright_cyan()
        );
    } else {
        let emulator_cmd = android
            .find_tool("emulator")
            .context("No running emulator; install an Android Virtual Device in Android Studio")?;
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

        serial = get_serial(&adb_cmd)?;
    }

    let mut ready = false;
    // Wait for device to be ready
    for i in 1..=30 {
        if serial.is_none() {
            serial = get_serial(&adb_cmd)?;
        }

        if serial.is_none() {
            std::thread::sleep(std::time::Duration::from_secs(1));
            continue;
        }
        let mut check_cmd = Command::new(&adb_cmd);
        if let Some(ref s) = serial {
            check_cmd.args(["-s", s]);
        }
        check_cmd.args(["shell", "getprop", "sys.boot_completed"]);

        if let Ok(output) = check_cmd.output() {
            if String::from_utf8_lossy(&output.stdout).trim() == "1" {
                ready = true;
                break;
            }
        }

        if i % 5 == 0 {
            println!("  {} Still waiting... ({}/30s)", "→".bright_blue(), i);
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    if !ready {
        anyhow::bail!("Android device did not finish booting within 30 seconds");
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

    let mut gradle_cmd = if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "gradlew.bat"]);
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.arg("gradlew");
        cmd
    };
    gradle_cmd.current_dir(android_dir);
    if release {
        // Local release runs use the development key; distribution uses `bundle` profiles.
        let init_dir = std::env::current_dir()?.join("target/jffi/generated/android");
        std::fs::create_dir_all(&init_dir)?;
        let init_script = init_dir.join("run-release.gradle");
        std::fs::write(&init_script, "allprojects { p -> p.plugins.withId('com.android.application') { p.extensions.getByName('androidComponents').finalizeDsl { android -> android.buildTypes.getByName('release').signingConfig = android.signingConfigs.getByName('debug') } } }\n")?;
        gradle_cmd.arg("--init-script").arg(init_script);
        println!("  Local release run uses the Android development signing key; use jffi bundle for distribution.");
    }
    gradle_cmd.arg(if release {
        "assembleRelease"
    } else {
        "assembleDebug"
    });
    if !verbose {
        gradle_cmd.stdout(Stdio::null()).stderr(Stdio::null());
    }

    let build_result = gradle_cmd.status().context("Failed to run Gradle build")?;

    if !build_result.success() {
        anyhow::bail!("Gradle build failed. Check the error messages above.");
    }

    println!("  {} Installing APK to device...", "→".bright_blue());

    // Find the built APK
    let apk_path = android_dir.join(format!(
        "app/build/outputs/apk/{0}/app-{0}.apk",
        cargo_profile(release)
    ));

    if !apk_path.exists() {
        anyhow::bail!("APK not found at: {}", apk_path.display());
    }

    // Install APK using adb
    let mut install_cmd = Command::new(&adb_cmd);
    if let Some(ref s) = serial {
        install_cmd.args(["-s", s]);
    }
    install_cmd.args(["install", "-r", apk_path.to_str().unwrap()]);
    let install_output = install_cmd.output().context("Failed to install APK")?;

    if verbose {
        print!("{}", String::from_utf8_lossy(&install_output.stdout));
        eprint!("{}", String::from_utf8_lossy(&install_output.stderr));
    }

    if !install_output.status.success() {
        let stderr = String::from_utf8_lossy(&install_output.stderr);
        let stdout = String::from_utf8_lossy(&install_output.stdout);
        let details = if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        };
        anyhow::bail!("Failed to install APK on device: {details}");
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
    let launch_status = launch_cmd
        .args([
            "shell",
            "am",
            "start",
            "-n",
            &format!("{}/{}", package_name, activity),
        ])
        .status()
        .context("Failed to launch app")?;
    if !launch_status.success() {
        anyhow::bail!("Android activity launch failed");
    }

    println!();
    println!("{}", "  ✅ App launched on device!".green());
    println!();

    Ok(())
}

fn run_macos(release: bool) -> Result<()> {
    use crate::platform::XcodeProject;

    println!("  {} Finding Xcode project...", "→".bright_blue());
    let project = XcodeProject::find(crate::platform::Platform::Macos)?;

    println!(
        "  {} Building and launching macOS app...",
        "→".bright_blue()
    );
    project.build(
        "",
        &[
            "CODE_SIGN_IDENTITY=-",
            "-configuration",
            native_configuration(release),
            "-derivedDataPath",
            "target/jffi/xcode/macos",
        ],
    )?; // No destination needed for macOS

    println!("  {} Launching app...", "→".bright_blue());
    let app_path = apple_app_path("macos", &project.scheme, release);

    checked_status(Command::new("open").arg(&app_path), "macOS app launch")?;

    println!();
    println!("{}", "  ✅ macOS app launched!".green());

    Ok(())
}

const WINDOWS_LAUNCH_SCRIPT: &str = r#"
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
            $manifest = Get-Item -LiteralPath (Join-Path $env:JFFI_RUN_OUTPUT 'AppxManifest.xml')
            if (-not $manifest) { 
                throw "AppxManifest.xml not found for $platformDir. Build the project first with: jffi build --platform windows" 
            }

            Write-Host "Found manifest: $($manifest.FullName)"
            
            # Get the Package Name from the manifest
            $xml = [xml](Get-Content $manifest.FullName)
            $identity = $xml.Package.Identity.Name
            
            # Register/update in place. Uninstalling here would discard application data.
            Add-AppxPackage -Register $manifest.FullName -ForceApplicationShutdown -ForceUpdateFromAnyVersion

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
            "#;

fn windows_host_platform() -> &'static str {
    match std::env::var("PROCESSOR_ARCHITECTURE").as_deref() {
        Ok("x86") => "x86",
        Ok("ARM64") => "ARM64",
        _ => "x64",
    }
}

fn launch_windows_app(release: bool) -> Result<()> {
    use std::process::{Command, Stdio};

    let windows_dir = std::env::current_dir()?.join("platforms/windows");
    let verbose = std::env::var("JFFI_VERBOSE").is_ok();

    // Deploy and launch the MSIX package via PowerShell
    // Detects host architecture and deploys only the matching build
    let mut ps_cmd = Command::new("powershell");
    ps_cmd
        .args([
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            WINDOWS_LAUNCH_SCRIPT,
        ])
        .current_dir(&windows_dir)
        .env(
            "JFFI_RUN_OUTPUT",
            std::env::current_dir()?.join(crate::platform::windows_output_dir(
                windows_host_platform(),
                release,
            )),
        );

    if !verbose {
        ps_cmd.stdout(Stdio::null()).stderr(Stdio::null());
    }

    let status = ps_cmd.status()?;

    if !status.success() {
        anyhow::bail!("Failed to deploy/launch Windows app");
    }

    Ok(())
}

fn run_windows(release: bool) -> Result<()> {
    // Build project for host/active architecture (handles icons generation, uniffi cs, and msbuild)
    println!(
        "  {} Building Windows app (host architecture)...",
        "→".bright_blue()
    );
    // Already built with the requested configuration by run_project.

    println!();
    println!(
        "{}",
        "  🚀 Deploying and launching Windows app...".bright_cyan()
    );

    launch_windows_app(release)?;

    println!();
    println!("{}", "  ✅ App launched!".green());

    Ok(())
}

fn run_linux(release: bool) -> Result<()> {
    // run_project has already checked prerequisites and built the requested profile.
    // Copy the .so file to platforms/linux with the correct name
    let lib_path = std::fs::read_dir(format!("target/{}", cargo_profile(release)))
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

fn cargo_profile(release: bool) -> &'static str {
    if release {
        "release"
    } else {
        "debug"
    }
}

fn native_configuration(release: bool) -> &'static str {
    if release {
        "Release"
    } else {
        "Debug"
    }
}

fn apple_app_path(platform: &str, name: &str, release: bool) -> std::path::PathBuf {
    let configuration = native_configuration(release);
    let products = if platform == "ios" {
        format!("{}-iphonesimulator", configuration)
    } else {
        configuration.to_string()
    };
    std::path::PathBuf::from("target/jffi/xcode")
        .join(platform)
        .join("Build/Products")
        .join(products)
        .join(format!("{}.app", name))
}

fn checked_status(command: &mut Command, operation: &str) -> Result<()> {
    if !command
        .status()
        .with_context(|| format!("Failed to execute {}", operation))?
        .success()
    {
        anyhow::bail!("{} failed", operation);
    }
    Ok(())
}

fn select_android_device(
    output: &str,
    physical: bool,
    requested: Option<&str>,
) -> Result<Option<String>> {
    let candidates: Vec<&str> = output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let id = fields.next()?;
            if fields.next()? != "device" {
                return None;
            }
            if let Some(requested) = requested {
                return (id == requested && (!physical || !id.starts_with("emulator-")))
                    .then_some(id);
            }
            (physical != id.starts_with("emulator-")).then_some(id)
        })
        .collect();
    if candidates.len() > 1 {
        anyhow::bail!(
            "Multiple Android devices match: {}. Pass --device-id",
            candidates.join(", ")
        );
    }
    Ok(candidates.first().map(|id| id.to_string()))
}

#[cfg(test)]
mod parity_tests {
    use super::*;
    #[test]
    fn windows_registration_preserves_user_data_and_uses_the_selected_output() {
        assert!(!super::WINDOWS_LAUNCH_SCRIPT.contains("Remove-AppxPackage"));
        assert!(!super::WINDOWS_LAUNCH_SCRIPT.contains("Get-ChildItem"));
        assert!(super::WINDOWS_LAUNCH_SCRIPT.contains("$env:JFFI_RUN_OUTPUT"));
        assert!(super::WINDOWS_LAUNCH_SCRIPT.contains("-ForceUpdateFromAnyVersion"));
    }

    #[test]
    fn release_apple_paths_do_not_fall_back_to_debug() {
        assert_eq!(
            apple_app_path("ios", "Sample", true),
            std::path::PathBuf::from(
                "target/jffi/xcode/ios/Build/Products/Release-iphonesimulator/Sample.app"
            )
        );
        assert_eq!(
            apple_app_path("macos", "Sample", false),
            std::path::PathBuf::from("target/jffi/xcode/macos/Build/Products/Debug/Sample.app")
        );
    }
    #[test]
    fn android_selection_handles_physical_emulated_and_unauthorized_devices() {
        let devices = "List of devices attached\nemulator-5554\tdevice\nphone-1\tdevice\nphone-2\tunauthorized\n";
        assert_eq!(
            select_android_device(devices, true, None)
                .unwrap()
                .as_deref(),
            Some("phone-1")
        );
        assert_eq!(
            select_android_device(devices, false, None)
                .unwrap()
                .as_deref(),
            Some("emulator-5554")
        );
        assert_eq!(
            select_android_device(devices, false, Some("phone-1"))
                .unwrap()
                .as_deref(),
            Some("phone-1")
        );
        assert!(select_android_device(devices, true, Some("phone-2"))
            .unwrap()
            .is_none());
        assert!(select_android_device("one device\ntwo device", true, None).is_err());
    }
}
