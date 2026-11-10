//! A connected Android phone must not depend on emulator tooling or a macOS SDK path.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

#[test]
fn physical_android_release_run_preserves_sdk_and_uses_selected_device() {
    let root =
        std::env::temp_dir().join(format!("jffi-android-regression-{}", uuid::Uuid::new_v4()));
    let project = root.join("project");
    let tools = root.join("tools");
    fs::create_dir_all(&tools).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_jffi"))
        .args([
            "new",
            "regression",
            "--platforms",
            "android",
            "--template",
            "hello",
            "--no-build",
            "--path",
        ])
        .arg(&project)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::write(
        project.join("Cargo.lock"),
        "[[package]]\nname = \"uniffi\"\nversion = \"0.31.1\"\n",
    )
    .unwrap();
    let script = r#"#!/bin/sh
set -eu
tool=${0##*/}
printf '%s|%s\n' "$tool" "$*" >> "$JFFI_TEST_LOG"
case "$tool" in
  rustup) printf '%s\n' aarch64-linux-android armv7-linux-androideabi x86_64-linux-android ;;
  uniffi-bindgen) if [ "${1:-}" = --version ]; then echo 'uniffi-bindgen 0.31.1'; fi ;;
  cargo)
    if [ "${1:-}" = ndk ] && [ "${2:-}" = --version ]; then echo 'cargo-ndk 4.0.0'; exit 0; fi
    for target in aarch64-linux-android armv7-linux-androideabi x86_64-linux-android; do
      mkdir -p "target/$target/release"
      touch "target/$target/release/libregression_core.so"
    done ;;
  adb)
    case "${1:-}" in
      --version) echo 'Android Debug Bridge' ;;
      devices)
        if [ "${JFFI_TEST_NAMED_EMULATOR:-0}" = 1 ]; then
          printf 'List of devices attached\nemulator-5554\tdevice\nemulator-5556\tdevice\n'
        else
          printf 'List of devices attached\nselected-phone\tdevice\n'
        fi ;;
      -s)
        if [ "${3:-}" = emu ]; then
          if [ "$2" = emulator-5556 ]; then echo Pixel_Tablet; else echo Pixel_Phone; fi
        elif [ "${3:-}" = shell ] && [ "${4:-}" = getprop ]; then echo 1; fi ;;
      *) if [ "${3:-}" = shell ] && [ "${4:-}" = getprop ]; then echo 1; fi ;;
    esac ;;
