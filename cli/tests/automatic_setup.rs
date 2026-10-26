//! Verify run's actual setup path without installing tools or accessing the network.
#![cfg(unix)]
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

#[test]
fn web_run_installs_missing_and_mismatched_tools_unless_disabled() {
    let root = std::env::temp_dir().join(format!("jffi-auto-setup-{}", uuid::Uuid::new_v4()));
    let project = root.join("project");
    let tools = root.join("tools");
    fs::create_dir_all(&tools).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_jffi"))
        .args([
            "new",
            "regression",
            "--platforms",
            "web",
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
    fs::write(project.join("Cargo.lock"), "[[package]]\nname = \"uniffi\"\nversion = \"0.31.1\"\n[[package]]\nname = \"wasm-bindgen\"\nversion = \"0.2.100\"\n").unwrap();
    let script = r#"#!/bin/sh
set -eu
tool=${0##*/}
printf '%s|%s\n' "$tool" "$*" >> "$JFFI_TEST_LOG"
case "$tool" in
 uniffi-bindgen) echo 'uniffi-bindgen 0.31.1' ;;
 rustup) echo wasm32-unknown-unknown ;;
 wasm-pack) if [ "$JFFI_TEST_CASE" = missing ]; then exit 1; fi; echo 'wasm-pack 0.13.1' ;;
 wasm-bindgen) echo 'wasm-bindgen 0.2.99' ;;
 cargo) echo 'intentional test stop at installation' >&2; exit 1 ;;
esac
"#;
    for tool in [
        "cargo",
        "uniffi-bindgen",
        "rustup",
        "wasm-pack",
        "wasm-bindgen",
    ] {
        let path = tools.join(tool);
        fs::write(&path, script).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = std::env::join_paths(std::iter::once(tools).chain(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    )))
    .unwrap();
    for case in ["missing", "mismatched"] {
        for flag in [None, Some("--no-setup"), Some("--offline")] {
            let log = root.join("commands.log");
            fs::write(&log, "").unwrap();
            let mut command = Command::new(env!("CARGO_BIN_EXE_jffi"));
            command
                .current_dir(&project)
                .env("PATH", &path)
                .env("JFFI_TEST_LOG", &log)
                .env("JFFI_TEST_CASE", case)
                // Inherited opt-in must not override explicit opt-outs.
                .env("JFFI_INSTALL_MISSING", "1")
                .env_remove("JFFI_NO_SETUP")
                .env_remove("CARGO_NET_OFFLINE")
                .args(["run", "--platform", "web"]);
            if let Some(flag) = flag {
                command.arg(flag);
            } else {
                command.env_remove("JFFI_INSTALL_MISSING");
            }
            let output = command.output().unwrap();
            assert!(!output.status.success());
            let commands = fs::read_to_string(&log).unwrap();
            if flag.is_some() {
                assert!(!commands.contains("cargo|install"), "{commands}");
            } else if case == "missing" {
                assert!(commands.contains("cargo|install wasm-pack"), "{commands}");
            } else {
                assert!(
                    commands.contains("cargo|install -f wasm-bindgen-cli --version 0.2.100"),
                    "{commands}"
                );
            }
        }
    }
    fs::remove_dir_all(root).unwrap();
}
