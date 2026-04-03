use anyhow::Result;
use colored::*;

pub fn add_platform(platform: &str) -> Result<()> {
    println!("{}", format!("➕ Adding {} platform...", platform).bright_green().bold());
    println!();
    
    // Check if platform already exists
    let platform_dir = std::path::PathBuf::from("platforms").join(platform);
    if platform_dir.exists() {
        anyhow::bail!("Platform {} already exists", platform);
    }
    
    // Read current config
    let mut config = crate::platforms::config::load_config()?;
    
    // Add platform to config
    if !config.platforms.enabled.contains(&platform.to_string()) {
        config.platforms.enabled.push(platform.to_string());
        crate::platforms::config::save_config(&config)?;
    }
    
    // Create platform directory
    match platform {
        "ios" => {
            let name = config.package.name.clone();
            crate::templates::ios::create_ios_project(&std::path::PathBuf::from("platforms"), &name)?;
        }
        "android" => {
            println!("  {} Android template (coming soon)", "○".yellow());
        }
        "macos" => {
            println!("  {} macOS template (coming soon)", "○".yellow());
        }
        "windows" => {
            println!("  {} Windows template (coming soon)", "○".yellow());
        }
        "linux" => {
            println!("  {} Linux template (coming soon)", "○".yellow());
        }
        "web" => {
            println!("  {} Web template (coming soon)", "○".yellow());
        }
        _ => anyhow::bail!("Unknown platform: {}", platform),
    }
    
    println!();
    println!("{}", format!("✅ {} platform added", platform).green());
    println!();
    println!("Next steps:");
    println!("  uniffi-app build --platform {}", platform);
    println!("  uniffi-app run --platform {}", platform);
    
    Ok(())
}
