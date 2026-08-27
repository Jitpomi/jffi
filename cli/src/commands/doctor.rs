use anyhow::Result;
use colored::*;

pub fn run_doctor(
    subcommand: Option<&str>,
    platform: Option<&str>,
    release: bool,
    profile: &str,
) -> Result<()> {
    match subcommand {
        Some("bundle") => doctor_bundle(platform, release, profile),
        Some("config") => {
            let config = crate::config::load_config()?;
            println!(
                "{} jffi.toml is valid (schema version {})",
                "✓".green(),
                config.schema_version
            );
            Ok(())
        }
        Some(cmd) => anyhow::bail!("Unknown doctor command: {}", cmd),
        None => {
            println!(
                "{} Please specify a subsystem to check: `jffi doctor config` or `jffi doctor bundle`",
                "ℹ".bright_blue()
            );
            Ok(())
        }
    }
}

fn doctor_bundle(platform: Option<&str>, release: bool, profile: &str) -> Result<()> {
    use std::process::Command;
    println!(
        "{}",
        "🩺 Running JFFI Bundle Doctor...".bright_green().bold()
    );

    // Always check basic rust toolchain
    check_tool("cargo", &["--version"], "Rust package manager is required")?;
    check_tool("rustc", &["--version"], "Rust compiler is required")?;

    let platforms_to_check = if let Some(p) = platform {
        vec![p]
    } else {
        vec!["macos", "ios", "android", "windows", "linux", "web"]
    };

    for p in &platforms_to_check {
        println!(
            "\n{}",
            format!("Checking prerequisites for {}:", p.to_uppercase())
                .bright_cyan()
                .bold()
        );

        match *p {
            "macos" | "ios" => {
                check_tool(
                    "xcodebuild",
                    &["-version"],
                    "Xcode is required for Apple platforms",
                )?;
                check_tool(
                    "xcrun",
                    &["--find", "lipo"],
                    "lipo is required for universal binaries",
                )?;

                if release {
                    println!(
                        "  {} Checking code signing identities...",
                        "→".bright_blue()
                    );
                    let status = Command::new("security")
                        .args(["find-identity", "-v", "-p", "codesigning"])
                        .status();
                    if status.is_err() || !status.unwrap().success() {
                        println!(
                            "    {} 'security' command failed to list identities",
                            "⚠".yellow()
                        );
                    }
                }
            }
            "android" => {
                check_tool(
                    "cargo-ndk",
                    &["--version"],
                    "cargo-ndk is required for Android Rust builds",
                )?;
                // Check JAVA_HOME or gradlew
                check_tool("java", &["-version"], "Java is required for Android builds")?;

                if std::env::var("ANDROID_HOME").is_err()
                    && std::env::var("ANDROID_SDK_ROOT").is_err()
                {
                    anyhow::bail!(
                        "ANDROID_HOME or ANDROID_SDK_ROOT is required for Android builds"
                    );
                } else {
                    println!("  {} Env Var: Android SDK path found", "✓".green());
                }

                if release {
                    println!(
                        "  {} Checking keytool for Android keystores...",
                        "→".bright_blue()
                    );
                    check_tool(
                        "keytool",
                        &["-help"],
                        "keytool is required to inspect/create keystores",
                    )?;
                    // We could also check env vars for signing
                    if std::env::var("JFFI_ANDROID_KEY_PASSWORD").is_err()
                        || std::env::var("JFFI_ANDROID_STORE_PASSWORD").is_err()
                    {
                        println!("    {} Signing environment variables (JFFI_ANDROID_KEY_PASSWORD, JFFI_ANDROID_STORE_PASSWORD) not set", "⚠".yellow());
                    }
                }
            }
            "windows" => {
                check_tool(
                    "MakeAppx",
                    &["/?"],
                    "MakeAppx.exe is required for MSIX generation (install Windows SDK)",
                )?;
                if release {
                    check_tool(
                        "signtool",
                        &["/?"],
                        "signtool.exe is required for Windows package signing",
                    )?;
                }
            }
            "linux" => {
                check_tool(
                    "flatpak-builder",
                    &["--version"],
                    "flatpak-builder is required for Flatpak generation",
                )?;
            }
            "web" => {
                check_tool("npm", &["--version"], "npm is required for Web bundling")?;
                check_tool(
                    "wasm-bindgen",
                    &["--version"],
                    "wasm-bindgen CLI is required",
                )?;
                let wasm_opt_enabled = crate::config::load_config()
                    .ok()
                    .and_then(|config| config.bundle)
                    .and_then(|bundle| bundle.web)
                    .map(|web| web.wasm_opt)
                    .unwrap_or(true);
                if wasm_opt_enabled {
                    check_tool(
                        "wasm-opt",
                        &["--version"],
                        "wasm-opt (binaryen) is required when bundle.web.wasm_opt=true",
                    )?;
                }
            }
            _ => {
                println!("  {} Unknown platform: {}", "⚠".yellow(), p);
            }
        }
    }

    // Validate jffi.toml config if it exists
    if std::path::Path::new("jffi.toml").exists() {
        println!(
            "\n{}",
            "Checking project toolchain versions...".bright_cyan()
        );
        std::env::set_var("JFFI_SETUP_PLATFORM", platform.unwrap_or("<platform>"));
        crate::setup::ensure_uniffi_bindgen()?;
        println!(
            "\n{}",
            "🔍 Validating jffi.toml configuration..."
                .bright_cyan()
                .bold()
        );
        let config = crate::config::load_config()?;
        for p in &platforms_to_check {
            println!(
                "\nChecking store-readiness for platform: {}",
                p.to_uppercase()
            );
            crate::commands::bundle::validate_bundle_config(&config, p, profile, !release)?;
        }
    } else {
        println!(
            "\n{} 'jffi.toml' not found. Skipping store-readiness checks.",
            "⚠".yellow()
        );
    }

    println!("\n{}", "✅ Doctor check complete.".bright_green().bold());
    Ok(())
}

fn check_tool(name: &str, args: &[&str], msg: &str) -> Result<()> {
    use std::process::Command;
    match Command::new(name).args(args).output() {
        Ok(output) if output.status.success() => {
            println!("  {} {} found", "✓".green(), name.bright_cyan());
            Ok(())
        }
        Ok(output) => anyhow::bail!(
            "Tool {} was found but exited unsuccessfully ({}): {}",
            name,
            output.status,
            msg
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            anyhow::bail!("Missing tool {}: {}", name, msg)
        }
        Err(_) => {
            println!(
                "  {} {} found (but execution failed)",
                "✓".green(),
                name.bright_cyan()
            );
            Ok(())
        }
    }
}
