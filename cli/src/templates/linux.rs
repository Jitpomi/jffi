use anyhow::Result;
use colored::*;
use std::fs;
use std::path::PathBuf;

pub fn create_linux_project(platforms_dir: &PathBuf, name: &str) -> Result<()> {
    let linux_dir = platforms_dir.join("linux");
    fs::create_dir_all(&linux_dir)?;
    
    // Create Python application files
    create_main_py(&linux_dir, name)?;
    create_app_py(&linux_dir, name)?;
    create_window_py(&linux_dir, name)?;
    create_core_wrapper_py(&linux_dir, name)?;
    
    // Create requirements.txt
    create_requirements(&linux_dir)?;
    
    // Create setup script
    create_setup_script(&linux_dir)?;
    
    println!("  {} platforms/linux/", "✓".green());
    Ok(())
}

fn create_main_py(dir: &PathBuf, name: &str) -> Result<()> {
    let app_class = to_pascal_case(name);
    let content = format!(r#"#!/usr/bin/env python3
import sys
import gi

gi.require_version('Gtk', '4.0')
gi.require_version('Adw', '1')
from gi.repository import Gtk, Adw

from app import {}Application

def main():
    # Initialize GTK
    Gtk.init()
    
    app = {}Application()
    return app.run(sys.argv)

if __name__ == '__main__':
    sys.exit(main())
"#, app_class, app_class);
    
    fs::write(dir.join("main.py"), content)?;
    Ok(())
}

fn create_app_py(dir: &PathBuf, name: &str) -> Result<()> {
    let app_class = to_pascal_case(name);
    let app_id = format!("com.example.{}", name.replace("-", ""));
    let content = format!(r#"import gi

gi.require_version('Gtk', '4.0')
gi.require_version('Adw', '1')
from gi.repository import Gtk, Adw

from window import {}Window

class {}Application(Adw.Application):
    def __init__(self):
        super().__init__(application_id='{}')
        self.window = None
    
    def do_activate(self):
        if not self.window:
            self.window = {}Window(application=self)
        self.window.present()
"#, app_class, app_class, app_id, app_class);
    
    fs::write(dir.join("app.py"), content)?;
    Ok(())
}

fn create_window_py(dir: &PathBuf, name: &str) -> Result<()> {
    let window_class = to_pascal_case(name) + "Window";
    let content = format!(r#"import gi

gi.require_version('Gtk', '4.0')
gi.require_version('Adw', '1')
from gi.repository import Gtk, Adw

from core_wrapper import CoreWrapper

class {}(Adw.ApplicationWindow):
    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        
        self.core = CoreWrapper()
        
        self.set_title("Hello from JFFI")
        self.set_default_size(600, 400)
        
        # Create header bar
        header = Adw.HeaderBar()
        
        # Create main content
        main_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        main_box.append(header)
        
        # Center content
        center_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        center_box.set_vexpand(True)
        center_box.set_valign(Gtk.Align.CENTER)
        center_box.set_halign(Gtk.Align.CENTER)
        
        # Greeting label
        self.greeting_label = Gtk.Label()
        self.greeting_label.add_css_class("title-1")
        center_box.append(self.greeting_label)
        
        # Refresh button
        refresh_button = Gtk.Button(label="Refresh")
        refresh_button.add_css_class("suggested-action")
        refresh_button.connect("clicked", self.on_refresh_clicked)
        center_box.append(refresh_button)
        
        main_box.append(center_box)
        
        self.set_content(main_box)
        
        # Load initial greeting
        self.greeting_label.set_text(self.core.greeting())
    
    def on_refresh_clicked(self, button):
        self.greeting_label.set_text(self.core.greeting())
"#, window_class);
    
    fs::write(dir.join("window.py"), content)?;
    Ok(())
}

fn create_core_wrapper_py(dir: &PathBuf, name: &str) -> Result<()> {
    let module_name = name.replace("-", "_");
    let content = format!(r#"from {}_core import Core

class CoreWrapper:
    """Wrapper for Rust Core bindings"""
    
    def __init__(self):
        self.core = Core()
    
    def greeting(self):
        return self.core.greeting()
"#, module_name);
    
    fs::write(dir.join("core_wrapper.py"), content)?;
    Ok(())
}

fn create_requirements(dir: &PathBuf) -> Result<()> {
    let content = r#"PyGObject>=3.42.0
"#;
    
    fs::write(dir.join("requirements.txt"), content)?;
    Ok(())
}

fn create_setup_script(dir: &PathBuf) -> Result<()> {
    let content = r#"#!/bin/bash
# Setup script for Linux development

set -e

echo "Setting up Linux development environment..."

# Install build essentials (required for Rust compilation)
if ! command -v gcc &> /dev/null; then
    echo "Installing build essentials..."
    sudo apt update
    sudo apt install -y build-essential pkg-config
fi

# Check for Python 3
if ! command -v python3 &> /dev/null; then
    echo "Installing Python 3..."
    sudo apt install -y python3 python3-pip
fi

# Install GTK 4 and dependencies
if ! pkg-config --exists gtk4; then
    echo "Installing GTK 4..."
    sudo apt install -y libgtk-4-dev libadwaita-1-dev python3-gi python3-gi-cairo gir1.2-gtk-4.0 gir1.2-adw-1
fi

# Note: uniffi-bindgen-cli will be run via cargo from the ffi crate
# No separate installation needed - it's built into the uniffi dependency

# Install Python dependencies
echo "Installing Python dependencies..."
pip3 install --user -r requirements.txt

echo "Setup complete!"
"#;
    
    fs::write(dir.join("setup.sh"), content)?;
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(dir.join("setup.sh"))?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(dir.join("setup.sh"), perms)?;
    }
    
    Ok(())
}

fn to_pascal_case(s: &str) -> String {
    s.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}
