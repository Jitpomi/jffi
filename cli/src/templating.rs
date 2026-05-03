use anyhow::{Context, Result};
use rand::Rng;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// UniFFI version used in generated projects (default for iOS, macOS, Android, Linux, Web)
/// Windows template overrides to 0.29.4 for uniffi-bindgen-cs compatibility
const UNIFFI_VERSION: &str = "0.31.1";

/// Template manifest configuration
#[derive(Debug, Deserialize, Default)]
pub struct TemplateManifest {
    pub template: TemplateInfo,
    #[serde(default)]
    pub variables: HashMap<String, String>,
    #[serde(default)]
    pub platforms: PlatformConfig,
    #[serde(default)]
    pub metadata: TemplateMetadata,
}

#[derive(Debug, Deserialize, Default)]
pub struct TemplateMetadata {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub message: String,
}

impl TemplateMetadata {
    pub fn is_coming_soon(&self) -> bool {
        self.status == "coming_soon"
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct TemplateInfo {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub author: String,
    #[serde(default = "default_version")]
    #[allow(dead_code)]
    pub version: String,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

#[derive(Debug, Deserialize, Default)]
pub struct PlatformConfig {
    #[serde(default = "all_platforms")]
    pub supported: Vec<String>,
}

fn all_platforms() -> Vec<String> {
    vec![
        "ios".to_string(),
        "macos".to_string(),
        "android".to_string(),
        "windows".to_string(),
        "linux".to_string(),
        "web".to_string(),
    ]
}

/// A discovered template
#[derive(Debug)]
pub struct Template {
    pub name: String,
    pub path: PathBuf,
    pub manifest: TemplateManifest,
}

/// Template engine for loading and rendering templates
pub struct TemplateEngine {
    templates_dir: PathBuf,
}

impl TemplateEngine {
    /// Create engine with templates directory
    pub fn new(templates_dir: impl AsRef<Path>) -> Self {
        Self {
            templates_dir: templates_dir.as_ref().to_path_buf(),
        }
    }

    /// Discover all available templates
    pub fn discover_templates(&self) -> Result<Vec<Template>> {
        let mut templates = Vec::new();

        if !self.templates_dir.exists() {
            return Ok(templates);
        }

        for entry in fs::read_dir(&self.templates_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let manifest = self.load_manifest(&path, &name)?;

                templates.push(Template {
                    name: manifest.template.name.clone(),
                    path,
                    manifest,
                });
            }
        }

        // Sort by name for consistent ordering
        templates.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(templates)
    }

    /// Load manifest for a template directory
    fn load_manifest(&self, template_path: &Path, folder_name: &str) -> Result<TemplateManifest> {
        let manifest_path = template_path.join("manifest.toml");

        if manifest_path.exists() {
            let contents = fs::read_to_string(&manifest_path)
                .context("Failed to read manifest.toml")?;
            
            // Parse as raw TOML value first to handle nested tables
            let toml_value: toml::Value = toml::from_str(&contents)
                .context("Failed to parse manifest as TOML value")?;
            
            let mut manifest = TemplateManifest::default();
            
            if let toml::Value::Table(root) = toml_value {
                // Parse [template] section
                if let Some(toml::Value::Table(template_table)) = root.get("template") {
                    if let Some(toml::Value::String(name)) = template_table.get("name") {
                        manifest.template.name = name.clone();
                    }
                    if let Some(toml::Value::String(desc)) = template_table.get("description") {
                        manifest.template.description = desc.clone();
                    }
                    if let Some(toml::Value::String(author)) = template_table.get("author") {
                        manifest.template.author = author.clone();
                    }
                    if let Some(toml::Value::String(version)) = template_table.get("version") {
                        manifest.template.version = version.clone();
                    }
                }
                
                // Parse [variables] section
                if let Some(toml::Value::Table(vars_table)) = root.get("variables") {
                    for (key, value) in vars_table {
                        // Check if this is a nested table (platform-specific)
                        if let toml::Value::Table(platform_vars) = value {
                            // This is [variables.platform]
                            for (var_key, var_value) in platform_vars {
                                if let toml::Value::String(s) = var_value {
                                    let platform_key = format!("variables.{}:{}", key, var_key);
                                    manifest.variables.insert(platform_key, s.clone());
                                }
                            }
                        } else if let toml::Value::String(s) = value {
                            // This is a base variable
                            manifest.variables.insert(key.clone(), s.clone());
                        }
                    }
                }
                
                // Parse [platforms] section
                if let Some(toml::Value::Table(platforms_table)) = root.get("platforms") {
                    if let Some(toml::Value::Array(supported)) = platforms_table.get("supported") {
                        manifest.platforms.supported = supported
                            .iter()
                            .filter_map(|v| {
                                if let toml::Value::String(s) = v {
                                    Some(s.clone())
                                } else {
                                    None
                                }
                            })
                            .collect();
                    }
                }
                
                // Parse [metadata] section
                if let Some(toml::Value::Table(metadata_table)) = root.get("metadata") {
                    if let Some(toml::Value::String(status)) = metadata_table.get("status") {
                        manifest.metadata.status = status.clone();
                    }
                    if let Some(toml::Value::String(message)) = metadata_table.get("message") {
                        manifest.metadata.message = message.clone();
                    }
                }
            }
            
            Ok(manifest)
        } else {
            // Create default manifest
            Ok(TemplateManifest {
                template: TemplateInfo {
                    name: folder_name.to_string(),
                    description: format!("{} template", folder_name),
                    author: String::new(),
                    version: "1.0.0".to_string(),
                },
                variables: HashMap::new(),
                platforms: PlatformConfig::default(),
                metadata: TemplateMetadata::default(),
            })
        }
    }

    /// Get a specific template by name
    pub fn get_template(&self, name: &str) -> Result<Option<Template>> {
        let templates = self.discover_templates()?;
        Ok(templates.into_iter().find(|t| {
            t.name.eq_ignore_ascii_case(name) || 
            t.path.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.eq_ignore_ascii_case(name))
                .unwrap_or(false)
        }))
    }

    /// Generate project from template
    pub fn generate(
        &self,
        template: &Template,
        project_dir: &Path,
        name: &str,
        platforms: &[&str],
    ) -> Result<()> {
        // If Windows is included, use Windows-specific context for core (for UniFFI version)
        // Otherwise use base context
        let core_platform = if platforms.contains(&"windows") {
            Some("windows")
        } else {
            None
        };
        let core_context = self.build_context(name, &template.manifest, core_platform);
        self.generate_core(template, project_dir, &core_context)?;

        // Generate platforms with platform-specific contexts
        for platform in platforms {
            if template.manifest.platforms.supported.contains(&platform.to_string()) {
                let platform_context = self.build_context(name, &template.manifest, Some(platform));
                self.generate_platform(template, project_dir, platform, &platform_context)?;
            }
        }

        Ok(())
    }

    /// Build variable context for substitution
    /// If platform is provided, merges platform-specific variable overrides
    fn build_context(&self, name: &str, manifest: &TemplateManifest, platform: Option<&str>) -> HashMap<String, String> {
        let mut context = HashMap::new();

        // Standard variables
        context.insert("name".to_string(), name.to_string());
        context.insert("name_pascal".to_string(), to_pascal_case(name));
        context.insert("name_snake".to_string(), name.replace("-", "_"));
        context.insert("name_package".to_string(), name.replace("-", ""));

        // Generate UUIDs for Xcode project files (no hyphens)
        // macOS template needs up to UUID29
        let mut rng = rand::thread_rng();
        for i in 1..=30 {
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
                rng.gen::<u64>() & 0xFFFFFFFFFFFF // 48 bits
            );
            context.insert(format!("GUID{}", i), guid);
        }

        // Template manifest variables (can override defaults)
        // First, add base variables
        for (key, value) in &manifest.variables {
            // Skip platform-specific keys (they're in format "variables.platform:key")
            if !key.contains(':') {
                context.insert(key.clone(), value.clone());
            }
        }

        // Platform-specific variable overrides (e.g., [variables.windows])
        // These were stored as "variables.windows:uniffi_version" during manifest loading
        if let Some(platform_name) = platform {
            let platform_prefix = format!("variables.{}:", platform_name);
            for (key, value) in &manifest.variables {
                if key.starts_with(&platform_prefix) {
                    // Extract the actual variable name (after the ":")
                    if let Some(var_name) = key.split(':').nth(1) {
                        context.insert(var_name.to_string(), value.clone());
                    }
                }
            }
        }

        context
    }

