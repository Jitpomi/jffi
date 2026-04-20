use anyhow::Result;
use colored::*;
use std::fs;
use std::path::PathBuf;

pub fn create_web_project(platforms_dir: &PathBuf, name: &str) -> Result<()> {
    let web_dir = platforms_dir.join("web");
    fs::create_dir_all(&web_dir)?;
    
    // Create pkg directory with .gitkeep
    let pkg_dir = web_dir.join("pkg");
    fs::create_dir_all(&pkg_dir)?;
    fs::write(pkg_dir.join(".gitkeep"), "")?;
    
    // Create web application files
    create_index_html(&web_dir, name)?;
    create_main_js(&web_dir)?;
    create_styles_css(&web_dir)?;
    create_package_json(&web_dir, name)?;
    create_vite_config(&web_dir)?;
    create_gitignore(&web_dir)?;
    
    println!("  {} platforms/web/", "✓".green());
    Ok(())
}

fn create_index_html(dir: &PathBuf, name: &str) -> Result<()> {
    let title = name.split('-').map(|s| {
        let mut c = s.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    }).collect::<Vec<_>>().join(" ");
    
    let content = format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <link rel="stylesheet" href="styles.css">
</head>
<body>
    <div id="app">
        <div class="greeting-container">
            <h1 id="greeting">Loading...</h1>
            <button id="refresh-btn" class="btn-primary">Refresh</button>
        </div>
    </div>
    
    <script type="module" src="main.js"></script>
</body>
</html>
"#, title);
    
    fs::write(dir.join("index.html"), content)?;
    Ok(())
}

fn create_main_js(dir: &PathBuf) -> Result<()> {
    let content = r#"import init, { Core } from './pkg/wasm.js';

let core = null;

async function initApp() {
    await init();
    core = new Core();
    
    const greetingEl = document.getElementById('greeting');
    greetingEl.textContent = core.greeting();
    
    document.getElementById('refresh-btn').addEventListener('click', () => {
        greetingEl.textContent = core.greeting();
    });
}

initApp();
"#;
    
    fs::write(dir.join("main.js"), content)?;
    Ok(())
}

fn create_styles_css(dir: &PathBuf) -> Result<()> {
    let content = r#"* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
    background: #f5f5f7;
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
}

#app {
    background: white;
    padding: 48px;
    border-radius: 16px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
}

.greeting-container {
    text-align: center;
}

#greeting {
    font-size: 24px;
    font-weight: 600;
    color: #1d1d1f;
    margin-bottom: 24px;
    min-height: 32px;
}

.btn-primary {
    background: #007aff;
    color: white;
    border: none;
    padding: 12px 24px;
    border-radius: 8px;
    font-weight: 600;
    font-size: 16px;
    cursor: pointer;
    transition: background 0.2s;
}

.btn-primary:hover {
    background: #0051d5;
}
"#;
    
    fs::write(dir.join("styles.css"), content)?;
    Ok(())
}

fn create_package_json(dir: &PathBuf, name: &str) -> Result<()> {
    let content = format!(r#"{{
  "name": "{}-web",
  "version": "0.1.0",
  "type": "module",
  "scripts": {{
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview"
  }},
  "devDependencies": {{
    "vite": "^5.0.0"
  }}
}}
"#, name);
    
    fs::write(dir.join("package.json"), content)?;
    Ok(())
}

fn create_vite_config(dir: &PathBuf) -> Result<()> {
    let content = r#"import { defineConfig } from 'vite';

export default defineConfig({
  server: {
    port: 3000,
    open: true,
    fs: {
      strict: false
    }
  },
  optimizeDeps: {
    exclude: ['./pkg/wasm.js']
  }
});
"#;
    
    fs::write(dir.join("vite.config.js"), content)?;
    Ok(())
}

fn create_gitignore(dir: &PathBuf) -> Result<()> {
    let content = r#"node_modules
dist
pkg/*.js
pkg/*.wasm
pkg/*.ts
!pkg/.gitkeep
"#;
    
    fs::write(dir.join(".gitignore"), content)?;
    Ok(())
}