esac
"#;
    for tool in ["rustup", "cargo", "uniffi-bindgen", "adb"] {
        let file = tools.join(tool);
        fs::write(&file, script).unwrap();
        fs::set_permissions(file, fs::Permissions::from_mode(0o755)).unwrap();
    }
    // The actual build pipeline still strips only the packaged copy of the core library.
    let strip = root.join("ndk/toolchains/llvm/prebuilt/test-host/bin/llvm-strip");
    fs::create_dir_all(strip.parent().unwrap()).unwrap();
    fs::write(&strip, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&strip, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(
        project.join("platforms/android/gradlew"),
        r#"#!/bin/sh
set -eu
printf 'gradlew|%s|sdk=%s\n' "$*" "$ANDROID_HOME" >> "$JFFI_TEST_LOG"
mkdir -p app/build/outputs/apk/release
echo APK > app/build/outputs/apk/release/app-release.apk
"#,
    )
    .unwrap();
    let log = root.join("commands.log");
    let path = std::env::join_paths(std::iter::once(tools).chain(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    )))
    .unwrap();
    let home = root.join("home");
    let sdk = home.join("Library/Android/sdk");
    fs::create_dir_all(&sdk).unwrap();
    let config_path = project.join("jffi.toml");
    let config = fs::read_to_string(&config_path).unwrap();
    fs::write(
        &config_path,
        config.replace("com.example.regression", "com.jitpomi.regression"),
    )
    .unwrap();
    let doctor = Command::new(env!("CARGO_BIN_EXE_jffi"))
        .current_dir(&project)
        .env("PATH", &path)
        .env("HOME", &home)
        .env_remove("ANDROID_HOME")
        .env_remove("ANDROID_SDK_ROOT")
        .env("JFFI_TEST_LOG", &log)
        .args(["--no-setup", "doctor", "bundle", "--platform", "android"])
        .output()
        .unwrap();
    assert!(
        doctor.status.success(),
        "{}",
        String::from_utf8_lossy(&doctor.stderr)
    );
    assert!(String::from_utf8_lossy(&doctor.stdout).contains(&sdk.display().to_string()));
    let output = Command::new(env!("CARGO_BIN_EXE_jffi"))
        .current_dir(&project)
        .env("PATH", &path)
        .env("HOME", &home)
        .env_remove("ANDROID_HOME")
        .env_remove("ANDROID_SDK_ROOT")
        .env("ANDROID_NDK_HOME", root.join("ndk"))
        .env("JFFI_TEST_LOG", &log)
        .args([
            "run",
            "--platform",
            "android",
            "--release",
            "--device",
            "--device-id",
            "selected-phone",
            "--no-setup",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let commands = fs::read_to_string(&log).unwrap();
    assert!(commands.contains("assembleRelease|sdk="), "{commands}");
    assert!(
        commands.contains(&format!("sdk={}", sdk.display())),
        "{commands}"
    );
    assert!(commands.contains("adb|-s selected-phone install -r platforms/android/app/build/outputs/apk/release/app-release.apk"), "{commands}");
    assert!(
        commands.contains("adb|-s selected-phone shell am start"),
        "{commands}"
    );
    assert!(!commands.contains("assembleDebug"));
    assert!(!commands.contains("emulator|"));
    fs::write(&log, "").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_jffi"))
        .current_dir(&project)
        .env("PATH", &path)
        .env("ANDROID_HOME", &sdk)
        .env("ANDROID_NDK_HOME", root.join("ndk"))
        .env("JFFI_TEST_LOG", &log)
        .env("JFFI_TEST_NAMED_EMULATOR", "1")
        .args([
            "run",
            "--platform",
            "android",
            "--release",
            "--device",
            "Pixel_Tablet",
            "--no-setup",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let commands = fs::read_to_string(&log).unwrap();
    assert!(
        commands.contains("adb|-s emulator-5556 install -r"),
        "{commands}"
    );
    assert!(
        !commands.contains("adb|-s emulator-5554 install"),
        "{commands}"
    );
    // A fresh project can have the wrapper JAR but no wrapper scripts. JAVA_HOME
    // must win over an incompatible Java on PATH when bootstrapping those scripts.
    let wrapper_template = root.join("wrapper-template");
    fs::rename(project.join("platforms/android/gradlew"), &wrapper_template).unwrap();
    let java_home = root.join("selected jdk");
    fs::create_dir_all(java_home.join("bin")).unwrap();
    let home_java = java_home.join("bin/java");
    fs::write(
        &home_java,
        r#"#!/bin/sh
set -eu
printf 'home-java|%s\n' "$*" >> "$JFFI_TEST_LOG"
cp "$JFFI_TEST_WRAPPER_TEMPLATE" gradlew
"#,
    )
    .unwrap();
    fs::set_permissions(&home_java, fs::Permissions::from_mode(0o755)).unwrap();
    let path_java = root.join("tools/java");
    fs::write(
        &path_java,
        "#!/bin/sh\nprintf 'wrong-java\\n' >> \"$JFFI_TEST_LOG\"\nexit 97\n",
    )
    .unwrap();
    fs::set_permissions(&path_java, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(&log, "").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_jffi"))
        .current_dir(&project)
        .env("PATH", &path)
        .env("JAVA_HOME", &java_home)
        .env("ANDROID_HOME", &sdk)
        .env("ANDROID_NDK_HOME", root.join("ndk"))
        .env("JFFI_TEST_LOG", &log)
        .env("JFFI_TEST_WRAPPER_TEMPLATE", &wrapper_template)
        .args([
            "run",
            "--platform",
            "android",
            "--release",
            "--device",
            "--device-id",
            "selected-phone",
            "--no-setup",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let commands = fs::read_to_string(&log).unwrap();
    assert!(commands.contains("home-java|-classpath gradle/wrapper/gradle-wrapper.jar org.gradle.wrapper.GradleWrapperMain wrapper"), "{commands}");
    assert!(!commands.contains("wrong-java"), "{commands}");
    assert!(
        commands
            .lines()
            .any(|line| line.starts_with("gradlew|") && line.contains("assembleRelease|sdk=")),
        "{commands}"
    );
    fs::write(
        project.join("platforms/android/gradlew"),
        "#!/bin/sh\necho 'specific Gradle failure: SDK could not be opened' >&2\nexit 7\n",
    )
    .unwrap();
    let failed = Command::new(env!("CARGO_BIN_EXE_jffi"))
        .current_dir(&project)
        .env("PATH", &path)
        .env("HOME", &home)
        .env_remove("ANDROID_HOME")
        .env_remove("ANDROID_SDK_ROOT")
        .env("ANDROID_NDK_HOME", root.join("ndk"))
        .env("JFFI_TEST_LOG", &log)
        .args([
            "run",
            "--platform",
            "android",
            "--release",
            "--device-id",
            "selected-phone",
            "--no-setup",
        ])
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr)
        .contains("specific Gradle failure: SDK could not be opened"));
    fs::remove_dir_all(root).unwrap();
}
