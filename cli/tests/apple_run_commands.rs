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
    previous=''
    for arg in "$@"; do
      if [ "$previous" = -output ]; then mkdir -p "$arg"; fi
      previous=$arg
    done ;;
  xcrun)
    if [ "${2:-}" = list ]; then
      printf '%s\n' '{"devices":{"com.apple.CoreSimulator.SimRuntime.iOS-27-0":[{"name":"iPad","udid":"selected-pad","isAvailable":true,"state":"Booted"}]}}'
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
            .args(["run", "--platform", platform, "--release", "--no-setup"]);
        if platform == "ios" {
            run.args(["--device-id", "selected-pad"]);
        }
        succeeds(run.output().unwrap());
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
