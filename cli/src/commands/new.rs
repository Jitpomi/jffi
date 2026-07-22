use anyhow::{Context, Result};
use colored::*;
use dialoguer::{theme::ColorfulTheme, Input, MultiSelect};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::templating::TemplateEngine;
use crate::platform::Platform;

/// Find templates directory with multiple fallback locations
fn find_templates_dir() -> Result<PathBuf> {
    // 1. Development mode: use templates relative to source
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dev_templates = manifest_dir.join("templates");
    if dev_templates.exists() {
        return Ok(dev_templates);
    }
    
    // 2. Installed mode: check relative to executable
    let exe_path = env::current_exe()?;
    
    // If exe is in ~/.cargo/bin/, check for templates alongside
    if let Some(exe_dir) = exe_path.parent() {
        // Try ../share/jffi/templates (standard FHS layout)
        let share_templates = exe_dir.join("../share/jffi/templates");
        if share_templates.exists() {
            return Ok(share_templates.canonicalize()?);
        }
        
        // Try templates/ subdirectory of exe location
        let local_templates = exe_dir.join("templates");
        if local_templates.exists() {
            return Ok(local_templates);
        }
        
        // Try ../templates (if exe is in bin/)
        let sibling_templates = exe_dir.join("../templates");
        if sibling_templates.exists() {
            return Ok(sibling_templates.canonicalize()?);
        }
    }
    
    // 3. Default: return the development path even if it doesn't exist
    // (will show a clearer error message later)
    Ok(dev_templates)
}

