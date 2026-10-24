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
      devices) printf 'List of devices attached\nselected-phone\tdevice\n' ;;
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
    let sdk = root.join("custom-sdk");
    fs::create_dir_all(&sdk).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_jffi"))
        .current_dir(&project)
        .env("PATH", path)
        .env("ANDROID_HOME", &sdk)
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
    fs::remove_dir_all(root).unwrap();
}
