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
    println!(
        "{}",
        "🩺 Running JFFI Bundle Doctor...".bright_green().bold()
    );

    // Always check basic rust toolchain
    check_tool("cargo", &["--version"], "Rust package manager is required")?;
    check_tool("rustc", &["--version"], "Rust compiler is required")?;

    let config = crate::config::load_config()?;
    let platforms_to_check = select_platforms(&config.platforms.enabled, platform)?;

    for p in &platforms_to_check {
        let selected = crate::platform::Platform::from_str(p)
            .expect("platform selection validates platform names");
        crate::setup::setup_platform_for_build(&selected)?;

        println!(
            "\n{}",
            format!("Checking prerequisites for {}:", p.to_uppercase())
                .bright_cyan()
                .bold()
        );

        match p.as_str() {
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
            }
            "android" => {
                check_tool(
                    "cargo",
                    &["ndk", "--version"],
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
                }
            }
            "windows" => {
                check_makeappx(
                    &std::env::var("JFFI_MAKEAPPX")
                        .ok()
                        .or_else(|| crate::commands::bundle::find_sdk_tool("makeappx"))
                        .unwrap_or_else(|| "MakeAppx".into()),
                )?;
                if release {
                    check_tool(
                        &std::env::var("JFFI_SIGNTOOL")
                            .ok()
                            .or_else(|| crate::commands::bundle::find_sdk_tool("signtool"))
                            .unwrap_or_else(|| "signtool".into()),
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
                {
                    let linux = config
                        .bundle
                        .as_ref()
                        .and_then(|bundle| bundle.linux.as_ref())
                        .cloned()
                        .unwrap_or_default();
                    for reference in [&linux.runtime, &linux.sdk] {
                        let reference = format!("{}//{}", reference, linux.runtime_version);
                        let installed = [true, false].iter().any(|user| {
                            let mut command = std::process::Command::new("flatpak");
                            command.arg("info");
                            if *user {
                                command.arg("--user");
                            }
                            command
                                .arg(&reference)
                                .output()
                                .is_ok_and(|output| output.status.success())
                        });
                        if !installed {
                            anyhow::bail!(
                                "Missing Flatpak reference {}. Install it with `flatpak install --user flathub {}` after configuring Flathub, or allow `jffi bundle` to reconcile runtimes",
                                reference, reference
                            );
                        }
                    }
                }
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
            _ => unreachable!("platform selection validates platform names"),
        }
    }

    println!(
        "\n{}",
        "Checking project toolchain versions...".bright_cyan()
    );
    std::env::set_var(
        "JFFI_SETUP_PLATFORM",
        platform.unwrap_or("enabled host platforms"),
    );
    crate::setup::ensure_uniffi_bindgen()?;
    println!(
        "\n{}",
        "🔍 Validating jffi.toml configuration..."
            .bright_cyan()
            .bold()
    );
    for p in &platforms_to_check {
        println!(
            "\nChecking store-readiness for platform: {}",
            p.to_uppercase()
        );
        crate::commands::bundle::validate_bundle_config(&config, p, profile, !release)?;

        if release && matches!(p.as_str(), "ios" | "macos") {
            let certificate = config
                .bundle
                .as_ref()
                .and_then(|bundle| bundle.signing.as_ref())
                .and_then(|signing| signing.profiles.as_ref())
                .and_then(|profiles| profiles.get(profile))
                .and_then(|profile| profile.apple.as_ref())
                .and_then(|apple| apple.signing_certificate.as_deref())
                .expect("bundle validation requires an Apple signing certificate");
            check_apple_signing_identity(certificate)?;
        }
    }

    println!("\n{}", "✅ Doctor check complete.".bright_green().bold());
    Ok(())
}

fn select_platforms(enabled: &[String], requested: Option<&str>) -> Result<Vec<String>> {
    if let Some(requested) = requested {
        if requested == "all" {
            anyhow::bail!(
                "Use `jffi doctor bundle` to check enabled platforms supported by this host; `--platform all` is not supported"
            );
        }
        let platform = crate::platform::Platform::from_str(requested)
            .ok_or_else(|| anyhow::anyhow!("Unknown platform: {}", requested))?;
        let canonical = platform.as_str();
        if !enabled.iter().any(|value| value == canonical) {
            anyhow::bail!("Platform '{}' is not enabled in jffi.toml", canonical);
        }
        return Ok(vec![canonical.to_string()]);
    }

    let selected: Vec<String> = enabled
        .iter()
        .filter_map(|value| crate::platform::Platform::from_str(value))
        .filter(|platform| platform.is_host_supported())
        .map(|platform| platform.as_str().to_string())
        .collect();
    if selected.is_empty() {
        anyhow::bail!("No enabled platforms can be checked on this host; pass --platform explicitly on a compatible host");
    }
    Ok(selected)
}

fn check_apple_signing_identity(certificate: &str) -> Result<()> {
    println!("  {} Checking Apple signing identity...", "→".bright_blue());
    let output = std::process::Command::new("security")
        .args(["find-identity", "-v", "-p", "codesigning"])
        .output()
        .map_err(|error| anyhow::anyhow!("Failed to list Apple signing identities: {}", error))?;
    if !output.status.success() {
        anyhow::bail!(
            "Apple signing identity lookup failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let identities = String::from_utf8_lossy(&output.stdout);
    if !identities.contains(certificate) {
        anyhow::bail!(
            "Required Apple signing identity '{}' is not installed or valid",
            certificate
        );
    }
    println!("  {} Apple signing identity found", "✓".green());
    Ok(())
}

// MakeAppx prints valid help with exit code 1 on Windows SDK releases. Accept
// that specific response, not arbitrary failures or other tools' nonzero exits.
fn makeappx_help_succeeded(code: Option<i32>, stdout: &[u8], stderr: &[u8]) -> bool {
    if code == Some(0) {
        return true;
    }
    if code != Some(1) {
        return false;
    }
    let help = format!(
        "{}\n{}",
        String::from_utf8_lossy(stdout),
        String::from_utf8_lossy(stderr)
    )
    .to_ascii_lowercase();
    help.contains("makeappx")
        && ["pack", "unpack", "bundle", "/?"].iter().all(|word| {
            help.split(|c: char| !c.is_ascii_alphanumeric() && c != '/' && c != '?')
                .any(|token| token == *word)
        })
}

fn check_makeappx(name: &str) -> Result<()> {
    check_tool_with(
        name,
        &["/?"],
        "MakeAppx.exe is required for MSIX generation (install Windows SDK)",
        |output| makeappx_help_succeeded(output.status.code(), &output.stdout, &output.stderr),
    )
}

fn check_tool(name: &str, args: &[&str], msg: &str) -> Result<()> {
    check_tool_with(name, args, msg, |output| output.status.success())
}

fn check_tool_with(
    name: &str,
    args: &[&str],
    msg: &str,
    succeeded: impl Fn(&std::process::Output) -> bool,
) -> Result<()> {
    use std::process::Command;
    match Command::new(name).args(args).output() {
        Ok(output) if succeeded(&output) => {
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
        Err(error) => anyhow::bail!("Could not execute tool {}: {} ({})", name, msg, error),
    }
}

#[cfg(test)]
mod tests {
    use super::{makeappx_help_succeeded, select_platforms};

    const MAKEAPPX_HELP: &[u8] = b"Microsoft (R) MakeAppx Tool\nUsage:\n MakeAppx <command> [options]\n pack -- Create a new app package\n unpack -- Extract files\n bundle -- Create a bundle\n /? -- Display help";

    #[test]
    fn makeappx_accepts_help_exit_one_on_stdout_or_stderr() {
        assert!(makeappx_help_succeeded(Some(1), MAKEAPPX_HELP, b""));
        assert!(makeappx_help_succeeded(Some(1), b"", MAKEAPPX_HELP));
        assert!(makeappx_help_succeeded(Some(0), MAKEAPPX_HELP, b""));
    }

    #[test]
    fn makeappx_rejects_real_failures_and_unrecognized_output() {
        for code in [Some(2), Some(-1), None] {
            assert!(!makeappx_help_succeeded(code, MAKEAPPX_HELP, b""));
        }
        assert!(!makeappx_help_succeeded(Some(1), b"", b""));
        assert!(!makeappx_help_succeeded(
            Some(1),
            b"",
            b"MakeAppx: DLL initialization failed"
        ));
        assert!(!makeappx_help_succeeded(
            Some(1),
            b"some other tool /? pack unpack bundle",
            b""
        ));
    }

    #[test]
    fn makeappx_missing_executable_remains_an_error() {
        let missing =
            std::env::temp_dir().join(format!("jffi-missing-makeappx-{}", uuid::Uuid::new_v4()));
        let error = super::check_makeappx(missing.to_str().unwrap()).unwrap_err();
        assert!(error.to_string().contains("Missing tool"));
    }

    #[cfg(unix)]
    #[test]
    fn other_tool_help_failures_remain_errors() {
        let error = super::check_tool("sh", &["-c", "exit 1"], "tool required").unwrap_err();
        assert!(error.to_string().contains("exited unsuccessfully"));
    }

    #[test]
    fn rejects_unknown_or_disabled_explicit_platforms() {
        let enabled = vec!["web".to_string()];
        assert!(select_platforms(&enabled, Some("plan9")).is_err());
        assert!(select_platforms(&enabled, Some("android")).is_err());
        assert!(select_platforms(&enabled, Some("all")).is_err());
    }

    #[test]
    fn accepts_enabled_explicit_platform() {
        let enabled = vec!["web".to_string()];
        assert_eq!(select_platforms(&enabled, Some("web")).unwrap(), ["web"]);
    }

    #[test]
    fn defaults_to_enabled_platforms_supported_by_host() {
        let enabled = vec!["web".to_string(), "android".to_string()];
        assert_eq!(
            select_platforms(&enabled, None).unwrap(),
            ["web", "android"]
        );
    }
}