    /// Generate core crate
    fn generate_core(
        &self,
        template: &Template,
        project_dir: &Path,
        context: &HashMap<String, String>,
    ) -> Result<()> {
        let core_dir = project_dir.join("core");
        fs::create_dir_all(core_dir.join("src"))?;

        // Read core.rs template
        let core_template = template.path.join("core.rs");
        if core_template.exists() {
            let content = fs::read_to_string(&core_template)?;
            let rendered = self.render_template(&content, context);
            fs::write(core_dir.join("src/lib.rs"), rendered)?;
        }

        // Generate Cargo.toml for core
        let uniffi_version = context.get("uniffi_version").map(|s| s.as_str()).unwrap_or(UNIFFI_VERSION);
        let cargo_toml = format!(
            r#"[package]
name = "{}-core"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "staticlib", "lib"]

[dependencies]
uniffi = {{ version = "{}", features = ["wasm-unstable-single-threaded"] }}
"#,
            context.get("name_snake").unwrap_or(&"app".to_string()),
            uniffi_version
        );
        fs::write(core_dir.join("Cargo.toml"), cargo_toml)?;

        // Generate uniffi.toml with C# binding configuration
        let uniffi_toml = format!(
            r#"[bindings.csharp]
cdylib_name = "{name_snake}_core"
"#,
            name_snake = context.get("name_snake").unwrap_or(&"app".to_string())
        );
        fs::write(core_dir.join("uniffi.toml"), uniffi_toml)?;

