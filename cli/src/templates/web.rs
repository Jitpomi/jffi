use anyhow::Result;
use colored::*;
use std::fs;
use std::path::PathBuf;

pub fn create_web_project(platforms_dir: &PathBuf, name: &str) -> Result<()> {
    let web_dir = platforms_dir.join("web");
    fs::create_dir_all(&web_dir)?;
    
    // Create web application files
    create_index_html(&web_dir, name)?;
    create_main_js(&web_dir)?;
    create_styles_css(&web_dir)?;
    create_package_json(&web_dir, name)?;
    create_vite_config(&web_dir)?;
    
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
        <header>
            <h1>Today</h1>
            <button id="add-btn" class="btn-primary">+ Add Task</button>
        </header>
        
        <div class="stats">
            <div class="stat-card">
                <div class="stat-value" id="total-count">0</div>
                <div class="stat-label">Total</div>
            </div>
            <div class="stat-card">
                <div class="stat-value" id="active-count">0</div>
                <div class="stat-label">Active</div>
            </div>
            <div class="stat-card">
                <div class="stat-value" id="done-count">0</div>
                <div class="stat-label">Done</div>
            </div>
        </div>
        
        <div id="tasks-container" class="tasks-list"></div>
    </div>
    
    <div id="modal" class="modal hidden">
        <div class="modal-content">
            <h2>New Task</h2>
            <input type="text" id="task-input" placeholder="Enter task name..." />
            <div class="modal-actions">
                <button id="cancel-btn" class="btn-secondary">Cancel</button>
                <button id="submit-btn" class="btn-primary">Add</button>
            </div>
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
    let content = r#"import init, { FfiApp } from './pkg/wasm.js';

let app = null;

async function initApp() {
    await init();
    app = new FfiApp();
    render();
    setupEventListeners();
}

function render() {
    const items = app.get_items();
    renderStats(items);
    renderTasks(items);
}

function renderStats(items) {
    const total = items.length;
    const active = items.filter(item => !item.completed).length;
    const done = items.filter(item => item.completed).length;
    
    document.getElementById('total-count').textContent = total;
    document.getElementById('active-count').textContent = active;
    document.getElementById('done-count').textContent = done;
}

function renderTasks(items) {
    const container = document.getElementById('tasks-container');
    container.innerHTML = '';
    
    items.forEach(item => {
        const taskEl = document.createElement('div');
        taskEl.className = 'task-item';
        
        const checkbox = document.createElement('input');
        checkbox.type = 'checkbox';
        checkbox.checked = item.completed;
        checkbox.addEventListener('change', () => {
            app.toggle_item(item.id);
            render();
        });
        
        const label = document.createElement('span');
        label.textContent = item.title;
        label.className = item.completed ? 'completed' : '';
        
        taskEl.appendChild(checkbox);
        taskEl.appendChild(label);
        container.appendChild(taskEl);
    });
}

function setupEventListeners() {
    const addBtn = document.getElementById('add-btn');
    const modal = document.getElementById('modal');
    const cancelBtn = document.getElementById('cancel-btn');
    const submitBtn = document.getElementById('submit-btn');
    const taskInput = document.getElementById('task-input');
    
    addBtn.addEventListener('click', () => {
        modal.classList.remove('hidden');
        taskInput.focus();
    });
    
    cancelBtn.addEventListener('click', () => {
        modal.classList.add('hidden');
        taskInput.value = '';
    });
    
    submitBtn.addEventListener('click', () => {
        const title = taskInput.value.trim();
        if (title) {
            const id = crypto.randomUUID();
            app.add_item(id, title);
            render();
            modal.classList.add('hidden');
            taskInput.value = '';
        }
    });
    
    taskInput.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') {
            submitBtn.click();
        }
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
    padding: 20px;
}

#app {
    max-width: 600px;
    margin: 0 auto;
    background: white;
    border-radius: 16px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
    overflow: hidden;
}

header {
    background: white;
    color: #1d1d1f;
    padding: 24px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid #e5e5e7;
}

h1 {
    font-size: 28px;
    font-weight: 600;
}

.btn-primary {
    background: #007aff;
    color: white;
    border: none;
    padding: 10px 20px;
    border-radius: 8px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s;
}

.btn-primary:hover {
    background: #0051d5;
}

.btn-secondary {
    background: #e5e7eb;
    color: #374151;
    border: none;
    padding: 10px 20px;
    border-radius: 8px;
    font-weight: 600;
    cursor: pointer;
}

.stats {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 16px;
    padding: 24px;
    background: #fafafa;
}

.stat-card {
    background: white;
    padding: 20px;
    border-radius: 12px;
    text-align: center;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.stat-value {
    font-size: 32px;
    font-weight: 700;
    color: #1d1d1f;
}

.stat-label {
    font-size: 14px;
    color: #86868b;
    margin-top: 4px;
}

.tasks-list {
    padding: 24px;
}

.task-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px;
    background: #fafafa;
    border-radius: 8px;
    margin-bottom: 8px;
}

.task-item input[type="checkbox"] {
    width: 20px;
    height: 20px;
    cursor: pointer;
}

.task-item span {
    flex: 1;
    font-size: 16px;
}

.task-item span.completed {
    text-decoration: line-through;
    color: #9ca3af;
}

.modal {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
}

.modal.hidden {
    display: none;
}

.modal-content {
    background: white;
    padding: 24px;
    border-radius: 16px;
    width: 90%;
    max-width: 400px;
}

.modal-content h2 {
    margin-bottom: 16px;
    color: #111827;
}

.modal-content input {
    width: 100%;
    padding: 12px;
    border: 2px solid #e5e7eb;
    border-radius: 8px;
    font-size: 16px;
    margin-bottom: 16px;
}

.modal-content input:focus {
    outline: none;
    border-color: #007aff;
}

.modal-actions {
    display: flex;
    gap: 12px;
    justify-content: flex-end;
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
    open: true
  }
});
"#;
    
    fs::write(dir.join("vite.config.js"), content)?;
    Ok(())
}
