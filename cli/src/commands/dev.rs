use anyhow::{Context, Result};
use colored::*;
use std::process::Command;

pub fn watch_project(platform: &str) -> Result<()> {
    println!("{}", format!("👀 Watch mode for {}...", platform).bright_green().bold());
    println!("   Watching for Rust changes...");
    println!("   Press Ctrl+C to stop");
    println!();
    
    // Check if cargo-watch is installed
    if !Command::new("which")
        .arg("cargo-watch")
        .output()?
        .status
        .success()
    {
        println!("{}", "  ⚠️  cargo-watch not installed".yellow());
        println!("     Install: cargo install cargo-watch");
        println!();
        anyhow::bail!("cargo-watch required for watch mode");
    }
    
    let build_cmd = get_build_command(platform)?;
    let bindings_cmd = get_bindings_command(platform)?;
    
    let status = Command::new("cargo-watch")
        .args(&[
            "-w", "core",
            "-w", "ffi",
            "-x", &build_cmd,
            "-s", &bindings_cmd,
            "-s", &format!("echo '✅ {} updated! Reload your IDE'", platform),
        ])
        .status()
        .context("Failed to start watch mode")?;
    
    if !status.success() {
        anyhow::bail!("Watch mode failed");
    }
    
    Ok(())
}

fn get_build_command(platform: &str) -> Result<String> {
    let target = match platform {
        "ios" => "aarch64-apple-ios-sim",
        "android" => "aarch64-linux-android",
        "macos" | "macos-arm64" => "aarch64-apple-darwin",
        "macos-x64" => "x86_64-apple-darwin",
        "windows" | "windows-x64" => "x86_64-pc-windows-msvc",
        "windows-x86" => "i686-pc-windows-msvc",
        "linux" => "x86_64-unknown-linux-gnu",
        "web" => "wasm32-unknown-unknown",
        _ => anyhow::bail!("Unknown platform: {}", platform),
    };
    
    Ok(format!("build --release --package ffi --target {}", target))
}

fn get_bindings_command(platform: &str) -> Result<String> {
    let (target, profile, ext, lang, out_dir) = match platform {
        "ios" => (
            "aarch64-apple-ios-sim",
            "release",
            "dylib",
            "swift",
            "platforms/ios",
        ),
        "android" => (
            "aarch64-linux-android",
            "release",
            "so",
            "kotlin",
            "platforms/android/app/src/main/java",
        ),
        "macos" | "macos-arm64" => (
            "aarch64-apple-darwin",
            "release",
            "dylib",
            "swift",
            "platforms/macos",
        ),
        "macos-x64" => (
            "x86_64-apple-darwin",
            "release",
            "dylib",
            "swift",
            "platforms/macos",
        ),
        "linux" => (
            "x86_64-unknown-linux-gnu",
            "release",
            "so",
            "python",
            "platforms/linux",
        ),
        _ => anyhow::bail!("Watch mode not supported for platform: {}", platform),
    };
    
    Ok(format!(
        "uniffi-bindgen-cli generate --library target/{}/{}/libffi.{} --language {} --out-dir {}",
        target, profile, ext, lang, out_dir
    ))
}
