use anyhow::{Context, Result};
use colored::*;
use std::fs;
use std::path::PathBuf;

pub fn create_project(name: &str, platforms: &str, path: Option<PathBuf>) -> Result<()> {
    let project_dir = path.unwrap_or_else(|| PathBuf::from(name));
    
    println!("{}", "🚀 Creating new JFFI app...".bright_green().bold());
    println!("   Name: {}", name.bright_cyan());
    println!("   Platforms: {}", platforms.bright_cyan());
    println!();
    
    // Parse platforms
    let platform_list: Vec<&str> = if platforms == "multi" {
        vec!["ios", "android", "macos", "windows", "linux", "web"]
    } else {
        platforms.split(',').map(|s| s.trim()).collect()
    };
    
    // Create project structure
    create_project_structure(&project_dir, name, &platform_list)?;
    
    println!();
    println!("{}", "✅ Project created successfully!".bright_green().bold());
    println!();
    println!("Next steps:");
    println!("  cd {}", name);
    println!("  jffi build --platform {}", platform_list[0]);
    println!("  jffi run --platform {}", platform_list[0]);
    println!();
    
    Ok(())
}

fn create_project_structure(dir: &PathBuf, name: &str, platforms: &[&str]) -> Result<()> {
    // Create root directory
    fs::create_dir_all(dir).context("Failed to create project directory")?;
    
    // Create workspace Cargo.toml
    create_workspace_cargo_toml(dir)?;
    
    // Create core business logic crate
    create_core_crate(dir, name)?;
    
    // Create FFI crate
    create_ffi_crate(dir, name)?;
    
    // Create platform directories
    for platform in platforms {
        create_platform_dir(dir, name, platform)?;
    }
    
    // Create config file
    create_config_file(dir, name, platforms)?;
    
    // Create Makefile
    create_makefile(dir, platforms)?;
    
    // Create README
    create_readme(dir, name, platforms)?;
    
    Ok(())
}

fn create_workspace_cargo_toml(dir: &PathBuf) -> Result<()> {
    let cargo_toml = r#"[workspace]
members = ["core", "ffi"]
resolver = "2"
"#;
    fs::write(dir.join("Cargo.toml"), cargo_toml)?;
    Ok(())
}

fn create_core_crate(dir: &PathBuf, name: &str) -> Result<()> {
    let core_dir = dir.join("core");
    fs::create_dir_all(core_dir.join("src"))?;
    
    // Cargo.toml
    let cargo_toml = format!(r#"[package]
name = "{}-core"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = {{ version = "1.0", features = ["derive"] }}
"#, name);
    fs::write(core_dir.join("Cargo.toml"), cargo_toml)?;
    
    // lib.rs with example code
    let lib_rs = r#"use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub title: String,
    pub completed: bool,
}

pub struct App {
    items: Vec<Item>,
}

impl App {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }
    
    pub fn add_item(&mut self, id: String, title: String) {
        self.items.push(Item {
            id,
            title,
            completed: false,
        });
    }
    
    pub fn toggle_item(&mut self, id: &str) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.completed = !item.completed;
        }
    }
    
    pub fn delete_item(&mut self, id: &str) {
        self.items.retain(|i| i.id != id);
    }
    
    pub fn get_items(&self) -> &[Item] {
        &self.items
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
"#;
    fs::write(core_dir.join("src/lib.rs"), lib_rs)?;
    
    println!("  {} core/", "✓".green());
    Ok(())
}

fn create_ffi_crate(dir: &PathBuf, name: &str) -> Result<()> {
    let ffi_dir = dir.join("ffi");
    fs::create_dir_all(ffi_dir.join("src"))?;
    
    // Cargo.toml
    let cargo_toml = format!(r#"[package]
name = "{}-ffi"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "lib"]

[[bin]]
name = "uniffi-bindgen"
path = "uniffi-bindgen.rs"

[dependencies]
{}-core = {{ path = "../core" }}
uniffi = {{ version = "0.31.0", features = ["cli"] }}

[build-dependencies]
uniffi = {{ version = "0.31.0", features = ["build"] }}
"#, name, name);
    fs::write(ffi_dir.join("Cargo.toml"), cargo_toml)?;
    
    // Create uniffi-bindgen.rs binary
    let uniffi_bindgen_rs = r#"fn main() {
    uniffi::uniffi_bindgen_main()
}
"#;
    fs::write(ffi_dir.join("uniffi-bindgen.rs"), uniffi_bindgen_rs)?;
    
    // lib.rs with FFI exports
    let module_name = name.replace("-", "_");
    let lib_rs = format!(r#"use {}_core::{{App, Item}};
use std::sync::Mutex;

#[derive(uniffi::Record)]
pub struct ItemViewModel {{
    pub id: String,
    pub title: String,
    pub completed: bool,
}}

impl From<&Item> for ItemViewModel {{
    fn from(item: &Item) -> Self {{
        Self {{
            id: item.id.clone(),
            title: item.title.clone(),
            completed: item.completed,
        }}
    }}
}}

#[derive(uniffi::Object)]
pub struct FfiApp {{
    app: Mutex<App>,
}}

#[uniffi::export]
impl FfiApp {{
    #[uniffi::constructor]
    pub fn new() -> Self {{
        Self {{
            app: Mutex::new(App::new()),
        }}
    }}
    
    pub fn add_item(&self, id: String, title: String) -> Vec<ItemViewModel> {{
        let mut app = self.app.lock().unwrap();
        app.add_item(id, title);
        app.get_items().iter().map(ItemViewModel::from).collect()
    }}
    
    pub fn toggle_item(&self, id: String) -> Vec<ItemViewModel> {{
        let mut app = self.app.lock().unwrap();
        app.toggle_item(&id);
        app.get_items().iter().map(ItemViewModel::from).collect()
    }}
    
    pub fn delete_item(&self, id: String) -> Vec<ItemViewModel> {{
        let mut app = self.app.lock().unwrap();
        app.delete_item(&id);
        app.get_items().iter().map(ItemViewModel::from).collect()
    }}
    
    pub fn get_items(&self) -> Vec<ItemViewModel> {{
        let app = self.app.lock().unwrap();
        app.get_items().iter().map(ItemViewModel::from).collect()
    }}
}}

uniffi::setup_scaffolding!();
"#, module_name);
    fs::write(ffi_dir.join("src/lib.rs"), lib_rs)?;
    
    println!("  {} ffi/", "✓".green());
    Ok(())
}

