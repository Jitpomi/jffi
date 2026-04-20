use anyhow::{Context, Result};
use colored::*;
use dialoguer::{theme::ColorfulTheme, Input, MultiSelect};
use std::fs;
use std::path::PathBuf;

// Template: Hello World
const HELLO_TEMPLATE: &str = r##"use uniffi;

#[derive(uniffi::Object)]
pub struct Core {}

#[uniffi::export]
impl Core {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {}
    }

    pub fn greeting(&self) -> String {
        "Hello from JFFI".to_string()
    }
}

uniffi::setup_scaffolding!();
"##;

// Template: Todo (placeholder - unified platform pattern)
const TODO_TEMPLATE: &str = r##"use uniffi;

#[derive(uniffi::Object)]
pub struct Core {}

#[uniffi::export]
impl Core {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {}
    }

    pub fn greeting(&self) -> String {
        "Hello from JFFI (Todo Template)".to_string()
    }
}

uniffi::setup_scaffolding!();
"##;

// Template: Counter (placeholder - unified platform pattern)
const COUNTER_TEMPLATE: &str = r##"use uniffi;

#[derive(uniffi::Object)]
pub struct Core {}

#[uniffi::export]
impl Core {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {}
    }

    pub fn greeting(&self) -> String {
        "Hello from JFFI (Counter Template)".to_string()
    }
}

uniffi::setup_scaffolding!();
"##;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectTemplate {
    Hello,
    Todo,
    Counter,
}

impl ProjectTemplate {
    fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "hello" | "helloworld" => Some(Self::Hello),
            "todo" => Some(Self::Todo),
            "counter" => Some(Self::Counter),
            _ => None,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::Hello => "hello",
            Self::Todo => "todo",
            Self::Counter => "counter",
        }
    }
}


