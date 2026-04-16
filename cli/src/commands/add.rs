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
    let name = config.package.name.clone();
    let platforms_dir = std::path::PathBuf::from("platforms");
    
    match platform {
        "ios" => crate::templates::ios::create_ios_project(&platforms_dir, &name, "todo")?,
        "android" => crate::templates::android::create_android_project(&platforms_dir, &name)?,
        "macos" => crate::templates::macos::create_macos_project(&platforms_dir, &name, "todo")?,
        "linux" => crate::templates::linux::create_linux_project(&platforms_dir, &name)?,
        "windows" => crate::templates::windows::create_windows_project(&platforms_dir, &name)?,
        "web" => crate::templates::web::create_web_project(&platforms_dir, &name)?,
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
