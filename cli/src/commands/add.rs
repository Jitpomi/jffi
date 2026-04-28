use anyhow::Result;
use colored::*;
use std::env;
use std::fs;
use std::path::PathBuf;

use crate::templating::TemplateEngine;

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
    use rand::Rng;
    let mut rng = rand::thread_rng();
    for i in 1..=15 {
        let uuid = format!(
            "{:08X}{:08X}{:08X}{:08X}",
            rng.gen::<u32>(),
            rng.gen::<u32>(),
            rng.gen::<u32>(),
            rng.gen::<u32>()
        );
        context.insert(format!("UUID{}", i), uuid);
    }
    
    // Generate GUIDs for Windows manifests (with hyphens, RFC 4122 format)
    for i in 1..=5 {
        let guid = format!(
            "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
            rng.gen::<u32>(),
            rng.gen::<u16>(),
            rng.gen::<u16>(),
            rng.gen::<u16>(),
            rng.gen::<u64>() & 0xFFFFFFFFFFFF
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
            let content = fs::read_to_string(&path)?;
            let rendered = render_template(&content, context);
            fs::write(&dest_path, rendered)?;
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
