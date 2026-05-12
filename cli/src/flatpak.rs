use anyhow::{Context, Result};
use colored::*;
use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Command;
use crate::config::load_config;

pub fn build_flatpak(release: bool) -> Result<()> {
    let config = load_config()?;
    let app_id = &config.platforms.linux.app_id;
    let runtime_version = &config.platforms.linux.runtime_version;
    let profile = if release { "release" } else { "debug" };
    
    // 1. Get library name from core/Cargo.toml
    let cargo_toml = fs::read_to_string("core/Cargo.toml")
        .context("Failed to read core/Cargo.toml")?;
    let lib_name = cargo_toml
        .lines()
        .find(|line| line.trim().starts_with("name"))
        .and_then(|line| line.split('"').nth(1))
        .context("Could not find project name in core/Cargo.toml")?
        .replace("-", "_");
    
    let lib_filename = format!("lib{}.so", lib_name);
    
    println!("  {} Generating Flatpak manifest for {}...", "→".bright_blue(), app_id);
    
    let manifest_dir = ".jffi/flatpak";
    fs::create_dir_all(manifest_dir)?;
    let manifest_path = format!("{}/manifest.json", manifest_dir);
    
    let manifest = json!({
        "app-id": app_id,
        "runtime": "org.gnome.Platform",
        "runtime-version": runtime_version,
        "sdk": "org.gnome.Sdk",
        "sdk-extensions": ["org.freedesktop.Sdk.Extension.rust-stable"],
        "command": "launch-app",
        "finish-args": [
            "--share=ipc",
            "--socket=fallback-x11",
            "--socket=wayland",
            "--device=dri",
            "--share=network",
            "--filesystem=home"
        ],
        "build-options": {
            "append-path": "/usr/lib/sdk/rust-stable/bin",
            "build-args": ["--share=network"],
            "env": {
                "CARGO_HOME": "/run/build/jffi-module/cargo"
            }
        },
        "modules": [
            {
                "name": "jffi-module",
                "buildsystem": "simple",
                "build-commands": [
                    // Install Python requirements
                    "pip3 install --prefix=/app -r platforms/linux/requirements.txt",
                    // Build Rust core
                    format!("cargo build {} --manifest-path core/Cargo.toml", if release { "--release" } else { "" }),
                    // Install app files
                    "mkdir -p /app/bin /app/lib",
                    format!("cp target/{}/{} /app/lib/", profile, lib_filename),
                    "cp -r platforms/linux/* /app/bin/",
                    // Create entrypoint script
                    "echo '#!/bin/sh' > /app/bin/launch-app",
                    "echo 'export PYTHONPATH=$PYTHONPATH:/app/lib' >> /app/bin/launch-app",
                    "echo 'python3 /app/bin/main.py' >> /app/bin/launch-app",
                    "chmod +x /app/bin/launch-app"
                ],
                "sources": [
                    { "type": "dir", "path": "../../" }
                ]
            }
        ]
    });
    
    fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;
    
    println!("  {} Building Flatpak bundle (this may take a while)...", "→".bright_blue());
    
    let status = Command::new("flatpak-builder")
        .args(&["--user", "--install", "--force-clean", "build-dir", "manifest.json"])
        .current_dir(manifest_dir)
        .status()
        .context("Failed to run flatpak-builder. Is it installed?")?;
        
    if !status.success() {
        anyhow::bail!("Flatpak build failed");
    }
    
    println!("  {} Flatpak built and installed successfully!", "✓".green());
    Ok(())
}

pub fn run_flatpak() -> Result<()> {
    let config = load_config()?;
    let app_id = &config.platforms.linux.app_id;
    
    println!("  {} Launching {} via Flatpak...", "→".bright_blue(), app_id);
    
    Command::new("flatpak")
        .args(&["run", app_id])
        .status()
        .context("Failed to run flatpak")?;
        
    Ok(())
}

pub fn check_flatpak_requirements() -> Result<()> {
    let check = Command::new("flatpak-builder").arg("--version").output();
    if check.is_err() || !check.unwrap().status.success() {
        anyhow::bail!("flatpak-builder not found. Install it with: sudo apt install flatpak-builder");
    }
    
    // Check for runtime/sdk? We'll let flatpak-builder handle that error as it's more descriptive.
    Ok(())
}
