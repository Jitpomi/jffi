use anyhow::Result;
use colored::*;
use std::env;
use std::fs;
use std::path::PathBuf;

use crate::templating::TemplateEngine;
use crate::commands::new::{create_ffi_web_crate, create_workspace_cargo_toml};

/// Find templates directory (same logic as new.rs)
fn find_templates_dir() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dev_templates = manifest_dir.join("templates");
    if dev_templates.exists() {
        return Ok(dev_templates);
    }
    
    let exe_path = env::current_exe()?;
    if let Some(exe_dir) = exe_path.parent() {
        let share_templates = exe_dir.join("../share/jffi/templates");
        if share_templates.exists() {
            return Ok(share_templates.canonicalize()?);
        }
        
        let local_templates = exe_dir.join("templates");
        if local_templates.exists() {
            return Ok(local_templates);
        }
        
        let sibling_templates = exe_dir.join("../templates");
        if sibling_templates.exists() {
            return Ok(sibling_templates.canonicalize()?);
        }
    }
    
    Ok(dev_templates)
}

pub fn add_platform(platform: &str) -> Result<()> {
    println!("{}", format!("➕ Adding {} platform...", platform).bright_green().bold());
    println!();
    
    // Check if platform already exists
    let platform_dir = std::path::PathBuf::from("platforms").join(platform);
    if platform_dir.exists() {
        anyhow::bail!("Platform {} already exists", platform);
    }
    
    // Read current config
    let mut config = crate::config::load_config()?;
    let name = config.package.name.clone();
    
    // Initialize template engine
    let templates_dir = find_templates_dir()?;
    let engine = TemplateEngine::new(&templates_dir);
    
    // Get the template used for this project (default to hello)
    let template_name = "hello";
    let template = engine.get_template(template_name)?
        .ok_or_else(|| anyhow::anyhow!("Template '{}' not found", template_name))?;
    
    // Check if platform is supported by template
    if !template.manifest.platforms.supported.contains(&platform.to_string()) {
        anyhow::bail!(
            "Platform '{}' is not supported by template '{}'",
            platform,
            template_name
        );
    }
    
    // Create platforms directory
    let platforms_dir = std::path::PathBuf::from("platforms");
    fs::create_dir_all(&platforms_dir)?;
    
    // Generate only this platform using template engine
    let mut context = std::collections::HashMap::new();
    context.insert("name".to_string(), name.clone());
    context.insert("name_pascal".to_string(), to_pascal_case(&name));
    context.insert("name_snake".to_string(), name.replace("-", "_"));
    context.insert("name_package".to_string(), name.replace("-", ""));
    context.insert("greeting".to_string(), "Hello from JFFI".to_string());
    
    // Generate UUIDs for Xcode project files (no hyphens)
    // macOS template needs up to UUID29
    use rand::RngExt;
    let mut rng = rand::rng();
    for i in 1..=30 {
        let uuid = format!(
            "{:08X}{:08X}{:08X}{:08X}",
            rng.random::<u32>(),
            rng.random::<u32>(),
            rng.random::<u32>(),
            rng.random::<u32>()
        );
        context.insert(format!("UUID{}", i), uuid);
    }
    
    // Generate GUIDs for Windows manifests (with hyphens, RFC 4122 format)
    for i in 1..=5 {
        let guid = format!(
            "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
            rng.random::<u32>(),
            rng.random::<u16>(),
            rng.random::<u16>(),
            rng.random::<u16>(),
            rng.random::<u64>() & 0xFFFFFFFFFFFF
        );
        context.insert(format!("GUID{}", i), guid);
    }
    
    // Add template variables from manifest
    for (key, value) in &template.manifest.variables {
        context.insert(key.clone(), value.clone());
    }
    
    // Copy platform files from template
    let platform_template_dir = template.path.join("platforms").join(platform);
    if !platform_template_dir.exists() {
        anyhow::bail!(
            "Platform '{}' not found in template '{}'",
            platform,
            template_name
        );
    }
    
    let platform_dest_dir = platforms_dir.join(platform);
    fs::create_dir_all(&platform_dest_dir)?;
    
    copy_dir_with_render(&platform_template_dir, &platform_dest_dir, &context)?;
    
    // Create web-specific infrastructure if adding web platform
    if platform == "web" {
        let project_dir = PathBuf::from(".");
        let all_platforms: Vec<&str> = config.platforms.enabled.iter().map(|s| s.as_str()).collect();
        let mut updated_platforms = all_platforms.clone();
        if !updated_platforms.contains(&"web") {
            updated_platforms.push("web");
        }
        create_ffi_web_crate(&project_dir, &name)?;
        create_workspace_cargo_toml(&project_dir, &updated_platforms)?;
        println!("  {} ffi-web/", "✓".green());
    }
    
    // Create .cargo/config.toml for iOS/macOS deployment targets
    if platform == "ios" || platform == "macos" {
        let project_dir = PathBuf::from(".");
        if !project_dir.join(".cargo").join("config.toml").exists() {
            fs::create_dir_all(project_dir.join(".cargo"))?;
            let cargo_config = r#"[env]
IPHONEOS_DEPLOYMENT_TARGET = "16.0"
MACOSX_DEPLOYMENT_TARGET = "13.0"
"#;
            fs::write(project_dir.join(".cargo").join("config.toml"), cargo_config)?;
            println!("  {} .cargo/config.toml", "✓".green());
        }
    }
    
    // Add platform to config
    if !config.platforms.enabled.contains(&platform.to_string()) {
        config.platforms.enabled.push(platform.to_string());
        crate::config::save_config(&config)?;
    }
    
    
    println!("  {} platforms/{}/", "✓".green(), platform);
    println!();
    println!("{}", format!("✅ {} platform added", platform).green());
    println!();
    println!("Next steps:");
    println!("  {} - Compile the core and generate bindings", format!("jffi build --platform {}", platform).bright_cyan());
    println!("  {} - Build and launch on simulator/device", format!("jffi run --platform {}", platform).bright_cyan());
    println!("  {} - Watch mode (auto-rebuild on changes)", format!("jffi dev --platform {}", platform).bright_cyan());
    
    Ok(())
}

fn copy_dir_with_render(
    src: &PathBuf,
    dst: &PathBuf,
    context: &std::collections::HashMap<String, String>,
) -> Result<()> {
    fs::create_dir_all(dst)?;
    
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name().unwrap_or_default().to_string_lossy();
        
        // Render template variables in filename/directory name
        let rendered_name = render_template(&file_name, context);
        let dest_path = dst.join(rendered_name);
        
        if path.is_dir() {
            copy_dir_with_render(&path, &dest_path, context)?;
        } else {
            // Try to read as text for templating, fall back to binary copy
            match fs::read_to_string(&path) {
                Ok(content) => {
                    let rendered = render_template(&content, context);
                    fs::write(&dest_path, rendered)?;
                }
                Err(_) => {
                    // Binary file (image, etc.) - copy without modification
                    fs::copy(&path, &dest_path)?;
                }
            }
        }
    }
    
    Ok(())
}

fn render_template(template: &str, context: &std::collections::HashMap<String, String>) -> String {
    let mut result = template.to_string();
    
    for (key, value) in context {
        let placeholder = format!("{{{{{}}}}}", key);
        result = result.replace(&placeholder, value);
    }
    
    result
}

fn to_pascal_case(s: &str) -> String {
    s.split('-')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

