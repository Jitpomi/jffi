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
    create_ffi_wrapper_py(&linux_dir, name)?;
    
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
from gi.repository import Gtk, Adw, GLib

from ffi_wrapper import FfiWrapper

class {}(Adw.ApplicationWindow):
    def __init__(self, **kwargs):"#, window_class);
    let content = content + r#"
        super().__init__(**kwargs)
        
        self.ffi = FfiWrapper()
        self.items = []
        
        self.set_title("Today")
        self.set_default_size(600, 450)
        
        # Create header bar
        header = Adw.HeaderBar()
        add_button = Gtk.Button(icon_name="list-add-symbolic")
        add_button.connect("clicked", self.on_add_clicked)
        header.pack_end(add_button)
        
        # Create main content
        main_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        main_box.append(header)
        
        # Stats cards
        stats_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        stats_box.set_margin_start(20)
        stats_box.set_margin_end(20)
        stats_box.set_margin_top(20)
        stats_box.set_margin_bottom(20)
        stats_box.set_homogeneous(True)
        
        self.total_label = self.create_stat_card("Total", "0")
        self.active_label = self.create_stat_card("Active", "0")
        self.done_label = self.create_stat_card("Done", "0")
        
        stats_box.append(self.total_label)
        stats_box.append(self.active_label)
        stats_box.append(self.done_label)
        
        main_box.append(stats_box)
        
        # Tasks list
        scrolled = Gtk.ScrolledWindow()
        scrolled.set_vexpand(True)
        
        self.list_box = Gtk.ListBox()
        self.list_box.set_margin_start(20)
        self.list_box.set_margin_end(20)
        self.list_box.set_margin_bottom(20)
        self.list_box.add_css_class("boxed-list")
        
        scrolled.set_child(self.list_box)
        main_box.append(scrolled)
        
        self.set_content(main_box)
        self.refresh_items()
    
    def create_stat_card(self, title, value):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        box.set_margin_top(16)
        box.set_margin_bottom(16)
        
        value_label = Gtk.Label(label=value)
        value_label.add_css_class("title-1")
        
        title_label = Gtk.Label(label=title)
        title_label.add_css_class("dim-label")
        
        box.append(value_label)
        box.append(title_label)
        
        frame = Gtk.Frame()
        frame.set_child(box)
        
        return box
    
    def refresh_items(self):
        self.items = self.ffi.get_items()
        
        # Update stats
        total = len(self.items)
        active = sum(1 for item in self.items if not item['completed'])
        done = sum(1 for item in self.items if item['completed'])
        
        self.total_label.get_first_child().set_label(str(total))
        self.active_label.get_first_child().set_label(str(active))
        self.done_label.get_first_child().set_label(str(done))
        
        # Clear and rebuild list
        while True:
            row = self.list_box.get_row_at_index(0)
            if row is None:
                break
            self.list_box.remove(row)
        
        for item in self.items:
            row = self.create_task_row(item)
            self.list_box.append(row)
    
    def create_task_row(self, item):
        row = Gtk.ListBoxRow()
        
        box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        box.set_margin_start(12)
        box.set_margin_end(12)
        box.set_margin_top(12)
        box.set_margin_bottom(12)
        
        # Checkbox
        check = Gtk.CheckButton()
        check.set_active(item['completed'])
        check.connect("toggled", self.on_toggle_clicked, item['id'])
        
        # Title
        label = Gtk.Label(label=item['title'])
        label.set_hexpand(True)
        label.set_halign(Gtk.Align.START)
        
        if item['completed']:
            label.add_css_class("dim-label")
        
        box.append(check)
        box.append(label)
        
        row.set_child(box)
        return row
    
    def on_add_clicked(self, button):
        dialog = Adw.MessageDialog(
            transient_for=self,
            heading="New Task",
            body="Enter task name:"
        )
        
        entry = Gtk.Entry()
        entry.set_margin_start(12)
        entry.set_margin_end(12)
        entry.set_margin_top(12)
        entry.set_margin_bottom(12)
        
        dialog.set_extra_child(entry)
        dialog.add_response("cancel", "Cancel")
        dialog.add_response("add", "Add")
        dialog.set_response_appearance("add", Adw.ResponseAppearance.SUGGESTED)
        
        def on_response(dialog, response):
            if response == "add":
                title = entry.get_text()
                if title:
                    import uuid
                    self.ffi.add_item(str(uuid.uuid4()), title)
                    self.refresh_items()
        
        dialog.connect("response", on_response)
        dialog.present()
    
    def on_toggle_clicked(self, check, item_id):
        self.ffi.toggle_item(item_id)
        self.refresh_items()
"#;
    
    fs::write(dir.join("window.py"), content)?;
    Ok(())
}

fn create_ffi_wrapper_py(dir: &PathBuf, name: &str) -> Result<()> {
    let module_name = name.replace("-", "_");
    let content = format!(r#"from {}_ffi import FfiApp

class FfiWrapper:
    """Wrapper for Rust FFI bindings"""
    
    def __init__(self):
        self.ffi_app = FfiApp()
    
    def get_items(self):
        items = self.ffi_app.get_items()
        return [{{'id': item.id, 'title': item.title, 'completed': item.completed}} for item in items]
    
    def add_item(self, id, title):
        self.ffi_app.add_item(id, title)
    
    def toggle_item(self, id):
        self.ffi_app.toggle_item(id)
    
    def delete_item(self, id):
        self.ffi_app.delete_item(id)
"#, module_name);
    
    fs::write(dir.join("ffi_wrapper.py"), content)?;
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