fn create_platform_dir(dir: &PathBuf, name: &str, platform: &str) -> Result<()> {
    let platforms_dir = dir.join("platforms");
    fs::create_dir_all(&platforms_dir)?;
    
    match platform {
        "ios" => crate::templates::ios::create_ios_project(&platforms_dir, name)?,
        "android" => crate::templates::android::create_android_project(&platforms_dir, name)?,
        "macos" => crate::templates::macos::create_macos_project(&platforms_dir, name)?,
        "windows" => crate::templates::windows::create_windows_project(&platforms_dir, name)?,
        "linux" => crate::templates::linux::create_linux_project(&platforms_dir, name)?,
        "web" => crate::templates::web::create_web_project(&platforms_dir, name)?,
        _ => println!("  {} Unknown platform: {}", "✗".red(), platform),
    }
    
    Ok(())
}

fn create_config_file(dir: &PathBuf, name: &str, platforms: &[&str]) -> Result<()> {
    let config = format!(r#"[package]
name = "{}"
version = "0.1.0"

[platforms]
enabled = {:?}

[platforms.ios]
deployment_target = "16.0"
bundle_id = "com.example.{}"

[platforms.android]
min_sdk = 26
package = "com.example.{}"

[platforms.macos]
deployment_target = "13.0"

[platforms.windows]
min_version = "10.0.19041.0"

[platforms.linux]
gtk_version = "4.0"

[platforms.web]
target = "es2020"
"#, name, platforms, name.replace("-", ""), name.replace("-", ""));
    
    fs::write(dir.join("jffi.toml"), config)?;
    println!("  {} jffi.toml", "✓".green());
    Ok(())
}

fn create_makefile(dir: &PathBuf, platforms: &[&str]) -> Result<()> {
    let first_platform = platforms.first().unwrap_or(&"ios");
    
    let makefile = format!(r#".PHONY: help build run dev clean

help:
	@echo "UniFFI App - Build Commands"
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
	@uniffi-app build --platform $(PLATFORM)

run:
	@uniffi-app run --platform $(PLATFORM)

dev:
	@uniffi-app dev --platform $(PLATFORM)

clean:
	@cargo clean
	@echo "✅ Cleaned build artifacts"
"#, platforms.join(", "), first_platform, first_platform);
    
    fs::write(dir.join("Makefile"), makefile)?;
    println!("  {} Makefile", "✓".green());
    Ok(())
}

fn create_readme(dir: &PathBuf, name: &str, platforms: &[&str]) -> Result<()> {
    let readme = format!(r#"# {}

Cross-platform app built with Rust + UniFFI

## Platforms

{}

## Quick Start

```bash
# Build for your platform
uniffi-app build --platform {}

# Run the app
uniffi-app run --platform {}

# Development mode (auto-rebuild)
uniffi-app dev --platform {}
```

## Project Structure

- `core/` - Business logic (pure Rust)
- `ffi/` - FFI layer (UniFFI exports)
- `platforms/` - Platform-specific UIs

## Development

Edit your business logic in `core/src/lib.rs`. The FFI bindings will be automatically regenerated.

## Adding Features

1. Add logic to `core/src/lib.rs`
2. Expose via FFI in `ffi/src/lib.rs`
3. Rebuild: `uniffi-app build --platform <platform>`
4. Update UI in `platforms/<platform>/`

Built with [UniFFI Framework](https://github.com/mozilla/uniffi-rs)
"#, name, 
    platforms.iter().map(|p| format!("- {}", p)).collect::<Vec<_>>().join("\n"),
    platforms[0], platforms[0], platforms[0]);
    
    fs::write(dir.join("README.md"), readme)?;
    println!("  {} README.md", "✓".green());
    Ok(())
}