        Ok(())
    }

    /// Generate platform code
    fn generate_platform(
        &self,
        template: &Template,
        project_dir: &Path,
        platform: &str,
        context: &HashMap<String, String>,
    ) -> Result<()> {
        let platforms_dir = project_dir.join("platforms").join(platform);
        fs::create_dir_all(&platforms_dir)?;

        let platform_template_dir = template.path.join("platforms").join(platform);
        
        if !platform_template_dir.exists() {
            anyhow::bail!("Platform '{}' not supported by template '{}'", platform, template.name);
        }

        self.copy_dir_with_render(&platform_template_dir, &platforms_dir, context)?;

        Ok(())
    }

    /// Copy directory recursively with template rendering
    fn copy_dir_with_render(
        &self,
        src: &Path,
        dst: &Path,
        context: &HashMap<String, String>,
    ) -> Result<()> {
        fs::create_dir_all(dst)?;

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = path.file_name().unwrap_or_default().to_string_lossy();
            // Render file/directory names too (e.g., {{name_pascal}}.xcodeproj)
            let rendered_name = self.render_template(&file_name, context);
            let dest_path = dst.join(&rendered_name);

            if path.is_dir() {
                self.copy_dir_with_render(&path, &dest_path, context)?;
            } else {
                // Try to read as text for templating, fall back to binary copy
                match fs::read_to_string(&path) {
                    Ok(content) => {
                        let rendered = self.render_template(&content, context);
                        fs::write(&dest_path, rendered)?;
                    }
                    Err(_) => {
                        // Binary file - copy without modification
                        fs::copy(&path, &dest_path)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Simple template rendering - replace {{variable}} with value
    fn render_template(&self, template: &str, context: &HashMap<String, String>) -> String {
        let mut result = template.to_string();
        
        for (key, value) in context {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }
        
        result
    }
}

/// Convert kebab-case to PascalCase
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pascal_case() {
        assert_eq!(to_pascal_case("my-app"), "MyApp");
        assert_eq!(to_pascal_case("hello-world"), "HelloWorld");
        assert_eq!(to_pascal_case("test"), "Test");
    }

    #[test]
    fn test_render_template() {
        let engine = TemplateEngine::new("/tmp");
        let mut context = HashMap::new();
        context.insert("name".to_string(), "myapp".to_string());
        context.insert("greeting".to_string(), "Hello".to_string());

        let template = "Hello {{name}}, {{greeting}}!";
        let result = engine.render_template(template, &context);
        
        assert_eq!(result, "Hello myapp, Hello!");
    }

    #[test]
    fn test_uniffi_version_default() {
        let mut context = HashMap::new();
        context.insert("name_snake".to_string(), "test_app".to_string());
        
        // Simulate Cargo.toml generation without uniffi_version override
        let uniffi_version = context.get("uniffi_version").map(|s| s.as_str()).unwrap_or(UNIFFI_VERSION);
        
        assert_eq!(uniffi_version, "0.31.1");
    }

    #[test]
    fn test_uniffi_version_override() {
        let mut context = HashMap::new();
        context.insert("name_snake".to_string(), "test_app".to_string());
        context.insert("uniffi_version".to_string(), "0.32.0".to_string());
        
        // Simulate Cargo.toml generation with uniffi_version override
        let uniffi_version = context.get("uniffi_version").map(|s| s.as_str()).unwrap_or(UNIFFI_VERSION);
        
        assert_eq!(uniffi_version, "0.32.0");
    }
}