pub fn create_project(
    name: &str,
    platforms: Option<&str>,
    template: Option<&str>,
    path: Option<PathBuf>,
) -> Result<()> {
    let theme = ColorfulTheme::default();

    let selected_template = if let Some(template) = template {
        let t = ProjectTemplate::from_str(template)
            .with_context(|| format!("Unknown template: {} (expected: hello)", template))?;
        if t != ProjectTemplate::Hello {
            anyhow::bail!("Template '{}' is coming soon. Only 'hello' is available.", template);
        }
        t
    } else {
        println!("{}", "Available templates:".bright_green().bold());
        println!("  ✓ HelloWorld");
        println!("  ⏳ Todo (coming soon)");
        println!("  ⏳ Counter (coming soon)");
        println!();
        ProjectTemplate::Hello
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
            .items(&items)
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
    println!("   Template: {}", selected_template.as_str().bright_cyan());
    println!();

    let platform_list_ref: Vec<&str> = platform_list.iter().map(|s| s.as_str()).collect();
    create_project_structure(&project_dir, name, &platform_list_ref, selected_template)?;
    
    println!();
    println!("{}", "✅ Project created successfully!".bright_green().bold());
    println!();
    println!("Next steps:");
    println!("  cd {}", project_dir.file_name().and_then(|s| s.to_str()).unwrap_or(name));
    println!("  jffi build --platform {}", platform_list_ref[0]);
    println!("  jffi run --platform {}", platform_list_ref[0]);
    println!();
    
    Ok(())
}

fn create_project_structure(
    dir: &PathBuf,
    name: &str,
    platforms: &[&str],
    template: ProjectTemplate,
) -> Result<()> {
    // Create root directory
    fs::create_dir_all(dir).context("Failed to create project directory")?;
    
    // Create workspace Cargo.toml
    create_workspace_cargo_toml(dir, platforms)?;
    
    // Create core crate with UniFFI annotations
    create_core_crate(dir, name, template)?;
    
    // Create FFI-Web crate if web platform is selected
    if platforms.contains(&"web") {
        create_ffi_web_crate(dir, name)?;
    }
    
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

fn create_workspace_cargo_toml(dir: &PathBuf, platforms: &[&str]) -> Result<()> {
    let members = if platforms.contains(&"web") {
        r#"["core", "ffi-web"]"#
    } else {
        r#"["core"]"#
    };
    
    let cargo_toml = format!(r#"[workspace]
members = {}
resolver = "2"
"#, members);
    fs::write(dir.join("Cargo.toml"), cargo_toml)?;
    Ok(())
}

fn create_core_crate(dir: &PathBuf, name: &str, template: ProjectTemplate) -> Result<()> {
    let core_dir = dir.join("core");
    fs::create_dir_all(core_dir.join("src"))?;
    
    // Cargo.toml with UniFFI
    let cargo_toml = format!(r#"[package]
name = "{}-core"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "staticlib", "lib"]

[dependencies]
uniffi = {{ version = "0.31.0", features = ["cli"] }}
"#, name);
    fs::write(core_dir.join("Cargo.toml"), cargo_toml)?;
    
    // src/lib.rs with UniFFI annotations
    let lib_rs = match template {
        ProjectTemplate::Hello => HELLO_TEMPLATE,
        ProjectTemplate::Todo => TODO_TEMPLATE,
        ProjectTemplate::Counter => COUNTER_TEMPLATE,
    };
    fs::write(core_dir.join("src/lib.rs"), lib_rs)?;
    
    println!("  {} core/", "✓".green());
    Ok(())
}

fn create_ffi_web_crate(dir: &PathBuf, name: &str) -> Result<()> {
    let ffi_web_dir = dir.join("ffi-web");
    fs::create_dir_all(ffi_web_dir.join("src"))?;
    
    // Cargo.toml for WASM
    let cargo_toml = format!(r#"[package]
name = "{}-ffi-web"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
{}-core = {{ path = "../core" }}
wasm-bindgen = "0.2"
serde = {{ version = "1.0", features = ["derive"] }}
serde-wasm-bindgen = "0.6"

[profile.release]
opt-level = "z"
lto = true
"#, name, name);
    fs::write(ffi_web_dir.join("Cargo.toml"), cargo_toml)?;
    
    // lib.rs with wasm-bindgen exports
    let module_name = name.replace("-", "_");
    let lib_rs = format!(r#"use {}_core::{{App, Item}};
use wasm_bindgen::prelude::*;
use serde::{{Serialize, Deserialize}};

#[derive(Serialize, Deserialize)]
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

#[wasm_bindgen]
pub struct FfiApp {{
    app: App,
}}

#[wasm_bindgen]
impl FfiApp {{
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {{
        Self {{
            app: App::new(),
        }}
    }}
    
    pub fn add_item(&mut self, id: String, title: String) -> JsValue {{
        self.app.add_item(id, title);
        let items: Vec<ItemViewModel> = self.app.get_items().iter().map(ItemViewModel::from).collect();
        serde_wasm_bindgen::to_value(&items).unwrap()
    }}
    
    pub fn toggle_item(&mut self, id: String) -> JsValue {{
        self.app.toggle_item(&id);
        let items: Vec<ItemViewModel> = self.app.get_items().iter().map(ItemViewModel::from).collect();
        serde_wasm_bindgen::to_value(&items).unwrap()
    }}
    
    pub fn delete_item(&mut self, id: String) -> JsValue {{
        self.app.delete_item(&id);
        let items: Vec<ItemViewModel> = self.app.get_items().iter().map(ItemViewModel::from).collect();
        serde_wasm_bindgen::to_value(&items).unwrap()
    }}
    
    pub fn get_items(&self) -> JsValue {{
        let items: Vec<ItemViewModel> = self.app.get_items().iter().map(ItemViewModel::from).collect();
        serde_wasm_bindgen::to_value(&items).unwrap()
    }}
}}
"#, module_name);
    fs::write(ffi_web_dir.join("src/lib.rs"), lib_rs)?;
    
    println!("  {} ffi-web/", "✓".green());
    Ok(())
}

fn create_platform_dir(
    dir: &PathBuf,
    name: &str,
    platform: &str,
) -> Result<()> {
    let platforms_dir = dir.join("platforms");
    fs::create_dir_all(&platforms_dir)?;
    
    match platform {
        "ios" => crate::templates::ios::create_ios_project(&platforms_dir, name, "hello")?,
        "android" => crate::templates::android::create_android_project(&platforms_dir, name, "hello")?,
        "macos" => crate::templates::macos::create_macos_project(&platforms_dir, name, "hello")?,
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

fn create_readme(dir: &PathBuf, name: &str, platforms: &[&str]) -> Result<()> {
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

- `core/` - Business logic (pure Rust)
- `ffi/` - FFI layer (UniFFI exports)
- `platforms/` - Platform-specific UIs

## Development

Edit your business logic in `core/src/lib.rs`. The FFI bindings will be automatically regenerated.

## Adding Features

1. Add logic to `core/src/lib.rs`
2. Expose via `#[uniffi::export]` in `core/src/lib.rs`
3. Rebuild: `jffi build --platform <platform>`
4. Update UI in `platforms/<platform>/`

Built with [UniFFI Framework](https://github.com/mozilla/uniffi-rs)
"#, name, 
    platforms.iter().map(|p| format!("- {}", p)).collect::<Vec<_>>().join("\n"),
    platforms[0], platforms[0], platforms[0]);
    
    fs::write(dir.join("README.md"), readme)?;
    println!("  {} README.md", "✓".green());
    Ok(())
}
