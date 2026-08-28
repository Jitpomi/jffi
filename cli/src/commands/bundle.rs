use anyhow::{Context, Result};
use colored::*;
use std::fs;
use std::io::IsTerminal;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::{BundleAndroidConfig, Config};

#[derive(Debug, Clone, Copy)]
pub struct BundleOptions<'a> {
    pub platform: &'a str,
    pub format: Option<&'a str>,
    pub profile: &'a str,
    pub no_sign: bool,
    pub notarize: bool,
    pub dry_run: bool,
    pub print_plan: bool,
    pub print_commands: bool,
}

fn should_sync_native_configs(dry_run: bool, print_plan: bool) -> bool {
    !dry_run && !print_plan
}

fn should_reconcile_bundle_prerequisites(dry_run: bool, print_plan: bool) -> bool {
    !dry_run && !print_plan
}

pub fn bundle_project(options: BundleOptions<'_>) -> Result<()> {
    let BundleOptions {
        platform,
        format,
        profile,
        no_sign,
        notarize,
        dry_run,
        print_plan,
        print_commands,
    } = options;
    // Validate we are in a JFFI project
    if !Path::new("jffi.toml").exists() {
        anyhow::bail!("Error: Not in a JFFI project directory. 'jffi.toml' not found.");
    }

    let config = crate::config::load_config()?;

    validate_requested_format(platform, format)?;
    validate_enabled_platform(&config, platform)?;

    // Planning and dry-run modes must be safe to use as read-only CI
    // preflights. Native project synchronization can rewrite generated files,
    // so perform it only for an actual bundle operation.
    if should_sync_native_configs(dry_run, print_plan) {
        crate::commands::build::sync_configs_to_platforms(&config)?;
    }

    // Validate configurations for store compatibility
    validate_bundle_config(&config, platform, profile, no_sign)?;

    if print_plan {
        println!("{}", "📋 Bundle Plan:".bright_cyan().bold());
        println!("  Platform: {}", platform);
        println!("  Format:   {}", format.unwrap_or("default"));
        println!("  Profile:  {}", profile);
        println!("  Sign:     {}", !no_sign);
        println!("  Notarize: {}", notarize);
        return Ok(());
    }

    if dry_run {
        println!(
            "{}",
            "🏜️ Dry run enabled. No actual commands will be executed.".yellow()
        );
    }

    if should_reconcile_bundle_prerequisites(dry_run, print_plan) {
        let platform = crate::platform::Platform::from_str(platform)
            .ok_or_else(|| anyhow::anyhow!("Unknown platform: {}", platform))?;
        crate::setup::reconcile_platform(&platform)?;
    }

    println!(
        "{}",
        format!("📦 Bundling for {}...", platform)
            .bright_green()
            .bold()
    );

    if !dry_run {
        crate::commands::icons::generate_icons(&config, platform)?;
    }

    match platform {
        "android" => bundle_android(&config, format, profile, no_sign, dry_run, print_commands),
        "macos" => bundle_macos(
            &config,
            format,
            profile,
            no_sign,
            notarize,
            dry_run,
            print_commands,
        ),
        "ios" => bundle_ios(&config, profile, no_sign, dry_run, print_commands),
        "windows" => bundle_windows(&config, format, profile, no_sign, dry_run, print_commands),
        "linux" => bundle_linux(&config, format, profile, dry_run, print_commands),
        "web" => bundle_web(&config, dry_run, print_commands),
        "all" => {
            anyhow::bail!("Bundle each platform explicitly; `--platform all` is not supported")
        }
        _ => anyhow::bail!("Unknown platform: {}", platform),
    }
}

fn validate_enabled_platform(config: &Config, platform: &str) -> Result<()> {
    if config
        .platforms
        .enabled
        .iter()
        .any(|enabled| enabled == platform)
    {
        Ok(())
    } else {
        anyhow::bail!("Platform '{}' is not enabled in jffi.toml", platform)
    }
}

fn validate_requested_format(platform: &str, format: Option<&str>) -> Result<()> {
    let supported: &[&str] = match platform {
        "android" => &["aab", "apk"],
        "macos" => &["app", "dmg", "pkg"],
        "ios" => &["ipa"],
        "windows" => &["msix"],
        "linux" => &["flatpak"],
        "web" => &["web"],
        "all" => {
            anyhow::bail!("Bundle each platform explicitly; `--platform all` is not supported")
        }
        _ => anyhow::bail!("Unknown platform: {}", platform),
    };
    let Some(format) = format else {
        return Ok(());
    };
    if supported.contains(&format) {
        Ok(())
    } else {
        anyhow::bail!(
            "Unsupported bundle format `{}` for {}. Supported formats: {}",
            format,
            platform,
            if supported.is_empty() {
                "none".to_string()
            } else {
                supported.join(", ")
            }
        )
    }
}

fn validate_format_list(platform: &str, formats: &[String]) -> Result<()> {
    for format in formats {
        validate_requested_format(platform, Some(format))?;
    }
    Ok(())
}

fn verify_nonempty_file(path: &Path, artifact: &str) -> Result<()> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("{} was not created at {}", artifact, path.display()))?;
    if !metadata.is_file() || metadata.len() == 0 {
        anyhow::bail!("{} is empty or invalid: {}", artifact, path.display());
    }
    Ok(())
}

fn copy_directory_contents(source: &Path, destination: &Path) -> Result<usize> {
    let mut copied = 0;
    for entry in
        fs::read_dir(source).with_context(|| format!("Failed to read {}", source.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            fs::create_dir_all(&destination_path)?;
            copied += copy_directory_contents(&source_path, &destination_path)?;
        } else if entry.file_type()?.is_file() {
            fs::copy(&source_path, &destination_path).with_context(|| {
                format!(
                    "Failed to copy {} to {}",
                    source_path.display(),
                    destination_path.display()
                )
            })?;
            copied += 1;
        }
    }
    Ok(copied)
}

fn optimize_wasm_files(directory: &Path) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            optimize_wasm_files(&path)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("wasm") {
            let optimized = path.with_extension("wasm.optimized");
            let status = Command::new("wasm-opt")
                .args(["-Oz", "-o"])
                .arg(&optimized)
                .arg(&path)
                .status()
                .context(
                    "Failed to run wasm-opt; install Binaryen or set bundle.web.wasm_opt=false",
                )?;
            if !status.success() {
                anyhow::bail!("wasm-opt failed for {}", path.display());
            }
            fs::rename(&optimized, &path)?;
        }
    }
    Ok(())
}

