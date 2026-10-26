//! Exercise CLI orchestration without launching a UI or requiring simulator state.
#![cfg(target_os = "macos")]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

fn succeeds(output: Output) {
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn release_run_builds_and_launches_selected_simulator_without_debug_fallback() {
    let root = std::env::temp_dir().join(format!("jffi-run-regression-{}", uuid::Uuid::new_v4()));
    let project = root.join("project");
    let tools = root.join("tools");
    fs::create_dir_all(&tools).unwrap();
    succeeds(
        Command::new(env!("CARGO_BIN_EXE_jffi"))
            .args([
                "new",
                "regression",
                "--platforms",
                "ios,macos",
                "--template",
                "hello",
                "--no-build",
                "--path",
            ])
            .arg(&project)
            .output()
            .unwrap(),
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
  xcode-select) printf '%s\n' "$JFFI_TEST_DEVELOPER_DIR" ;;
  open)
    if [ "${JFFI_TEST_MISSING_SIMULATOR:-0}" = 1 ]; then exit 1; fi ;;
  rustup) printf '%s\n' aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios aarch64-apple-darwin x86_64-apple-darwin ;;
  uniffi-bindgen)
    if [ "${1:-}" = --version ]; then echo 'uniffi-bindgen 0.31.1'; fi ;;
  cargo)
    for target in aarch64-apple-ios-sim x86_64-apple-ios aarch64-apple-ios aarch64-apple-darwin x86_64-apple-darwin; do
      mkdir -p "target/$target/release"
      touch "target/$target/release/libregression_core.a" "target/$target/release/libregression_core.dylib"
    done ;;
  lipo)
    while [ "$#" -gt 0 ]; do
      if [ "$1" = -output ]; then shift; touch "$1"; break; fi
      shift
    done ;;
  xcodebuild)
    if [ "${JFFI_TEST_BUILD_FAILURE:-0}" = 1 ]; then
      for arg in "$@"; do
        if [ "$arg" = build ]; then
          echo "AppState.swift:12:14: error: value of type 'AppState' has no member 'count'"
          echo 'linker diagnostic from stderr' >&2
          exit 65
        fi
      done
    fi
    previous=''
    for arg in "$@"; do
      if [ "$previous" = -output ]; then mkdir -p "$arg"; fi
      previous=$arg
    done ;;
  xcrun)
    if [ "${2:-}" = list ]; then
      printf '%s\n' '{"devices":{"com.apple.CoreSimulator.SimRuntime.iOS-27-0":[{"name":"iPad","deviceTypeIdentifier":"com.apple.CoreSimulator.SimDeviceType.iPad-Pro","udid":"selected-pad","isAvailable":true,"state":"Booted"}]}}'
    fi ;;
esac
"#;
    for name in [
        "cargo",
        "rustup",
        "uniffi-bindgen",
        "xcodebuild",
        "xcrun",
        "lipo",
        "install_name_tool",
        "open",
        "xcode-select",
    ] {
        let file = tools.join(name);
        fs::write(&file, script).unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let log = root.join("commands.log");
    let path = std::env::join_paths(std::iter::once(tools.clone()).chain(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    )))
    .unwrap();
    for platform in ["ios", "macos", "macos-arm64"] {
        let mut run = Command::new(env!("CARGO_BIN_EXE_jffi"));
        run.current_dir(&project)
            .env("PATH", &path)
            .env("JFFI_TEST_LOG", &log)
            .env("JFFI_TEST_DEVELOPER_DIR", root.join("missing-developer"))
            .args(["run", "--platform", platform, "--release", "--no-setup"]);
        if platform == "ios" {
            run.args(["--device", "ipad"]);
        }
        succeeds(run.output().unwrap());
        if platform == "ios" {
            fs::write(&log, "").unwrap();
            let output = run
                .env("JFFI_TEST_MISSING_SIMULATOR", "1")
                .output()
                .unwrap();
            assert!(String::from_utf8_lossy(&output.stderr).contains("Continuing with simctl"));
            assert_eq!(
                String::from_utf8_lossy(&output.stderr)
                    .matches("Continuing with simctl")
                    .count(),
                1
            );
            succeeds(output);
            let fallback_commands = fs::read_to_string(&log).unwrap();
            assert!(fallback_commands.contains("xcrun|simctl install selected-pad"));
            assert!(fallback_commands
                .contains("xcrun|simctl launch selected-pad com.example.regression"));
        }
    }
    // New Xcode versions bundle Device Hub alongside the Developer directory.
    let contents = root.join("Xcode.app/Contents");
    let hub = contents.join("Applications/DeviceHub.app");
    fs::create_dir_all(&hub).unwrap();
    let hub_log = root.join("hub.log");
    let output = Command::new(env!("CARGO_BIN_EXE_jffi"))
        .current_dir(&project)
        .env("PATH", &path)
        .env("JFFI_TEST_LOG", &hub_log)
        .env("JFFI_TEST_DEVELOPER_DIR", contents.join("Developer"))
        .args(["run", "--platform", "ios", "--release", "--no-setup"])
        .output()
        .unwrap();
    assert!(!String::from_utf8_lossy(&output.stderr).contains("Could not open"));
    succeeds(output);
    let hub_commands = fs::read_to_string(&hub_log).unwrap();
    assert!(hub_commands.contains(&format!("open|{}", hub.display())));
    assert_eq!(
        hub_commands
            .lines()
            .filter(|line| line.starts_with("open|"))
            .count(),
        1
    );
    // Both default and verbose mode must surface diagnostics and stop before launch.
    for verbose in [false, true] {
        let failure_log = root.join(format!("failure-{verbose}.log"));
        let mut run = Command::new(env!("CARGO_BIN_EXE_jffi"));
        run.current_dir(&project)
            .env("PATH", &path)
            .env("JFFI_TEST_LOG", &failure_log)
            .env("JFFI_TEST_BUILD_FAILURE", "1")
            .env_remove("JFFI_VERBOSE")
            .args(["run", "--platform", "ios", "--release", "--no-setup"]);
        if verbose {
            run.env("JFFI_VERBOSE", "1");
        }
        let output = run.output().unwrap();
        assert!(!output.status.success());
        let diagnostics = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            diagnostics.contains("AppState.swift:12:14: error:"),
            "{diagnostics}"
        );
        assert!(
            diagnostics.contains("linker diagnostic from stderr"),
            "{diagnostics}"
        );
        assert!(diagnostics.contains("exit status: 65"), "{diagnostics}");
        let commands = fs::read_to_string(&failure_log).unwrap();
        assert!(!commands.contains("simctl install"), "{commands}");
        assert!(!commands.contains("simctl launch"), "{commands}");
    }
    let commands = fs::read_to_string(&log).unwrap();
    assert!(
        commands.lines().any(|line| line.starts_with("xcodebuild|")
            && line.contains("-configuration Release")
            && line.contains("id=selected-pad")
            && line.contains("target/jffi/xcode/ios")),
        "{commands}"
    );
    assert!(commands.contains("xcrun|simctl install selected-pad target/jffi/xcode/ios/Build/Products/Release-iphonesimulator/Regression.app"), "{commands}");
    assert!(
        commands.contains("xcrun|simctl launch selected-pad com.example.regression"),
        "{commands}"
    );
    assert!(
        commands.contains("open|target/jffi/xcode/macos/Build/Products/Release/Regression.app"),
        "{commands}"
    );
    assert!(!commands.contains("simctl install booted"));
    assert!(!commands.contains("/Debug"));
    assert!(Path::new(&project).exists());
    fs::remove_dir_all(root).unwrap();
}
