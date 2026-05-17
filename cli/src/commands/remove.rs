use anyhow::{Context, Result};
use colored::*;
use std::fs;
use std::path::PathBuf;

pub fn remove_platform(platform: &str) -> Result<()> {
    println!("{}", format!("🗑️  Removing {} platform...", platform).bright_red().bold());
    println!();

    // Validate project structure
    if !PathBuf::from("jffi.toml").exists() {
        anyhow::bail!(
            "{}\n\nThis command must be run from a project created with:\n  {} {}",
            "Error: Not in a JFFI project directory.".red().bold(),
            "jffi new".bright_cyan(),
            "<project-name>".bright_yellow()
        );
    }

    // Load and validate config
    let mut config = crate::config::load_config()?;
    if !config.platforms.enabled.contains(&platform.to_string()) {
        anyhow::bail!("Platform '{}' is not enabled in this project", platform);
    }

    let platform_dir = PathBuf::from("platforms").join(platform);

    // Remove platform directory
    if platform_dir.exists() {
        fs::remove_dir_all(&platform_dir)
            .with_context(|| format!("Failed to remove platforms/{} directory", platform))?;
        println!("  {} Removed platforms/{}/", "✓".green(), platform);
    } else {
        println!("  {} platforms/{}/ did not exist", "→".bright_blue(), platform);
    }

    // Web-specific cleanup: remove ffi-web crate and update workspace
    if platform == "web" {
        let ffi_web_dir = PathBuf::from("ffi-web");
        if ffi_web_dir.exists() {
            fs::remove_dir_all(&ffi_web_dir)
                .context("Failed to remove ffi-web directory")?;
            println!("  {} Removed ffi-web/", "✓".green());
        }

        // Update workspace Cargo.toml to remove ffi-web member
        let workspace_toml = PathBuf::from("Cargo.toml");
        if workspace_toml.exists() {
            let content = fs::read_to_string(&workspace_toml)?;
            let updated = content
                .replace(r#"["core", "ffi-web"]"#, r#"["core"]"#)
                .replace(r#"members = ["core", "ffi-web"]"#, r#"members = ["core"]"#);
            if content != updated {
                fs::write(&workspace_toml, updated)
                    .context("Failed to update workspace Cargo.toml")?;
                println!("  {} Updated workspace Cargo.toml", "✓".green());
            }
        }
    }

    // Remove from config
    config
        .platforms
        .enabled
        .retain(|p| p != platform);
    crate::config::save_config(&config)?;
    println!("  {} Updated jffi.toml", "✓".green());

    println!();
    println!("{}", format!("✅ {} platform removed", platform).bright_red());
    println!();
    println!("Tip: Run {} to re-add it later.",
        format!("jffi add {}", platform).bright_cyan()
    );

    Ok(())
}