fn bundle_android(
    config: &Config,
    format: Option<&str>,
    profile: &str,
    no_sign: bool,
    dry_run: bool,
    print_commands: bool,
) -> Result<()> {
    let bundle_config = config.bundle.clone().unwrap_or_default();
    let android_config = bundle_config
        .android
        .unwrap_or_else(|| BundleAndroidConfig {
            formats: vec!["aab".to_string(), "apk".to_string()],
            abis: vec![
                "arm64-v8a".to_string(),
                "armeabi-v7a".to_string(),
                "x86_64".to_string(),
            ],
            min_sdk: 23,
            compile_sdk: 36,
            build_type: "release".to_string(),
            split_debug_symbols: true,
            icon: None,
        });

    let formats = if let Some(f) = format {
        vec![f.to_string()]
    } else {
        android_config.formats.clone()
    };
    validate_format_list("android", &formats)?;

    let abis = &android_config.abis;

    println!("  {} Generating abi.gradle...", "→".bright_blue());

    let generated_dir = PathBuf::from("target/jffi/generated/android");
    if !dry_run {
        fs::create_dir_all(&generated_dir)?;

        let abi_list = abis
            .iter()
            .map(|abi| format!("'{}'", abi))
            .collect::<Vec<_>>()
            .join(", ");

        let mut gradle_glue = format!(
            "// Auto-generated by JFFI. Do not edit manually.\n\
             android {{\n    defaultConfig {{\n        ndk {{\n            abiFilters.clear()\n            abiFilters.addAll([{}])\n        }}\n    }}\n",
            abi_list
        );

        if !no_sign {
            if let Some(signing) = &bundle_config.signing {
                if let Some(profiles) = &signing.profiles {
                    if let Some(prof) = profiles.get(profile) {
                        if let Some(android_prof) = &prof.android {
                            gradle_glue.push_str("    signingConfigs {\n        release {\n");
                            if let Some(keystore) = &android_prof.keystore_path {
                                // jffi.toml paths are relative to project root.
                                // Gradle runs from platforms/android/ so strip that prefix
                                // and use rootProject.file() to resolve from the Gradle root.
                                let gradle_root = "platforms/android/";
                                let keystore_from_gradle_root = keystore
                                    .strip_prefix(gradle_root)
                                    .unwrap_or(keystore)
                                    .to_string();
                                gradle_glue.push_str(&format!(
                                    "            storeFile rootProject.file(\"{}\")\n",
                                    keystore_from_gradle_root
                                ));
                            }
                            if android_prof.store_password_env.is_some() {
                                gradle_glue.push_str("            storePassword System.getenv(\"JFFI_ANDROID_STORE_PASSWORD\")\n");
                            }
                            if let Some(alias) = &android_prof.key_alias {
                                gradle_glue
                                    .push_str(&format!("            keyAlias \"{}\"\n", alias));
                            }
                            if android_prof.key_password_env.is_some() {
                                gradle_glue.push_str("            keyPassword System.getenv(\"JFFI_ANDROID_KEY_PASSWORD\")\n");
                            }
                            gradle_glue.push_str("        }\n    }\n");
                        }
                    }
                }
            }
        }

        // Ask the Android Gradle Plugin to package native symbols separately.
        // Google Play uses this archive to symbolicate native Rust/C/C++ crashes,
        // while the libraries delivered to users remain optimized for release.
        gradle_glue.push_str("    buildTypes {\n        release {\n");
        if android_config.split_debug_symbols {
            gradle_glue.push_str("            ndk.debugSymbolLevel = 'SYMBOL_TABLE'\n");
        }
        if !no_sign {
            if let Some(signing) = &bundle_config.signing {
                if let Some(profiles) = &signing.profiles {
                    if profiles
                        .get(profile)
                        .and_then(|p| p.android.as_ref())
                        .is_some()
                    {
                        gradle_glue
                            .push_str("            signingConfig = signingConfigs.release\n");
                    }
                }
            }
        }
        gradle_glue.push_str("        }\n    }\n");

        gradle_glue.push_str("}\n");

        fs::write(generated_dir.join("jffi-bundle.gradle"), gradle_glue)
            .context("Failed to write jffi-bundle.gradle")?;

        // Generate JNA/UniFFI ProGuard rules — applied automatically via build.gradle.kts.
        // This means projects never need to manually add JNA keep rules.
        let proguard_rules = r#"# Auto-generated by JFFI. Do not edit manually.
# Required for UniFFI + JNA on Android.

# JNA: The `peer` field in Pointer is only accessed from native JNI code
# (libjnidispatch.so), so R8 must not strip it.
-keep class com.sun.jna.** { *; }
-keepclassmembers class com.sun.jna.** { *; }
-keep interface com.sun.jna.** { *; }
-keep class * implements com.sun.jna.Structure { *; }
-keep class * implements com.sun.jna.Callback { *; }

# JNA's Native$AWT references desktop java.awt classes that don't exist on
# Android. Suppress the R8 missing-class error for these dead code paths.
-dontwarn java.awt.**
-dontwarn sun.awt.**
-dontwarn java.applet.**

# UniFFI generated bindings — keep all generated FFI glue classes.
-keep class uniffi.** { *; }
-keepclassmembers class uniffi.** { *; }
"#;
        fs::write(generated_dir.join("jffi-android-rules.pro"), proguard_rules)
            .context("Failed to write jffi-android-rules.pro")?;

        println!(
            "  {} Generated jffi-android-rules.pro (JNA/UniFFI ProGuard rules)",
            "✓".green()
        );
    }

    println!("  {} Ensuring Rust core is built...", "→".bright_blue());
    if !dry_run {
        let mut build_cmd = Command::new(std::env::current_exe()?);
        build_cmd.args(["build", "--platform", "android", "--release"]);

        let status = build_cmd.status().context("Failed to build Rust core")?;
        if !status.success() {
            anyhow::bail!("Rust build failed for Android");
        }
    }

    // Collect signing env vars (and prompt interactively if needed) ONCE before the format loop
    let mut gradle_envs: Vec<(String, String)> = Vec::new();
    if !no_sign {
        let profile_name = profile;
        if let Some(signing) = &bundle_config.signing {
            if let Some(profiles) = &signing.profiles {
                if let Some(prof) = profiles.get(profile_name) {
                    if let Some(android_prof) = &prof.android {
                        if let Some(store_env) = &android_prof.store_password_env {
                            if let Ok(pwd) = std::env::var(store_env) {
                                gradle_envs.push(("JFFI_ANDROID_STORE_PASSWORD".to_string(), pwd));
                            } else if std::io::stdin().is_terminal() {
                                println!("  {} Env var {} not set. Keystore password is required for signing.", "🔑".cyan(), store_env);
                                let pwd = dialoguer::Password::new()
                                    .with_prompt(format!(
                                        "Enter Android Keystore Password ({})",
                                        store_env
                                    ))
                                    .interact()
                                    .context("Failed to read keystore password")?;
                                gradle_envs.push(("JFFI_ANDROID_STORE_PASSWORD".to_string(), pwd));
                            } else {
                                anyhow::bail!(
                                    "Required Android signing environment variable {} is not set",
                                    store_env
                                );
                            }
                        }
                        if let Some(key_env) = &android_prof.key_password_env {
                            if let Ok(pwd) = std::env::var(key_env) {
                                gradle_envs.push(("JFFI_ANDROID_KEY_PASSWORD".to_string(), pwd));
                            } else if std::io::stdin().is_terminal() {
                                println!("  {} Env var {} not set. Key password is required for signing.", "🔑".cyan(), key_env);
                                let pwd = dialoguer::Password::new()
                                    .with_prompt(format!(
                                        "Enter Android Key Password ({})",
                                        key_env
                                    ))
                                    .interact()
                                    .context("Failed to read key password")?;
                                gradle_envs.push(("JFFI_ANDROID_KEY_PASSWORD".to_string(), pwd));
                            } else {
                                anyhow::bail!(
                                    "Required Android signing environment variable {} is not set",
                                    key_env
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    for fmt in formats {
        let gradle_task = match fmt.as_str() {
            "aab" => "bundleRelease",
            "apk" => "assembleRelease",
            _ => anyhow::bail!("Unsupported Android format: {}", fmt),
        };

        let gradle_args = vec![gradle_task.to_string()];
        let gradle_envs = gradle_envs.clone();

        println!("  {} Running gradlew {}...", "→".bright_blue(), gradle_task);

        if !dry_run {
            let android_dir = Path::new("platforms/android");
            if !android_dir.exists() {
                anyhow::bail!(
                    "platforms/android directory not found. Did you create this project with jffi?"
                );
            }

            let gradlew = if cfg!(windows) {
                "gradlew.bat"
            } else {
                "./gradlew"
            };
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
                        anyhow::bail!("Failed to generate Gradle wrapper");
                    } else if !cfg!(windows) {
                        let status = Command::new("chmod")
                            .args(["+x", "gradlew"])
                            .current_dir(android_dir)
                            .status()
                            .context("Failed to mark gradlew executable")?;
                        if !status.success() {
                            anyhow::bail!("Failed to mark gradlew executable");
                        }
                    }
                } else {
                    println!(
                        "  {} Warning: gradle-wrapper.jar not found at {}",
                        "⚠".yellow(),
                        android_dir.join(wrapper_jar).display()
                    );
                }
            }

            if print_commands {
                let mut out = format!("{} {}", gradlew, gradle_args.join(" "));
                for (k, v) in &gradle_envs {
                    let kl = k.to_lowercase();
                    if kl.contains("password")
                        || kl.contains("key")
                        || kl.contains("token")
                        || kl.contains("secret")
                        || kl.contains("cert")
                        || kl.contains("pfx")
                    {
                        out = format!("{}='***' {}", k, out);
                    } else {
                        out = format!("{}='{}' {}", k, v, out);
                    }
                }
                println!("  $ {}", out.dimmed());
            }

            let mut cmd = Command::new(gradlew);
            cmd.args(&gradle_args).current_dir(android_dir);

            for (k, v) in gradle_envs {
                cmd.env(k, v);
            }

            let status = cmd
                .status()
                .context(format!("Failed to execute {}", gradlew))?;

            if !status.success() {
                anyhow::bail!("Gradle task {} failed", gradle_task);
            }
        }
    }

    if !dry_run && android_config.split_debug_symbols {
        let gradle_symbols = Path::new(
            "platforms/android/app/build/outputs/native-debug-symbols/release/native-debug-symbols.zip",
        );
        let output_dir = Path::new("target/bundle/android");
        fs::create_dir_all(output_dir)?;
        let output = output_dir.join("native-debug-symbols.zip");
        if gradle_symbols.exists() {
            fs::copy(gradle_symbols, &output).with_context(|| {
                format!(
                    "Failed to copy native debug symbols to {}",
                    output.display()
                )
            })?;
        } else {
            // Rust libraries are prebuilt before Gradle runs, so AGP may not emit
            // its own archive even when debugSymbolLevel is enabled. Package the
            // original, symbol-bearing ELF files in Play Console's ABI layout.
            create_android_native_symbols_archive(
                Path::new("platforms/android/app/src/main/jniLibs"),
                &output,
            )?;
        }
        println!(
            "  {} Native debug symbols: {}",
            "✓".green(),
            output.display()
        );
    }

    println!("{}", "  ✅ Android bundling complete!".green());
    Ok(())
}

fn create_android_native_symbols_archive(jni_libs: &Path, output: &Path) -> Result<()> {
    let file = fs::File::create(output)
        .with_context(|| format!("Failed to create {}", output.display()))?;
    let mut archive = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);
    let mut count = 0usize;

    for abi_entry in
        fs::read_dir(jni_libs).with_context(|| format!("Failed to read {}", jni_libs.display()))?
    {
        let abi_entry = abi_entry?;
        if !abi_entry.file_type()?.is_dir() {
            continue;
        }
        let abi = abi_entry.file_name().to_string_lossy().into_owned();
        for library_entry in fs::read_dir(abi_entry.path())? {
            let library_entry = library_entry?;
            let path = library_entry.path();
            let file_name = path.file_name().and_then(|name| name.to_str());
            if !library_entry.file_type()?.is_file()
                || path.extension().and_then(|ext| ext.to_str()) != Some("so")
                || !file_name
                    .is_some_and(|name| name.starts_with("lib") && name.ends_with("core.so"))
            {
                continue;
            }

            let name = file_name
                .context("Android native library path contains an unsupported file name")?;
            archive.start_file(format!("{abi}/{name}"), options)?;
            let mut library = fs::File::open(&path)?;
            let mut buffer = [0u8; 64 * 1024];
            loop {
                let read = library.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                archive.write_all(&buffer[..read])?;
            }
            count += 1;
        }
    }

    archive.finish()?;
    if count == 0 {
        anyhow::bail!(
            "Native debug symbols were requested, but no .so libraries were found in {}",
            jni_libs.display()
        );
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod android_symbols_tests {
    use super::*;

    #[test]
    fn archives_only_jffi_core_libraries_in_abi_layout() {
        let root = std::env::temp_dir().join(format!("jffi-symbols-{}", uuid::Uuid::new_v4()));
        let libs = root.join("jniLibs/arm64-v8a");
        fs::create_dir_all(&libs).unwrap();
        fs::write(libs.join("libsample_core.so"), b"symbols").unwrap();
        fs::write(libs.join("libdependency.so"), b"dependency").unwrap();
        let output = root.join("native-debug-symbols.zip");

        create_android_native_symbols_archive(&root.join("jniLibs"), &output).unwrap();

        let mut archive = zip::ZipArchive::new(fs::File::open(&output).unwrap()).unwrap();
        assert_eq!(archive.len(), 1);
        assert_eq!(
            archive.by_index(0).unwrap().name(),
            "arm64-v8a/libsample_core.so"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn accepts_only_implemented_bundle_formats() {
        assert!(validate_requested_format("windows", Some("msix")).is_ok());
        assert!(validate_requested_format("linux", Some("flatpak")).is_ok());
        assert!(validate_requested_format("android", Some("aab")).is_ok());
        assert!(validate_requested_format("macos", Some("pkg")).is_ok());
    }

    #[test]
    fn rejects_unimplemented_bundle_formats() {
        for (platform, format) in [
            ("windows", "msixbundle"),
            ("windows", "msi"),
            ("linux", "appimage"),
            ("linux", "deb"),
        ] {
            let error = validate_requested_format(platform, Some(format)).unwrap_err();
            assert!(error.to_string().contains("Unsupported bundle format"));
        }
    }

    #[test]
    fn preview_modes_never_sync_native_projects() {
        assert!(!should_sync_native_configs(true, false));
        assert!(!should_sync_native_configs(false, true));
        assert!(!should_sync_native_configs(true, true));
        assert!(should_sync_native_configs(false, false));
        assert!(!should_reconcile_bundle_prerequisites(true, false));
        assert!(!should_reconcile_bundle_prerequisites(false, true));
        assert!(should_reconcile_bundle_prerequisites(false, false));
    }

    #[test]
    fn rejects_unknown_and_all_platforms_even_without_a_format() {
        assert!(validate_requested_format("all", None).is_err());
        assert!(validate_requested_format("plan9", None).is_err());
    }

    #[test]
    fn bundles_only_enabled_platforms() {
        let config: Config = toml::from_str(
            r#"
schema_version = 1
[package]
name = "test"
version = "0.1.0"
[platforms]
enabled = ["web"]
"#,
        )
        .unwrap();
        assert!(validate_enabled_platform(&config, "web").is_ok());
        assert!(validate_enabled_platform(&config, "android").is_err());
    }

    #[test]
    fn ios_profile_mapping_must_include_main_bundle_identifier() {
        let config: Config = toml::from_str(
            r#"
schema_version = 1
[package]
name = "test"
version = "0.1.0"
[platforms]
enabled = ["ios"]
[platforms.ios]
bundle_id = "com.acme.app"
[bundle]
identifier = "com.acme.app"
[bundle.signing.profiles.release.apple]
team_id = "1234567890"
signing_certificate = "Apple Distribution"
provisioning_profiles = { "com.acme.app.ShareExtension" = "Share Profile" }
"#,
        )
        .unwrap();
        let error = validate_bundle_config(&config, "ios", "release", false).unwrap_err();
        assert!(error.to_string().contains("main iOS bundle identifier"));
    }

    #[test]
    fn android_validation_reports_configured_environment_variable() {
        let root = std::env::temp_dir().join(format!("jffi-signing-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let keystore = root.join("release.jks");
        fs::write(&keystore, b"test").unwrap();
        let input = format!(
            r#"
schema_version = 1
[package]
name = "test"
version = "0.1.0"
[platforms]
enabled = ["android"]
[platforms.android]
package = "com.acme.app"
min_sdk = 26
[bundle.android]
min_sdk = 26
build_type = "release"
[bundle.signing.profiles.release.android]
keystore_path = "{}"
key_alias = "release"
store_password_env = "JFFI_TEST_CUSTOM_STORE_PASSWORD"
key_password_env = "JFFI_TEST_CUSTOM_KEY_PASSWORD"
"#,
            keystore.to_string_lossy().replace('\\', "\\\\")
        );
        let config: Config = toml::from_str(&input).unwrap();
        let error = validate_bundle_config(&config, "android", "release", false).unwrap_err();
        assert!(error
            .to_string()
            .contains("JFFI_TEST_CUSTOM_STORE_PASSWORD"));
        fs::remove_dir_all(root).unwrap();
    }
}

fn bundle_macos(
    config: &Config,
    format: Option<&str>,
    profile: &str,
    no_sign: bool,
    notarize: bool,
    dry_run: bool,
    print_commands: bool,
) -> Result<()> {
    use crate::platform::{Platform, XcodeProject};

    let bundle_config = config.bundle.clone().unwrap_or_default();
    let macos_config = bundle_config
        .macos
        .unwrap_or_else(|| crate::config::BundleMacosConfig {
            formats: vec!["app".to_string(), "dmg".to_string()],
            targets: vec![
                "aarch64-apple-darwin".to_string(),
                "x86_64-apple-darwin".to_string(),
            ],
            minimum_system_version: "13.0".to_string(),
            category: None,
            signing_identity: Some("Developer ID Application".to_string()),
            hardened_runtime: true,
            notarize: true,
            staple: true,
            icon: None,
            provisioning_profile: None,
        });

    let mut formats = if let Some(f) = format {
        vec![f.to_string()]
    } else {
        macos_config.formats.clone()
    };

    println!("  {} Finding macOS Xcode project...", "→".bright_blue());
    let project = if !dry_run {
        XcodeProject::find(Platform::Macos)?
    } else {
        XcodeProject {
            project_path: PathBuf::from("platforms/macos/App.xcodeproj"),
            scheme: "App".to_string(),
        }
    };

    let mut method_val = "developer-id".to_string();
    let mut team_id_val = String::new();
    let mut signing_cert_val = None;
    let mut installer_cert_val = None;
    let mut apple_id_val = None;
    let mut apple_id_env_val = None;
    let mut app_specific_password_env_val = None;
    let mut api_key_path_val = None;
    let mut api_key_id_val = None;
    let mut api_key_issuer_id_val = None;
    let mut notarize_override = None;
    let mut formats_override = None;
    let mut provisioning_profile_override = None;

    if no_sign {
        method_val = "mac-application".to_string();
    } else {
        if let Some(signing) = &bundle_config.signing {
            if let Some(profiles) = &signing.profiles {
                if let Some(prof) = profiles.get(profile) {
                    if let Some(apple) = &prof.apple {
                        if let Some(m) = &apple.method {
                            method_val = m.clone();
                        }
                        if let Some(t) = &apple.team_id {
                            team_id_val = t.clone();
                        }
                        if let Some(c) = &apple.signing_certificate {
                            signing_cert_val = Some(c.clone());
                        }
                        if let Some(ic) = &apple.installer_signing_certificate {
                            installer_cert_val = Some(ic.clone());
                        }
                        if let Some(c) = &apple.apple_id {
                            apple_id_val = Some(c.clone());
                        }
                        if let Some(c) = &apple.apple_id_env {
                            apple_id_env_val = Some(c.clone());
                        }
                        if let Some(c) = &apple.app_specific_password_env {
                            app_specific_password_env_val = Some(c.clone());
                        }
                        if let Some(c) = &apple.api_key_path {
                            api_key_path_val = Some(c.clone());
                        }
                        if let Some(c) = &apple.api_key_id {
                            api_key_id_val = Some(c.clone());
                        }
                        if let Some(c) = &apple.api_key_issuer_id {
                            api_key_issuer_id_val = Some(c.clone());
                        }
                        if let Some(n) = &apple.notarize {
                            notarize_override = Some(*n);
                        }
                        if let Some(f) = &apple.formats {
                            formats_override = Some(f.clone());
                        }
                        if let Some(p) = &apple.provisioning_profile {
                            provisioning_profile_override = Some(p.clone());
                        }
                    }
                }
            }
        }
    }
    // Override formats if specified in the profile
    if let Some(f) = formats_override {
        if format.is_none() {
            formats = f;
        }
    }
    validate_format_list("macos", &formats)?;

    let generated_dir = PathBuf::from("target/jffi/generated/apple");
    let export_plist_path = generated_dir.join("ExportOptions.plist");

    if !dry_run {
        fs::create_dir_all(&generated_dir)?;

        let mut plist_content = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
             <plist version=\"1.0\">\n\
             <dict>\n\
                 <key>method</key>\n\
                 <string>{}</string>\n",
            method_val
        );

        if !team_id_val.is_empty() {
            plist_content.push_str(&format!(
                "    <key>teamID</key>\n\
                 <string>{}</string>\n",
                team_id_val
            ));
        }

        plist_content.push_str("    <key>manageAppVersionAndBuildNumber</key>\n    <false/>\n");
        plist_content.push_str("    <key>generateAppStoreInformation</key>\n    <false/>\n");
        if let Some(ref cert) = signing_cert_val {
            plist_content.push_str(&format!(
                "    <key>signingCertificate</key>\n    <string>{}</string>\n",
                cert
            ));
        }
        if let Some(icert) = installer_cert_val {
            plist_content.push_str(&format!(
                "    <key>installerSigningCertificate</key>\n    <string>{}</string>\n",
                icert
            ));
        }

        let prof_to_use = provisioning_profile_override
            .as_ref()
            .or(macos_config.provisioning_profile.as_ref());
        if let Some(prof) = prof_to_use {
            let identifier = config
                .bundle
                .as_ref()
                .and_then(|b| b.identifier.clone())
                .unwrap_or_else(|| config.platforms.ios.bundle_id.clone());
            plist_content.push_str(&format!(
                "    <key>provisioningProfiles</key>\n\
                 <dict>\n\
                     <key>{}</key>\n\
                     <string>{}</string>\n\
                 </dict>\n",
                identifier, prof
            ));
        }

        plist_content.push_str("</dict>\n</plist>\n");
        fs::write(&export_plist_path, plist_content)
            .context("Failed to write ExportOptions.plist")?;
    }

    println!("  {} Ensuring Rust core is built...", "→".bright_blue());
    if !dry_run {
        let mut build_cmd = Command::new(std::env::current_exe()?);
        build_cmd.args(["build", "--platform", "macos", "--release"]);

        let status = build_cmd.status().context("Failed to build Rust core")?;
        if !status.success() {
            anyhow::bail!("Rust build failed for macOS");
        }
    }

    let archive_path =
        PathBuf::from("target/bundle/macos").join(format!("{}.xcarchive", project.scheme));
    let export_path = PathBuf::from("target/bundle/macos/export");

    if !dry_run {
        fs::create_dir_all("target/bundle/macos")?;
    }

    println!("  {} Creating Xcode archive...", "→".bright_blue());
    if !dry_run {
        let mut archive_cmd = Command::new("xcodebuild");
        archive_cmd.args([
            "archive",
            "-project",
            project.project_path.to_str().unwrap(),
            "-scheme",
            &project.scheme,
            "-configuration",
            "Release",
            "-archivePath",
            archive_path.to_str().unwrap(),
            "-allowProvisioningUpdates",
        ]);

        // Universal binary handling
        if macos_config
            .targets
            .contains(&"aarch64-apple-darwin".to_string())
            && macos_config
                .targets
                .contains(&"x86_64-apple-darwin".to_string())
        {
            archive_cmd.arg("ARCHS=x86_64 arm64");
        } else if macos_config
            .targets
            .contains(&"aarch64-apple-darwin".to_string())
        {
            archive_cmd.arg("ARCHS=arm64");
        } else if macos_config
            .targets
            .contains(&"x86_64-apple-darwin".to_string())
        {
            archive_cmd.arg("ARCHS=x86_64");
        }

        if no_sign {
            archive_cmd.args([
                "CODE_SIGN_IDENTITY=",
                "CODE_SIGNING_REQUIRED=NO",
                "CODE_SIGNING_ALLOWED=NO",
            ]);
        } else {
            if !team_id_val.is_empty() {
                archive_cmd.arg(format!("DEVELOPMENT_TEAM={}", team_id_val));
            }
            if let Some(cert) = &signing_cert_val {
                archive_cmd.arg(format!("CODE_SIGN_IDENTITY={}", cert));
            }
            let prof_to_use = provisioning_profile_override
                .as_ref()
                .or(macos_config.provisioning_profile.as_ref());
            if let Some(prof) = prof_to_use {
                archive_cmd.arg(format!("PROVISIONING_PROFILE_SPECIFIER={}", prof));
                archive_cmd.arg("CODE_SIGN_STYLE=Manual");
            }
        }

        if print_commands {
            println!("  $ {:?}", archive_cmd);
        }

        let status = archive_cmd.status().context("Failed to create xcarchive")?;
        if !status.success() {
            anyhow::bail!("xcodebuild archive failed");
        }
    }

    println!("  {} Exporting archive...", "→".bright_blue());
    if !dry_run {
        fs::create_dir_all(&export_path)?;

        let mut export_cmd = Command::new("xcodebuild");
        export_cmd.args([
            "-exportArchive",
            "-archivePath",
            archive_path.to_str().unwrap(),
            "-exportPath",
            export_path.to_str().unwrap(),
            "-exportOptionsPlist",
            export_plist_path.to_str().unwrap(),
            "-allowProvisioningUpdates",
        ]);

        if print_commands {
            println!("  $ {:?}", export_cmd);
        }

        let status = export_cmd.status().context("Failed to export archive")?;
        if !status.success() {
            anyhow::bail!("xcodebuild exportArchive failed");
        }
    }

    if formats.contains(&"dmg".to_string()) {
        println!("  {} Creating DMG...", "→".bright_blue());
        if !dry_run {
            let app_path = export_path.join(format!("{}.app", project.scheme));
            let dmg_path = export_path.join(format!("{}.dmg", project.scheme));

            if app_path.exists() {
                // Remove existing DMG to avoid hdiutil error
                if dmg_path.exists() {
                    let _ = fs::remove_file(&dmg_path);
                }

                let mut hdiutil_cmd = Command::new("hdiutil");
                hdiutil_cmd.args([
                    "create",
                    "-volname",
                    &project.scheme,
                    "-srcfolder",
                    app_path.to_str().unwrap(),
                    "-ov",
                    "-format",
                    "UDZO",
                    dmg_path.to_str().unwrap(),
                ]);
                if print_commands {
                    println!("  $ {:?}", hdiutil_cmd);
                }
                let status = hdiutil_cmd.status().context("Failed to create DMG")?;

                if !status.success() {
                    anyhow::bail!("Failed to create DMG using hdiutil");
                }
                verify_nonempty_file(&dmg_path, "macOS DMG")?;
            } else {
                anyhow::bail!("Exported macOS app was not found at {}", app_path.display());
            }
        }
    }

    if !dry_run && formats.contains(&"app".to_string()) {
        let app_path = export_path.join(format!("{}.app", project.scheme));
        if !app_path.is_dir() {
            anyhow::bail!("Exported macOS app was not found at {}", app_path.display());
        }
    }

    if !dry_run && formats.contains(&"pkg".to_string()) {
        let pkg = fs::read_dir(&export_path)?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .find(|path| path.extension().and_then(|ext| ext.to_str()) == Some("pkg"))
            .context("xcodebuild exportArchive completed but did not create a macOS .pkg")?;
        verify_nonempty_file(&pkg, "macOS installer package")?;
    }

    let do_notarize = notarize_override.unwrap_or(notarize || macos_config.notarize);
    if do_notarize && !no_sign && !dry_run {
        println!("  {} Notarizing application...", "→".bright_blue());

        let path_to_notarize = if formats.contains(&"dmg".to_string()) {
            export_path.join(format!("{}.dmg", project.scheme))
        } else {
            // Must zip .app to notarize it if we don't have a DMG
            let app_path = export_path.join(format!("{}.app", project.scheme));
            let zip_path = export_path.join(format!("{}.zip", project.scheme));
            let status = Command::new("ditto")
                .args([
                    "-c",
                    "-k",
                    "--keepParent",
                    app_path.to_str().unwrap(),
                    zip_path.to_str().unwrap(),
                ])
                .status()
                .context("Failed to create notarization archive")?;
            if !status.success() {
                anyhow::bail!(
                    "Failed to create notarization archive: {}",
                    zip_path.display()
                );
            }
            zip_path
        };

        if path_to_notarize.exists() {
            let mut notary_cmd = Command::new("xcrun");
            notary_cmd.args(["notarytool", "submit", path_to_notarize.to_str().unwrap()]);

            let mut auth_provided = false;

            if let (Some(key_path), Some(key_id), Some(issuer)) =
                (&api_key_path_val, &api_key_id_val, &api_key_issuer_id_val)
            {
                notary_cmd.args(["--key", key_path, "--key-id", key_id, "--issuer", issuer]);
                auth_provided = true;
            } else {
                let pwd = app_specific_password_env_val
                    .as_ref()
                    .and_then(|e| std::env::var(e).ok());
                let aid = apple_id_val.clone().or_else(|| {
                    apple_id_env_val
                        .as_ref()
                        .and_then(|e| std::env::var(e).ok())
                });

                if let (Some(password), Some(apple_id)) = (pwd, aid) {
                    notary_cmd.args([
                        "--apple-id",
                        &apple_id,
                        "--password",
                        &password,
                        "--team-id",
                        &team_id_val,
                    ]);
                    auth_provided = true;
                }
            }

            if !auth_provided {
                let keychain_profile = std::env::var("JFFI_APPLE_NOTARY_PROFILE")
                    .unwrap_or_else(|_| "AC_PASSWORD".to_string());
                notary_cmd.args(["--keychain-profile", &keychain_profile]);
            }

            notary_cmd.arg("--wait");
            if print_commands {
                println!(
                    "  $ xcrun notarytool submit {} [auth_args] --wait",
                    path_to_notarize.to_str().unwrap()
                );
            }
            let status = notary_cmd
                .status()
                .context("Failed to submit to notarytool")?;

            if !status.success() {
                anyhow::bail!("Notarization failed or timed out");
            } else {
                println!("  {} Stapling ticket...", "→".bright_blue());
                if macos_config.staple {
                    let mut staple_cmd = Command::new("xcrun");
                    staple_cmd.args(["stapler", "staple", path_to_notarize.to_str().unwrap()]);
                    if print_commands {
                        println!("  $ {:?}", staple_cmd);
                    }
                    let status = staple_cmd.status().context("Failed to run stapler")?;
                    if !status.success() {
                        anyhow::bail!("Notarization succeeded, but stapling failed");
                    }
                }
                println!("  {} Notarization successful", "✓".green());
            }
        }
    }

    println!("{}", "  ✅ macOS bundling complete!".green());
    Ok(())
}

fn bundle_ios(
    config: &Config,
    profile: &str,
    no_sign: bool,
    dry_run: bool,
    print_commands: bool,
) -> Result<()> {
    use crate::platform::{Platform, XcodeProject};

    let bundle_config = config.bundle.clone().unwrap_or_default();
    let ios_config = bundle_config
        .ios
        .unwrap_or_else(|| crate::config::BundleIosConfig {
            format: "ipa".to_string(),
            destination: "generic/platform=iOS".to_string(),
            export_method: "app-store".to_string(),
            icon: None,
            provisioning_profile: None,
        });

    println!("  {} Finding iOS Xcode project...", "→".bright_blue());
    let project = if !dry_run {
        XcodeProject::find(Platform::Ios)?
    } else {
        XcodeProject {
            project_path: PathBuf::from("platforms/ios/App.xcodeproj"),
            scheme: "App".to_string(),
        }
    };

    let mut method_val = ios_config.export_method.clone();
    let mut team_id_val = String::new();
    let mut signing_cert_val = None;
    let mut installer_cert_val = None;
    let mut provisioning_profile_override = None;
    let mut provisioning_profiles = std::collections::BTreeMap::new();

    if !no_sign {
        if let Some(signing) = &bundle_config.signing {
            if let Some(profiles) = &signing.profiles {
                if let Some(prof) = profiles.get(profile) {
                    if let Some(apple) = &prof.apple {
                        if let Some(m) = &apple.method {
                            method_val = m.clone();
                        }
                        if let Some(t) = &apple.team_id {
                            team_id_val = t.clone();
                        }
                        if let Some(c) = &apple.signing_certificate {
                            signing_cert_val = Some(c.clone());
                        }
                        if let Some(ic) = &apple.installer_signing_certificate {
                            installer_cert_val = Some(ic.clone());
                        }
                        if let Some(p) = &apple.provisioning_profile {
                            provisioning_profile_override = Some(p.clone());
                        }
                        provisioning_profiles = apple.provisioning_profiles.clone();
                    }
                }
            }
        }
    } else {
        method_val = "development".to_string();
    }

    let generated_dir = PathBuf::from("target/jffi/generated/apple");
    let export_plist_path = generated_dir.join("ExportOptions_iOS.plist");

    if !dry_run {
        fs::create_dir_all(&generated_dir)?;

        let mut plist_content = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
             <plist version=\"1.0\">\n\
             <dict>\n\
                 <key>method</key>\n\
                 <string>{}</string>\n",
            method_val
        );

        if !team_id_val.is_empty() {
            plist_content.push_str(&format!(
                "    <key>teamID</key>\n\
                 <string>{}</string>\n",
                team_id_val
            ));
        }

        plist_content.push_str("    <key>manageAppVersionAndBuildNumber</key>\n    <false/>\n");
        plist_content.push_str("    <key>generateAppStoreInformation</key>\n    <false/>\n");
        if let Some(ref cert) = signing_cert_val {
            plist_content.push_str(&format!(
                "    <key>signingCertificate</key>\n    <string>{}</string>\n",
                cert
            ));
        }
        if let Some(icert) = installer_cert_val {
            plist_content.push_str(&format!(
                "    <key>installerSigningCertificate</key>\n    <string>{}</string>\n",
                icert
            ));
        }

        let prof_to_use = provisioning_profile_override
            .as_ref()
            .or(ios_config.provisioning_profile.as_ref());
        if !provisioning_profiles.is_empty() {
            plist_content.push_str("    <key>provisioningProfiles</key>\n    <dict>\n");
            for (identifier, profile) in &provisioning_profiles {
                plist_content.push_str(&format!(
                    "        <key>{}</key>\n        <string>{}</string>\n",
                    identifier, profile
                ));
            }
            plist_content.push_str("    </dict>\n");
        } else if let Some(prof) = prof_to_use {
            let identifier = config
                .bundle
                .as_ref()
                .and_then(|b| b.identifier.clone())
                .unwrap_or_else(|| config.package.name.clone());
            plist_content.push_str(&format!(
                "    <key>provisioningProfiles</key>\n\
                 <dict>\n\
                     <key>{}</key>\n\
                     <string>{}</string>\n\
                 </dict>\n",
                identifier, prof
            ));
        }

        plist_content.push_str("</dict>\n</plist>\n");
        fs::write(&export_plist_path, plist_content)
            .context("Failed to write ExportOptions.plist")?;
    }

    println!("  {} Ensuring Rust core is built...", "→".bright_blue());
    if !dry_run {
        let mut build_cmd = Command::new(std::env::current_exe()?);
        build_cmd.args(["build", "--platform", "ios", "--release"]);

        let status = build_cmd.status().context("Failed to build Rust core")?;
        if !status.success() {
            anyhow::bail!("Rust build failed for iOS");
        }
    }

    let archive_path =
        PathBuf::from("target/bundle/ios").join(format!("{}.xcarchive", project.scheme));
    let export_path = PathBuf::from("target/bundle/ios/export");

    if !dry_run {
        fs::create_dir_all("target/bundle/ios")?;
    }

    println!("  {} Creating Xcode archive...", "→".bright_blue());
    if !dry_run {
        let mut archive_cmd = Command::new("xcodebuild");
        archive_cmd.args([
            "archive",
            "-project",
            project.project_path.to_str().unwrap(),
            "-scheme",
            &project.scheme,
            "-configuration",
            "Release",
            "-destination",
            &ios_config.destination,
            "-archivePath",
            archive_path.to_str().unwrap(),
            "-allowProvisioningUpdates",
        ]);

        if no_sign {
            archive_cmd.args([
                "CODE_SIGN_IDENTITY=",
                "CODE_SIGNING_REQUIRED=NO",
                "CODE_SIGNING_ALLOWED=NO",
            ]);
        } else {
            if !team_id_val.is_empty() {
                archive_cmd.arg(format!("DEVELOPMENT_TEAM={}", team_id_val));
            }
            if let Some(cert) = &signing_cert_val {
                archive_cmd.arg(format!("CODE_SIGN_IDENTITY={}", cert));
            }
            let prof_to_use = provisioning_profile_override
                .as_ref()
                .or(ios_config.provisioning_profile.as_ref());
            if provisioning_profiles.is_empty() {
                if let Some(prof) = prof_to_use {
                    archive_cmd.arg(format!("PROVISIONING_PROFILE_SPECIFIER={}", prof));
                    archive_cmd.arg("CODE_SIGN_STYLE=Manual");
                }
            }
        }

        let status = archive_cmd.status().context("Failed to create xcarchive")?;
        if !status.success() {
            anyhow::bail!("xcodebuild archive failed");
        }
    }

    println!("  {} Exporting IPA...", "→".bright_blue());
    if !dry_run {
        fs::create_dir_all(&export_path)?;

        let mut export_cmd = Command::new("xcodebuild");
        export_cmd.args([
            "-exportArchive",
            "-archivePath",
            archive_path.to_str().unwrap(),
            "-exportPath",
            export_path.to_str().unwrap(),
            "-exportOptionsPlist",
            export_plist_path.to_str().unwrap(),
        ]);

        if print_commands {
            println!("  $ {:?}", export_cmd);
        }
        let status = export_cmd.status().context("Failed to export IPA")?;
        if !status.success() {
            anyhow::bail!("xcodebuild exportArchive failed");
        }
    }

    println!("{}", "  ✅ iOS bundling complete!".green());
    Ok(())
}

fn find_manifest_dir(dir: &Path) -> Option<PathBuf> {
    if dir.join("AppxManifest.xml").exists() {
        return Some(dir.to_path_buf());
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            if entry.path().is_dir() {
                if let Some(found) = find_manifest_dir(&entry.path()) {
                    return Some(found);
                }
            }
        }
    }
    None
}

fn find_sdk_tool(tool_name: &str) -> Option<String> {
    // Check if tool is in PATH
    let check_arg = if tool_name == "signtool" {
        "sign"
    } else {
        "/?"
    };
    if Command::new(tool_name).arg(check_arg).output().is_ok() {
        return Some(tool_name.to_string());
    }

    // On Windows, check standard SDK paths
    if cfg!(windows) {
        let sdk_root = Path::new("C:\\Program Files (x86)\\Windows Kits\\10\\bin");
        if sdk_root.exists() {
            if let Ok(entries) = fs::read_dir(sdk_root) {
                let mut versions = Vec::new();
                for entry in entries.filter_map(|e| e.ok()) {
                    if entry.path().is_dir() {
                        versions.push(entry.path());
                    }
                }
                // Sort descending to get the latest SDK
                versions.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

                for v in versions {
                    let tool_path = v.join("x64").join(format!("{}.exe", tool_name));
                    if tool_path.exists() {
                        return Some(tool_path.to_string_lossy().into_owned());
                    }
                }
            }
        }
    }
    None
}

fn bundle_windows(
    config: &Config,
    format: Option<&str>,
    profile: &str,
    no_sign: bool,
    dry_run: bool,
    print_commands: bool,
) -> Result<()> {
    let bundle_config = config.bundle.clone().unwrap_or_default();
    let windows_config =
        bundle_config
            .windows
            .unwrap_or_else(|| crate::config::BundleWindowsConfig {
                formats: vec!["msix".to_string()],
                identity_name: Some("Jitpomi.JffiApp".to_string()),
                publisher_id: Some("CN=Example".to_string()),
                targets: vec![
                    "x86_64-pc-windows-msvc".to_string(),
                    "aarch64-pc-windows-msvc".to_string(),
                ],
                icon: None,
            });

    let formats = if let Some(f) = format {
        vec![f.to_string()]
    } else {
        windows_config.formats.clone()
    };
    validate_format_list("windows", &formats)?;

    println!("  {} Ensuring Rust core is built...", "→".bright_blue());
    if !dry_run {
        let mut build_cmd = Command::new(std::env::current_exe()?);
        build_cmd.args(["build", "--platform", "windows", "--release"]);

        let status = build_cmd.status().context("Failed to build Rust core")?;
        if !status.success() {
            anyhow::bail!("Rust build failed for Windows");
        }
    }

    let export_path = PathBuf::from("target/bundle/windows");
    if !dry_run {
        fs::create_dir_all(&export_path)?;
    }

    // We expect the build system to have created platforms/windows/bin/x64 and ARM64
    let arch_dirs = vec![
        ("x86_64-pc-windows-msvc", "x64"),
        ("aarch64-pc-windows-msvc", "ARM64"),
    ];

    let mut generated_packages = Vec::new();

    for (target, arch_dir) in arch_dirs {
        if !windows_config.targets.contains(&target.to_string()) {
            continue;
        }

        let package_path = export_path.join(format!("App_{}.msix", arch_dir));

        println!("  {} Packaging {}...", "→".bright_blue(), target);
        if !dry_run {
            let bin_dir = Path::new("platforms/windows/bin").join(arch_dir);
            if bin_dir.exists() {
                let package_source = find_manifest_dir(&bin_dir).unwrap_or_else(|| bin_dir.clone());
                let makeappx_path = std::env::var("JFFI_MAKEAPPX")
                    .ok()
                    .or_else(|| find_sdk_tool("makeappx"))
                    .unwrap_or_else(|| "MakeAppx".to_string());
                let mut makeappx_cmd = Command::new(&makeappx_path);
                makeappx_cmd.args([
                    "pack",
                    "/d",
                    package_source.to_str().unwrap(),
                    "/p",
                    package_path.to_str().unwrap(),
                    "/o",
                ]);
                if print_commands {
                    println!("  $ {:?}", makeappx_cmd);
                }
                let status = makeappx_cmd.status().context("Failed to run MakeAppx pack");

                match status {
                    Ok(s) if s.success() && package_path.is_file() => {
                        verify_nonempty_file(&package_path, "Windows MSIX package")?;
                        generated_packages.push(package_path.clone())
                    }
                    Ok(_) => anyhow::bail!("MakeAppx did not create {}", package_path.display()),
                    Err(error) => {
                        return Err(error)
                            .context("MakeAppx pack failed; try setting JFFI_MAKEAPPX")
                    }
                }
            } else {
                anyhow::bail!(
                    "Windows package input directory {} not found",
                    bin_dir.display()
                );
            }
        }
    }

    if !dry_run && generated_packages.is_empty() {
        anyhow::bail!("No Windows MSIX packages were generated");
    }

    if !no_sign && !generated_packages.is_empty() {
        println!("  {} Signing MSIX packages...", "→".bright_blue());
        if !dry_run {
            let mut thumbprint = String::new();
            if let Some(signing) = &bundle_config.signing {
                if let Some(profiles) = &signing.profiles {
                    if let Some(prof) = profiles.get(profile) {
                        if let Some(win) = &prof.windows {
                            if let Some(cert) = &win.certificate_thumbprint {
                                thumbprint = cert.clone();
                            }
                        }
                    }
                }
            }

            if !thumbprint.is_empty() {
                let signtool_path = std::env::var("JFFI_SIGNTOOL")
                    .ok()
                    .or_else(|| find_sdk_tool("signtool"))
                    .unwrap_or_else(|| "signtool".to_string());
                for pkg in &generated_packages {
                    let mut sign_cmd = Command::new(&signtool_path);
                    sign_cmd.args([
                        "sign",
                        "/fd",
                        "SHA256",
                        "/a",
                        "/sha1",
                        &thumbprint,
                        pkg.to_str().unwrap(),
                    ]);
                    if print_commands {
                        println!(
                            "  $ {} sign /fd SHA256 /a /sha1 *** {:?}",
                            signtool_path,
                            pkg.as_os_str()
                        );
                    }
                    let status = sign_cmd.status().context("Failed to run signtool")?;
                    if !status.success() {
                        anyhow::bail!("Failed to sign {}", pkg.display());
                    }
                }
            } else {
                anyhow::bail!(
                    "No Windows certificate thumbprint in signing profile '{}'",
                    profile
                );
            }
        }
    }

    println!("{}", "  ✅ Windows bundling complete!".green());
    Ok(())
}

fn bundle_linux(
    config: &Config,
    format: Option<&str>,
    _profile: &str,
    dry_run: bool,
    print_commands: bool,
) -> Result<()> {
    let bundle_config = config.bundle.clone().unwrap_or_default();
    let linux_config = bundle_config
        .linux
        .unwrap_or_else(|| crate::config::BundleLinuxConfig {
            formats: vec!["flatpak".to_string()],
            app_id: Some("org.jffi.App".to_string()),
            runtime: "org.gnome.Platform".to_string(),
            runtime_version: "48".to_string(),
            sdk: "org.gnome.Sdk".to_string(),
            icon: None,
        });

    let formats = if let Some(f) = format {
        vec![f.to_string()]
    } else {
        linux_config.formats.clone()
    };
    validate_format_list("linux", &formats)?;

    let app_id = linux_config
        .app_id
        .unwrap_or_else(|| "org.jffi.App".to_string());

    println!("  {} Ensuring Rust core is built...", "→".bright_blue());
    if !dry_run {
        let mut build_cmd = Command::new(std::env::current_exe()?);
        build_cmd.args(["build", "--platform", "linux", "--release"]);

        let status = build_cmd.status().context("Failed to build Rust core")?;
        if !status.success() {
            anyhow::bail!("Rust build failed for Linux");
        }
    }

    let export_path = PathBuf::from("target/bundle/linux");
    if !dry_run {
        fs::create_dir_all(&export_path)?;
    }

    if formats.contains(&"flatpak".to_string()) {
        if !dry_run {
            crate::setup::ensure_flatpak_runtime(
                &linux_config.runtime,
                &linux_config.sdk,
                &linux_config.runtime_version,
            )?;
        }
        println!("  {} Generating Flatpak manifest...", "→".bright_blue());

        let manifest_path =
            PathBuf::from("target/jffi/generated/linux").join(format!("{}.json", app_id));
        if !dry_run {
            fs::create_dir_all("target/jffi/generated/linux")?;

            let manifest = format!(
                r#"{{
  "app-id": "{}",
  "runtime": "{}",
  "runtime-version": "{}",
  "sdk": "{}",
  "command": "jffi-app",
  "finish-args": [
    "--share=network",
    "--share=ipc",
    "--socket=fallback-x11",
    "--socket=wayland"
  ],
  "modules": [
    {{
      "name": "jffi-app",
      "buildsystem": "simple",
      "build-commands": [
        "install -D jffi-app /app/bin/jffi-app"
      ],
      "sources": [
        {{
          "type": "dir",
          "path": "../../../../platforms/linux"
        }}
      ]
    }}
  ]
}}"#,
                app_id, linux_config.runtime, linux_config.runtime_version, linux_config.sdk
            );

            fs::write(&manifest_path, manifest)?;
        }

        println!("  {} Running flatpak-builder...", "→".bright_blue());
        if !dry_run {
            let build_dir = export_path.join(format!("{}-build", app_id));
            let repo_dir = export_path.join("repo");

            let mut flatpak_builder_cmd = Command::new("flatpak-builder");
            flatpak_builder_cmd.args([
                "--repo",
                repo_dir.to_str().unwrap(),
                "--force-clean",
                build_dir.to_str().unwrap(),
                manifest_path.to_str().unwrap(),
            ]);
            if print_commands {
                println!("  $ {:?}", flatpak_builder_cmd);
            }
            let status = flatpak_builder_cmd
                .status()
                .context("Failed to run flatpak-builder")?;

            if status.success() {
                println!("  {} Exporting flatpak bundle...", "→".bright_blue());
                let bundle_file = export_path.join(format!("{}.flatpak", app_id));
                let status = Command::new("flatpak")
                    .args([
                        "build-bundle",
                        repo_dir.to_str().unwrap(),
                        bundle_file.to_str().unwrap(),
                        &app_id,
                    ])
                    .status()
                    .context("Failed to run flatpak build-bundle")?;
                if !status.success() {
                    anyhow::bail!("Flatpak bundle export failed: {}", bundle_file.display());
                }
                verify_nonempty_file(&bundle_file, "Flatpak bundle")?;
            } else {
                anyhow::bail!("flatpak-builder failed");
            }
        }
    }

    println!("{}", "  ✅ Linux bundling complete!".green());
    Ok(())
}

fn bundle_web(config: &Config, dry_run: bool, print_commands: bool) -> Result<()> {
    let bundle_config = config.bundle.clone().unwrap_or_default();
    let web_config = bundle_config
        .web
        .unwrap_or_else(|| crate::config::BundleWebConfig {
            dist_dir: "platforms/web/dist".to_string(),
            build_command: "npm run build".to_string(),
            wasm_opt: true,
            base_path: "/".to_string(),
        });

    println!(
        "  {} Ensuring Rust core is built for WebAssembly...",
        "→".bright_blue()
    );
    if !dry_run {
        let mut build_cmd = Command::new(std::env::current_exe()?);
        build_cmd.args(["build", "--platform", "web", "--release"]);

        let status = build_cmd.status().context("Failed to build Rust core")?;
        if !status.success() {
            anyhow::bail!("Rust build failed for Web");
        }
    }

    println!("  {} Running web bundler...", "→".bright_blue());
    if !dry_run {
        let web_dir = Path::new("platforms/web");
        if !web_dir.exists() {
            anyhow::bail!("platforms/web directory not found.");
        }

        let mut build_cmd_str = web_config.build_command.clone();
        if build_cmd_str == "npm run build" {
            // Auto detect
            if let Ok(pkg_json) = fs::read_to_string(web_dir.join("package.json")) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&pkg_json) {
                    if let Some(pm) = v.get("packageManager").and_then(|v| v.as_str()) {
                        if pm.starts_with("yarn") {
                            build_cmd_str = "yarn build".to_string();
                        } else if pm.starts_with("pnpm") {
                            build_cmd_str = "pnpm run build".to_string();
                        } else {
                            build_cmd_str = "npm run build".to_string();
                        }
                    }
                }
            }
            if build_cmd_str == "npm run build" {
                if web_dir.join("pnpm-lock.yaml").exists() {
                    build_cmd_str = "pnpm run build".to_string();
                } else if web_dir.join("yarn.lock").exists() {
                    build_cmd_str = "yarn build".to_string();
                }
            }
        }

        let cmd_parts = shell_words::split(&build_cmd_str)
            .context("Invalid quoting in bundle.web.build_command")?;
        if cmd_parts.is_empty() {
            anyhow::bail!("Invalid build_command in jffi.toml");
        }

        if print_commands {
            println!("  $ {}", build_cmd_str.dimmed());
        }

        let status = Command::new(&cmd_parts[0])
            .args(&cmd_parts[1..])
            .env("JFFI_BASE_PATH", &web_config.base_path)
            .env("BASE_PATH", &web_config.base_path)
            .current_dir(web_dir)
            .status()
            .context("Failed to run web build command");

        match status {
            Ok(s) if s.success() => {}
            _ => anyhow::bail!("Web build command failed"),
        }
    }

    let export_path = PathBuf::from("target/bundle/web");
    if !dry_run {
        fs::create_dir_all(&export_path)?;

        println!("  {} Copying distribution artifacts...", "→".bright_blue());
        let source_dir = Path::new(&web_config.dist_dir);
        if !source_dir.is_dir() {
            anyhow::bail!(
                "Web build completed but dist directory was not created: {}",
                source_dir.display()
            );
        }
        let copied = copy_directory_contents(source_dir, &export_path)?;
        if copied == 0 {
            anyhow::bail!("Web dist directory is empty: {}", source_dir.display());
        }
        if web_config.wasm_opt {
            optimize_wasm_files(&export_path)?;
        }
    }

    println!("{}", "  ✅ Web bundling complete!".green());
    Ok(())
}

pub fn validate_bundle_config(
    config: &Config,
    platform: &str,
    profile: &str,
    no_sign: bool,
) -> Result<()> {
    println!(
        "  {} Validating configuration for store-readiness...",
        "→".bright_blue()
    );

    if let Some(bundle) = &config.bundle {
        if let Some(android) = &bundle.android {
            if android.min_sdk != config.platforms.android.min_sdk {
                anyhow::bail!(
                    "Conflicting Android min SDK values: platforms.android.min_sdk={} but bundle.android.min_sdk={}",
                    config.platforms.android.min_sdk,
                    android.min_sdk
                );
            }
            if android.build_type != "release" {
                anyhow::bail!("bundle.android.build_type must be `release` for store bundles");
            }
        }
        if let Some(macos) = &bundle.macos {
            if macos.minimum_system_version != config.platforms.macos.deployment_target {
                anyhow::bail!(
                    "Conflicting macOS deployment targets: platforms.macos.deployment_target={} but bundle.macos.minimum_system_version={}",
                    config.platforms.macos.deployment_target,
                    macos.minimum_system_version
                );
            }
        }
        let configured_formats = match platform {
            "android" => bundle
                .android
                .as_ref()
                .map(|value| value.formats.as_slice()),
            "macos" => bundle
                .signing
                .as_ref()
                .and_then(|signing| signing.profiles.as_ref())
                .and_then(|profiles| profiles.get(profile))
                .and_then(|profile| profile.apple.as_ref())
                .and_then(|apple| apple.formats.as_deref())
                .or_else(|| bundle.macos.as_ref().map(|value| value.formats.as_slice())),
            "windows" => bundle
                .windows
                .as_ref()
                .map(|value| value.formats.as_slice()),
            "linux" => bundle.linux.as_ref().map(|value| value.formats.as_slice()),
            _ => None,
        };
        if let Some(formats) = configured_formats {
            validate_format_list(platform, formats)?;
        }
    }

    let check_placeholder = |value: &str, field_name: &str| -> Result<()> {
        if value.contains("com.example") || value.contains("example.app") {
            anyhow::bail!(
                "Store Rejection Risk: '{}' has value '{}'. You must customize this placeholder identifier in 'jffi.toml' before bundling for a store release.",
                field_name.yellow(),
                value.red()
            );
        }
        Ok(())
    };

    if platform == "android" {
        check_placeholder(
            &config.platforms.android.package,
            "platforms.android.package",
        )?;
    } else if platform == "ios" {
        check_placeholder(&config.platforms.ios.bundle_id, "platforms.ios.bundle_id")?;
    } else if platform == "macos" {
        let bundle_id = config
            .bundle
            .as_ref()
            .and_then(|b| b.identifier.clone())
            .unwrap_or_else(|| config.platforms.ios.bundle_id.clone());
        check_placeholder(&bundle_id, "bundle.identifier / platforms.ios.bundle_id")?;
    }

    if let Some(bundle) = &config.bundle {
        if let Some(identifier) = &bundle.identifier {
            check_placeholder(identifier, "bundle.identifier")?;
        }

        if platform == "linux" {
            if let Some(linux_conf) = &bundle.linux {
                if let Some(app_id) = &linux_conf.app_id {
                    if app_id == "org.jffi.App" {
                        anyhow::bail!("Store Rejection Risk: 'bundle.linux.app_id' is set to default '{}'. Change it in 'jffi.toml'.", app_id.red());
                    }
                }
            }
        }

        if platform == "windows" {
            if let Some(win_conf) = &bundle.windows {
                if let Some(identity_name) = &win_conf.identity_name {
                    if identity_name == "Jitpomi.JffiApp" {
                        anyhow::bail!("Store Rejection Risk: 'bundle.windows.identity_name' is set to default '{}'. Change it in 'jffi.toml'.", identity_name.red());
                    }
                }
                if let Some(publisher_id) = &win_conf.publisher_id {
                    if publisher_id == "CN=Example" {
                        anyhow::bail!("Store Rejection Risk: 'bundle.windows.publisher_id' is set to default '{}'. Change it in 'jffi.toml'.", publisher_id.red());
                    }
                }
            }
        }
    }

    if !no_sign {
        match platform {
            "android" => {
                if let Some(bundle_conf) = &config.bundle {
                    if let Some(signing) = &bundle_conf.signing {
                        if let Some(profiles) = &signing.profiles {
                            let active_profile = profile;
                            if let Some(profile) = profiles.get(active_profile) {
                                if let Some(android_prof) = &profile.android {
                                    if let Some(keystore_path) = &android_prof.keystore_path {
                                        if keystore_path.contains("placeholder")
                                            || keystore_path.is_empty()
                                        {
                                            anyhow::bail!("Store Validation Error: Android keystore_path is placeholder or empty. Setup a release keystore in 'jffi.toml'.");
                                        }

                                        let keystore_file = Path::new(keystore_path);
                                        if !keystore_file.exists() {
                                            anyhow::bail!(
                                                "Store Validation Error: Keystore file not found at '{}'. Please ensure the release keystore exists or is correctly mapped in 'jffi.toml'.",
                                                keystore_path.red()
                                            );
                                        }
                                    } else {
                                        anyhow::bail!("Store Validation Error: Android keystore_path is missing in 'signing.profiles.{}.android'.", active_profile);
                                    }

                                    if android_prof.key_alias.is_none()
                                        || android_prof
                                            .key_alias
                                            .as_ref()
                                            .map(|s| s.is_empty())
                                            .unwrap_or(true)
                                    {
                                        anyhow::bail!("Store Validation Error: Android key_alias is missing or empty in 'signing.profiles.{}.android'.", active_profile);
                                    }

                                    if android_prof.store_password_env.is_none()
                                        || android_prof.key_password_env.is_none()
                                    {
                                        anyhow::bail!("Store Validation Error: Android store_password_env or key_password_env is missing in 'signing.profiles.{}.android'.", active_profile);
                                    }
                                    for variable in [
                                        android_prof.store_password_env.as_deref().unwrap(),
                                        android_prof.key_password_env.as_deref().unwrap(),
                                    ] {
                                        if std::env::var_os(variable).is_none() {
                                            anyhow::bail!(
                                                "Store Validation Error: required Android signing environment variable '{}' is not set",
                                                variable
                                            );
                                        }
                                    }
                                } else {
                                    anyhow::bail!("Store Validation Error: Android signing profile 'android' is missing in 'signing.profiles{}'.", active_profile);
                                }
                            } else {
                                anyhow::bail!(
                                    "Store Validation Error: signing profile '{}' was not found",
                                    active_profile
                                );
                            }
                        } else {
                            anyhow::bail!(
                                "Store Validation Error: no signing profiles are configured"
                            );
                        }
                    } else {
                        anyhow::bail!("Store Validation Error: no signing configuration is present; use --no-sign for an intentionally unsigned bundle");
                    }
                } else {
                    anyhow::bail!("Store Validation Error: bundle configuration is missing");
                }
            }
            "ios" | "macos" => {
                let apple = config
                    .bundle
                    .as_ref()
                    .and_then(|bundle| bundle.signing.as_ref())
                    .and_then(|signing| signing.profiles.as_ref())
                    .and_then(|profiles| profiles.get(profile))
                    .and_then(|profile| profile.apple.as_ref())
                    .with_context(|| {
                        format!(
                            "Store Validation Error: Apple signing profile '{}' is missing",
                            profile
                        )
                    })?;
                let team_id = apple
                    .team_id
                    .as_deref()
                    .context("Store Validation Error: Apple developer team_id is missing")?;
                if team_id.len() != 10 || team_id.contains("placeholder") {
                    anyhow::bail!(
                        "Store Validation Error: Apple developer team_id '{}' must be a valid 10-character team ID",
                        team_id
                    );
                }
                let certificate = apple
                    .signing_certificate
                    .as_deref()
                    .context("Store Validation Error: Apple signing_certificate is missing")?;
                if certificate.trim().is_empty() {
                    anyhow::bail!("Store Validation Error: Apple signing_certificate is empty");
                }
                if platform == "ios" {
                    let singular_profile = apple.provisioning_profile.as_deref().or_else(|| {
                        config
                            .bundle
                            .as_ref()
                            .and_then(|bundle| bundle.ios.as_ref())
                            .and_then(|ios| ios.provisioning_profile.as_deref())
                    });
                    if apple.provisioning_profiles.is_empty()
                        && singular_profile.is_none_or(|value| value.trim().is_empty())
                    {
                        anyhow::bail!(
                            "Store Validation Error: iOS signing profile '{}' needs provisioning_profile or provisioning_profiles",
                            profile
                        );
                    }
                    for (identifier, provisioning_profile) in &apple.provisioning_profiles {
                        if identifier.trim().is_empty() || provisioning_profile.trim().is_empty() {
                            anyhow::bail!(
                                "Store Validation Error: iOS provisioning profile mappings cannot contain empty identifiers or profile names"
                            );
                        }
                    }
                    if !apple.provisioning_profiles.is_empty()
                        && !apple
                            .provisioning_profiles
                            .contains_key(&config.platforms.ios.bundle_id)
                    {
                        anyhow::bail!(
                            "Store Validation Error: provisioning_profiles does not map the main iOS bundle identifier '{}'",
                            config.platforms.ios.bundle_id
                        );
                    }
                }
            }
            "windows" => {
                let thumbprint = config
                    .bundle
                    .as_ref()
                    .and_then(|bundle| bundle.signing.as_ref())
                    .and_then(|signing| signing.profiles.as_ref())
                    .and_then(|profiles| profiles.get(profile))
                    .and_then(|profile| profile.windows.as_ref())
                    .and_then(|windows| windows.certificate_thumbprint.as_deref())
                    .with_context(|| format!("Store Validation Error: Windows signing profile '{}' with certificate_thumbprint is missing", profile))?;
                if thumbprint.trim().is_empty() {
                    anyhow::bail!(
                        "Store Validation Error: Windows certificate thumbprint is empty"
                    );
                }
            }
            _ => {}
        }
    }

    println!("  {} Configuration is store-ready!", "✓".green());
    Ok(())
}