pub fn create_project(
    name: &str,
    platforms: Option<&str>,
    template: Option<&str>,
    path: Option<PathBuf>,
) -> Result<()> {
    let theme = ColorfulTheme::default();

    // Initialize template engine - find templates directory
    let templates_dir = find_templates_dir()?;
    let engine = TemplateEngine::new(&templates_dir);

    // Discover available templates
    let available_templates = engine.discover_templates()?;
    
    if available_templates.is_empty() {
        anyhow::bail!("No templates found in {:?}", templates_dir);
    }

    // Show available templates and select one
    let selected_template_name = if let Some(template_arg) = template {
        let template_lower = template_arg.to_lowercase();
        
        if let Some(t) = available_templates.iter().find(|t| 
            t.name.to_lowercase() == template_lower || 
            t.path.file_name().and_then(|n| n.to_str()).map(|n| n.to_lowercase()) == Some(template_lower.clone())
        ) {
            if t.manifest.metadata.is_coming_soon() {
                anyhow::bail!("Template '{}' is coming soon. Only 'hello' is available.", template_arg);
            }
            t.name.clone()
        } else {
            anyhow::bail!("Unknown template: {}. Available: {}", 
                template_arg,
                available_templates.iter()
                    .filter(|t| !t.manifest.metadata.is_coming_soon())
                    .map(|t| t.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    } else {
        // Filter available (non-coming-soon) templates
        let available: Vec<_> = available_templates.iter()
            .filter(|t| !t.manifest.metadata.is_coming_soon())
            .collect();
        
        if available.is_empty() {
            anyhow::bail!("No templates available yet.");
        }
        
        // Show all templates with status
        println!("{}", "Available templates:".bright_green().bold());
        for t in &available_templates {
            if t.manifest.metadata.is_coming_soon() {
                println!("  {} {} (coming soon)", "⏳".yellow(), t.name);
            } else {
                println!("  {} {} - {}", "✓".green(), t.name, t.manifest.template.description);
            }
        }
        println!();
        
        // If only one template available, use it directly
        // Otherwise, show interactive selection
        if available.len() == 1 {
            println!("Using template: {} - {}", 
                available[0].name.bright_cyan(),
                available[0].manifest.template.description
            );
            available[0].name.clone()
        } else {
            // Interactive template selection
            let template_items: Vec<String> = available.iter()
                .map(|t| format!("{} - {}", t.name, t.manifest.template.description))
                .collect();
            
            let selected_idx = dialoguer::Select::with_theme(&theme)
                .with_prompt("Choose a template")
                .default(0)
                .items(&template_items)
                .interact()?;
            
            available[selected_idx].name.clone()
        }
    };

    let platform_list: Vec<String> = if let Some(platforms) = platforms {
        if platforms == "multi" {
            vec![
                "ios".to_string(),
                "android".to_string(),
                "macos".to_string(),
                "windows".to_string(),
                "linux".to_string(),
                "web".to_string(),
            ]
        } else {
            platforms
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        }
    } else {
        let items = ["ios", "macos", "android", "windows", "linux", "web"];
        let defaults = vec![true, false, false, false, false, false];
        let chosen = MultiSelect::with_theme(&theme)
            .with_prompt("Select platforms")
            .items(items)
            .defaults(&defaults)
            .interact()?;

        if chosen.is_empty() {
            anyhow::bail!("No platforms selected");
        }

        chosen.into_iter().map(|i| items[i].to_string()).collect()
    };

    let project_dir = if let Some(path) = path {
        path
    } else {
        let default_path = name.to_string();
        let input: String = Input::with_theme(&theme)
            .with_prompt("Project directory")
            .default(default_path)
            .interact_text()?;
        PathBuf::from(input)
    };
    
    println!("{}", "🚀 Creating new JFFI app...".bright_green().bold());
    println!("   Name: {}", name.bright_cyan());
    println!("   Platforms: {}", platform_list.join(",").bright_cyan());
    println!("   Template: {}", selected_template_name.bright_cyan());
    println!();

    let platform_list_ref: Vec<&str> = platform_list.iter().map(|s| s.as_str()).collect();
    
    let template = engine.get_template(&selected_template_name)?
        .ok_or_else(|| anyhow::anyhow!("Template '{}' not found", selected_template_name))?;
    
    create_project_structure(&engine, &template, &project_dir, name, &platform_list_ref)?;
    
    println!();
    println!("{}", "🔧 Generating FFI bindings...".bright_cyan().bold());
    println!();
    
    // Change to project directory for initial build
    let original_dir = env::current_dir()?;
    env::set_current_dir(&project_dir)?;
    
    // Run initial build for the first buildable platform to generate FFI bindings
    let first_buildable_platform = platform_list_ref.iter()
        .find(|&&p| {
            Platform::from_str(p)
                .map(|p_enum| p_enum.check_requirements().is_ok())
                .unwrap_or(false)
        });

    if let Some(platform) = first_buildable_platform {
        match crate::commands::build::build_platform(platform, false) {
            Ok(_) => {
                println!();
                println!("{}", format!("  ✅ FFI bindings generated for {}!", platform).green());
            }
            Err(e) => {
                println!();
                println!("{}", format!("  ⚠️  Warning: Initial build failed: {}", e).yellow());
                println!("{}", "  You can run 'jffi build' manually later.".yellow());
            }
        }
    } else {
        println!();
        println!("{}", "  ⚠️  Warning: No selected platforms are buildable on this host OS.".yellow());
        println!("{}", "  FFI bindings were not generated. You can generate them later on a supported OS.".yellow());
    }
    
    // Return to original directory
    env::set_current_dir(original_dir)?;
    
    println!();
    println!("{}", "✅ Project created successfully!".bright_green().bold());
    println!();
    println!("Next steps:");
    println!("  cd {}", project_dir.file_name().and_then(|s| s.to_str()).unwrap_or(name));
    println!();
    println!("  {} - Build and launch on simulator/device", format!("jffi run --platform {}", platform_list_ref[0]).bright_cyan());
    println!("  {} - Watch mode (auto-rebuild on changes)", format!("jffi dev --platform {}", platform_list_ref[0]).bright_cyan());
    println!();
    println!("{}", "  💡 Tip: All commands work independently - no need to build first!".bright_blue());
    println!();
    
    Ok(())
}

fn create_project_structure(
    engine: &TemplateEngine,
    template: &crate::templating::Template,
    dir: &PathBuf,
    name: &str,
    platforms: &[&str],
) -> Result<()> {
    fs::create_dir_all(dir).context("Failed to create project directory")?;
    
    create_workspace_cargo_toml(dir, platforms)?;
    
    engine.generate(template, dir, name, platforms)?;
    println!("  {} core/", "✓".green());
    
    if platforms.contains(&"web") {
        create_ffi_web_crate(dir, name)?;
    }
    
    create_config_file(dir, name, platforms)?;
    create_makefile(dir, platforms)?;
    create_readme(dir, name, platforms)?;
    
    Ok(())
}

pub fn create_workspace_cargo_toml(dir: &Path, platforms: &[&str]) -> Result<()> {
    let members = if platforms.contains(&"web") {
        r#"["core", "ffi-web"]"#
    } else {
        r#"["core"]"#
    };
    
    // Web projects optimize release build for size ("z"), others optimize for speed (3)
    let release_opt = if platforms.contains(&"web") { "\"z\"" } else { "3" };
    
    let cargo_toml = format!(
        r#"[workspace]
members = {}
resolver = "2"

# Optimize all dependencies in dev mode to make external libraries run at release speed
[profile.dev.package."*"]
opt-level = 3

# Also optimize our own core crate in dev mode
[profile.dev]
opt-level = 3

[profile.release]
opt-level = {}
lto = true
"#,
        members, release_opt
    );
    fs::write(dir.join("Cargo.toml"), cargo_toml)?;
    Ok(())
}

pub fn create_ffi_web_crate(dir: &Path, name: &str) -> Result<()> {
    let ffi_web_dir = dir.join("ffi-web");
    fs::create_dir_all(ffi_web_dir.join("src"))?;
    
    let module_name = name.replace("-", "_");
    let cargo_toml = format!(r#"[package]
name = "{}-ffi-web"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
{}-core = {{ path = "../core" }}
wasm-bindgen = "0.2"
"#, name, module_name);
    fs::write(ffi_web_dir.join("Cargo.toml"), cargo_toml)?;
    
    let module_name = name.replace("-", "_");
    let lib_rs = format!(r#"use {}_core::Core;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = Core)]
pub struct FfiCore {{
    core: Core,
}}

#[wasm_bindgen(js_class = Core)]
impl FfiCore {{
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {{
        Self {{ core: Core::new() }}
    }}
    
    pub fn greeting(&self) -> String {{
        self.core.greeting()
    }}
}}
"#, module_name);
    fs::write(ffi_web_dir.join("src/lib.rs"), lib_rs)?;
    
    println!("  {} ffi-web/", "✓".green());
    Ok(())
}

fn create_config_file(dir: &Path, name: &str, platforms: &[&str]) -> Result<()> {
    let platform_list: Vec<&str> = platforms.to_vec();
    let name_pascal: String = name.split('-').map(|part| {
        let mut chars = part.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }).collect();
    let config = format!(r##"[package]
name = "{}"
version = "0.1.0"

[platforms]
enabled = {:?}

[platforms.ios]
deployment_target = "16.0"
bundle_id = "com.example.{}"

[platforms.android]
min_sdk = 26
target_sdk = 36
package = "com.example.{}"

[platforms.macos]
deployment_target = "13.0"

[platforms.windows]
min_version = "10.0.19041.0"

[platforms.linux]
gtk_version = "4.0"

[platforms.web]
target = "es2020"
port = 5173
host = false
https = false
open = false
cors = true
title = "{}"
description = "A cross-platform app built with JFFI"
lang = "en"
# theme_color = "#000000"
# favicon = "/favicon.ico"
# og_image = "/og-image.png"
# og_url = "https://example.com"
# og_type = "website"
# twitter_card = "summary_large_image"
# keywords = ""
# author = ""
"##, name, platform_list, name.replace("-", ""), name.replace("-", ""), name_pascal);
    fs::write(dir.join("jffi.toml"), config)?;
    
    // Create .cargo/config.toml for iOS deployment target (needed by crates like blake3/ring)
    if platforms.contains(&"ios") || platforms.contains(&"macos") {
        fs::create_dir_all(dir.join(".cargo"))?;
        let cargo_config = r#"[env]
IPHONEOS_DEPLOYMENT_TARGET = "16.0"
MACOSX_DEPLOYMENT_TARGET = "13.0"
"#;
        fs::write(dir.join(".cargo").join("config.toml"), cargo_config)?;
    }
    
    Ok(())
}

fn create_makefile(dir: &Path, platforms: &[&str]) -> Result<()> {
    let first_platform = platforms.first().unwrap_or(&"ios");
    
    let makefile = format!(r#".PHONY: help build run dev clean

help:
	@echo "JFFI App - Build Commands"
	@echo ""
	@echo "  make build PLATFORM=<platform>  - Build for platform"
	@echo "  make run PLATFORM=<platform>    - Run on platform"
	@echo "  make dev PLATFORM=<platform>    - Watch mode"
	@echo "  make clean                      - Clean build artifacts"
	@echo ""
	@echo "Available platforms: {}"
	@echo "Default platform: {}"

PLATFORM ?= {}

build:
	@jffi build --platform $(PLATFORM)

run:
	@jffi run --platform $(PLATFORM)

dev:
	@jffi dev --platform $(PLATFORM)

clean:
	@cargo clean
	@echo "✅ Cleaned build artifacts"
"#, platforms.join(", "), first_platform, first_platform);
    
    fs::write(dir.join("Makefile"), makefile)?;
    println!("  {} Makefile", "✓".green());
    Ok(())
}

fn create_readme(dir: &Path, name: &str, platforms: &[&str]) -> Result<()> {
    let readme = format!(r#"# {}

Cross-platform app built with Rust + UniFFI

## Platforms

{}

## Quick Start

```bash
# Build for your platform
jffi build --platform {}

# Run the app
jffi run --platform {}

# Development mode (auto-rebuild)
jffi dev --platform {}
```

## Project Structure

- `core/` - Business logic + UniFFI exports
- `ffi-web/` - WASM FFI layer (present when web platform is enabled)
- `platforms/` - Platform-specific UIs
- `Cargo.toml` - Workspace manifest
- `jffi.toml` - Framework configuration

## Add or Remove Platforms

```bash
# Add another platform to this project
jffi add <platform>

# Remove a platform
jffi remove <platform>
```

## Development

Edit your business logic in `core/src/lib.rs`. The FFI bindings will be automatically regenerated.

## Adding Features

1. Add logic to `core/src/lib.rs`
2. Expose via `#[uniffi::export]`
3. Rebuild: `jffi build --platform <platform>`
4. Update UI in `platforms/<platform>/`
"#, name,
    platforms.iter().map(|p| format!("- {}", p)).collect::<Vec<_>>().join("\n"),
    platforms[0], platforms[0], platforms[0]);
    
    fs::write(dir.join("README.md"), readme)?;
    println!("  {} README.md", "✓".green());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_create_workspace_cargo_toml_non_web() {
        let unique_id = uuid::Uuid::new_v4().to_string();
        let temp_dir = env::temp_dir().join(format!("jffi_test_{}", unique_id));
        fs::create_dir_all(&temp_dir).unwrap();

        let platforms = vec!["ios", "android"];
        create_workspace_cargo_toml(&temp_dir, &platforms).unwrap();

        let cargo_toml_content = fs::read_to_string(temp_dir.join("Cargo.toml")).unwrap();
        assert!(cargo_toml_content.contains("members = [\"core\"]"));
        assert!(cargo_toml_content.contains("[profile.dev.package.\"*\"]"));
        assert!(cargo_toml_content.contains("opt-level = 3"));
        assert!(cargo_toml_content.contains("[profile.release]"));
        assert!(cargo_toml_content.contains("opt-level = 3"));
        assert!(cargo_toml_content.contains("lto = true"));

        fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_create_workspace_cargo_toml_web() {
        let unique_id = uuid::Uuid::new_v4().to_string();
        let temp_dir = env::temp_dir().join(format!("jffi_test_{}", unique_id));
        fs::create_dir_all(&temp_dir).unwrap();

        let platforms = vec!["ios", "web"];
        create_workspace_cargo_toml(&temp_dir, &platforms).unwrap();

        let cargo_toml_content = fs::read_to_string(temp_dir.join("Cargo.toml")).unwrap();
        assert!(cargo_toml_content.contains("members = [\"core\", \"ffi-web\"]"));
        assert!(cargo_toml_content.contains("[profile.dev.package.\"*\"]"));
        assert!(cargo_toml_content.contains("opt-level = 3"));
        assert!(cargo_toml_content.contains("[profile.release]"));
        assert!(cargo_toml_content.contains("opt-level = \"z\""));
        assert!(cargo_toml_content.contains("lto = true"));

        fs::remove_dir_all(temp_dir).ok();
    }
}
