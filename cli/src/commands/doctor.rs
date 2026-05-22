use anyhow::Result;
use colored::*;

pub fn run_doctor(subcommand: Option<&str>, platform: Option<&str>, release: bool) -> Result<()> {
    match subcommand {
        Some("bundle") => doctor_bundle(platform, release),
        Some(cmd) => anyhow::bail!("Unknown doctor command: {}", cmd),
        None => {
            println!("{} Please specify a subsystem to check. E.g., `jffi doctor bundle`", "ℹ".bright_blue());
            Ok(())
        }
    }
}

fn doctor_bundle(platform: Option<&str>, release: bool) -> Result<()> {
    use std::process::Command;
    println!("{}", "🩺 Running JFFI Bundle Doctor...".bright_green().bold());
    
    // Always check basic rust toolchain
    check_tool("cargo", &["--version"], "Rust package manager is required");
    check_tool("rustc", &["--version"], "Rust compiler is required");
    
    let platforms_to_check = if let Some(p) = platform {
        vec![p]
    } else {
        vec!["macos", "ios", "android", "windows", "linux", "web"]
    };
    
    for p in &platforms_to_check {
        println!("\n{}", format!("Checking prerequisites for {}:", p.to_uppercase()).bright_cyan().bold());
        
        match *p {
            "macos" | "ios" => {
                check_tool("xcodebuild", &["-version"], "Xcode is required for Apple platforms");
                check_tool("lipo", &["-info"], "lipo is required for universal binaries");
                
                if release {
                    println!("  {} Checking code signing identities...", "→".bright_blue());
                    let status = Command::new("security")
                        .args(["find-identity", "-v", "-p", "codesigning"])
                        .status();
                    if status.is_err() || !status.unwrap().success() {
                        println!("    {} 'security' command failed to list identities", "⚠".yellow());
                    }
                }
            }
            "android" => {
                // Check JAVA_HOME or gradlew
                check_tool("java", &["-version"], "Java is required for Android builds");
                
                if std::env::var("ANDROID_HOME").is_err() && std::env::var("ANDROID_SDK_ROOT").is_err() {
                    println!("  {} Missing Env Var: ANDROID_HOME or ANDROID_SDK_ROOT is not set", "✗".red());
                } else {
                    println!("  {} Env Var: Android SDK path found", "✓".green());
                }
                
                if release {
                    println!("  {} Checking keytool for Android keystores...", "→".bright_blue());
                    check_tool("keytool", &["-help"], "keytool is required to inspect/create keystores");
                    // We could also check env vars for signing
                    if std::env::var("JFFI_ANDROID_KEY_PASSWORD").is_err() || std::env::var("JFFI_ANDROID_STORE_PASSWORD").is_err() {
                        println!("    {} Signing environment variables (JFFI_ANDROID_KEY_PASSWORD, JFFI_ANDROID_STORE_PASSWORD) not set", "⚠".yellow());
                    }
                }
            }
            "windows" => {
                check_tool("MakeAppx", &["/?"], "MakeAppx.exe is required for MSIX/MSIXBundle generation (install Windows SDK)");
                if release {
                    check_tool("signtool", &["/?"], "signtool.exe is required for Windows package signing");
                }
            }
            "linux" => {
                check_tool("flatpak-builder", &["--version"], "flatpak-builder is required for Flatpak generation");
                check_tool("appimagetool", &["--version"], "appimagetool is required for AppImage generation");
            }
            "web" => {
                check_tool("npm", &["--version"], "npm is required for Web bundling");
                check_tool("wasm-bindgen", &["--version"], "wasm-bindgen CLI is required");
                check_tool("wasm-opt", &["--version"], "wasm-opt (binaryen) is required for WASM optimization");
            }
            _ => {
                println!("  {} Unknown platform: {}", "⚠".yellow(), p);
            }
        }
    }
    
    // Validate jffi.toml config if it exists
    if std::path::Path::new("jffi.toml").exists() {
        println!("\n{}", "🔍 Validating jffi.toml configuration...".bright_cyan().bold());
        match crate::config::load_config() {
            Ok(config) => {
                for p in &platforms_to_check {
                    println!("\nChecking store-readiness for platform: {}", p.to_uppercase());
                    if let Err(e) = crate::commands::bundle::validate_bundle_config(&config, p, !release) {
                        println!("  {} {}", "✗".red(), e);
                    }
                }
            }
            Err(e) => {
                println!("  {} Failed to load or parse jffi.toml: {}", "✗".red(), e);
            }
        }
    } else {
        println!("\n{} 'jffi.toml' not found. Skipping store-readiness checks.", "⚠".yellow());
    }
    
    println!("\n{}", "✅ Doctor check complete.".bright_green().bold());
    Ok(())
}

fn check_tool(name: &str, args: &[&str], msg: &str) {
    use std::process::Command;
    match Command::new(name).args(args).output() {
        Ok(output) if output.status.success() => {
            println!("  {} {} found", "✓".green(), name.bright_cyan());
        }
        _ => {
            println!("  {} {} {} - {}", "✗".red(), "Missing Tool:".bold(), name.bright_cyan(), msg);
        }
    }
}
