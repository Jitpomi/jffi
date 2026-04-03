use anyhow::Result;
use colored::*;
use std::fs;
use std::path::PathBuf;

pub fn create_macos_project(platforms_dir: &PathBuf, name: &str) -> Result<()> {
    let macos_dir = platforms_dir.join("macos");
    fs::create_dir_all(&macos_dir)?;
    
    // Create Swift files
    create_app_swift(&macos_dir, name)?;
    create_app_state_swift(&macos_dir)?;
    create_content_view_swift(&macos_dir)?;
    
    // Create Info.plist
    create_info_plist(&macos_dir, name)?;
    
    // Create Assets.xcassets
    create_assets_catalog(&macos_dir)?;
    
    // Create BridgingHeader
    create_bridging_header(&macos_dir, name)?;
    
    // Create Xcode project
    let project_root = platforms_dir.parent().unwrap();
    crate::xcode::create_macos_xcode_project(project_root, name)?;
    
    println!("  {} platforms/macos/", "✓".green());
    Ok(())
}

fn create_app_swift(dir: &PathBuf, name: &str) -> Result<()> {
    let app_name = to_pascal_case(name);
    let content = format!(r#"import SwiftUI

@main
struct {}App: App {{
    @StateObject private var appState = AppState()
    
    var body: some Scene {{
        WindowGroup {{
            ContentView()
                .environmentObject(appState)
                .frame(minWidth: 600, minHeight: 400)
        }}
        .commands {{
            CommandGroup(replacing: .newItem) {{}}
        }}
    }}
}}
"#, app_name);
    
    fs::write(dir.join(format!("{}App.swift", app_name)), content)?;
    Ok(())
}

fn create_app_state_swift(dir: &PathBuf) -> Result<()> {
    let content = r#"import SwiftUI
import Combine

class AppState: ObservableObject {
    @Published var items: [ItemViewModel] = []
    private let ffiApp: FfiApp
    
    init() {
        self.ffiApp = FfiApp()
        self.items = ffiApp.getItems()
    }
    
    func addItem(title: String) {
        let id = UUID().uuidString
        self.items = ffiApp.addItem(id: id, title: title)
    }
    
    func toggleItem(id: String) {
        self.items = ffiApp.toggleItem(id: id)
    }
    
    func deleteItem(id: String) {
        self.items = ffiApp.deleteItem(id: id)
    }
}

// Make ItemViewModel conform to Identifiable
extension ItemViewModel: Identifiable {}
"#;
    
    fs::write(dir.join("AppState.swift"), content)?;
    Ok(())
}

fn create_content_view_swift(dir: &PathBuf) -> Result<()> {
    let content = r#"import SwiftUI

struct ContentView: View {
    @EnvironmentObject var appState: AppState
    @State private var showingAddItem = false
    
    var body: some View {
        ScrollView {
            VStack(spacing: 24) {
                // Header
                HStack {
                    Text("Today")
                        .font(.system(size: 28, weight: .bold))
                    Spacer()
                    Button(action: { showingAddItem = true }) {
                        Image(systemName: "plus.circle.fill")
                            .font(.title2)
                            .foregroundColor(.blue)
                    }
                    .buttonStyle(.plain)
                }
                .padding(.horizontal, 20)
                .padding(.top, 20)
                
                // Stats Cards
                HStack(spacing: 12) {
                    StatCard(
                        title: "Total",
                        count: appState.items.count,
                        color: .blue
                    )
                    StatCard(
                        title: "Active",
                        count: appState.items.filter { !$0.completed }.count,
                        color: .orange
                    )
                    StatCard(
                        title: "Done",
                        count: appState.items.filter { $0.completed }.count,
                        color: .green
                    )
                }
                .padding(.horizontal, 20)
                
                // Tasks
                VStack(alignment: .leading, spacing: 12) {
                    Text("Tasks")
                        .font(.title3)
                        .fontWeight(.semibold)
                        .padding(.horizontal, 20)
                    
                    if appState.items.isEmpty {
                        EmptyStateView()
                    } else {
                        VStack(spacing: 8) {
                            ForEach(appState.items) { item in
                                TaskRow(item: item) {
                                    withAnimation(.spring(response: 0.3)) {
                                        appState.toggleItem(id: item.id)
                                    }
                                }
                            }
                        }
                        .padding(.horizontal, 20)
                    }
                }
            }
            .padding(.bottom, 20)
        }
        .background(Color(NSColor.windowBackgroundColor))
        .frame(minWidth: 600, minHeight: 450)
        .sheet(isPresented: $showingAddItem) {
            AddItemView(isPresented: $showingAddItem)
                .environmentObject(appState)
        }
    }
}

struct StatCard: View {
    let title: String
    let count: Int
    let color: Color
    
    var body: some View {
        VStack(spacing: 8) {
            Text("\(count)")
                .font(.system(size: 28, weight: .bold))
                .foregroundColor(color)
            Text(title)
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 16)
        .background(
            RoundedRectangle(cornerRadius: 16)
                .fill(Color(NSColor.controlBackgroundColor))
                .shadow(color: color.opacity(0.1), radius: 8, x: 0, y: 4)
        )
    }
}

struct TaskRow: View {
    let item: ItemViewModel
    let onToggle: () -> Void
    
    var body: some View {
        Button(action: onToggle) {
            HStack(spacing: 16) {
                ZStack {
                    Circle()
                        .stroke(item.completed ? Color.green : Color.gray.opacity(0.3), lineWidth: 2)
                        .frame(width: 24, height: 24)
                    
                    if item.completed {
                        Image(systemName: "checkmark")
                            .font(.system(size: 12, weight: .bold))
                            .foregroundColor(.green)
                    }
                }
                
                Text(item.title)
                    .font(.body)
                    .strikethrough(item.completed)
                    .foregroundColor(item.completed ? .secondary : .primary)
                
                Spacer()
            }
            .padding()
            .background(
                RoundedRectangle(cornerRadius: 12)
                    .fill(Color(NSColor.controlBackgroundColor))
            )
            .opacity(item.completed ? 0.6 : 1.0)
        }
        .buttonStyle(.plain)
    }
}

struct EmptyStateView: View {
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "checkmark.circle")
                .font(.system(size: 60))
                .foregroundColor(.gray.opacity(0.3))
            
            Text("No tasks yet")
                .font(.title3)
                .fontWeight(.medium)
                .foregroundColor(.secondary)
            
            Text("Click + to add your first task")
                .font(.subheadline)
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 60)
    }
}

struct AddItemView: View {
    @EnvironmentObject var appState: AppState
    @Binding var isPresented: Bool
    @State private var title = ""
    @FocusState private var isFocused: Bool
    
    var body: some View {
        VStack(spacing: 20) {
            // Title
            Text("New Task")
                .font(.title2)
                .fontWeight(.semibold)
            
            // Input field
            VStack(alignment: .leading, spacing: 8) {
                Text("Task Name")
                    .font(.subheadline)
                    .foregroundColor(.secondary)
                
                TextField("Enter task name", text: $title)
                    .textFieldStyle(.roundedBorder)
                    .focused($isFocused)
                    .onSubmit {
                        if !title.isEmpty {
                            appState.addItem(title: title)
                            isPresented = false
                        }
                    }
            }
            
            // Buttons
            HStack(spacing: 12) {
                Button("Cancel") {
                    isPresented = false
                }
                .keyboardShortcut(.cancelAction)
                .controlSize(.large)
                
                Button("Add Task") {
                    appState.addItem(title: title)
                    isPresented = false
                }
                .keyboardShortcut(.defaultAction)
                .disabled(title.isEmpty)
                .buttonStyle(.borderedProminent)
                .controlSize(.large)
            }
        }
        .padding(24)
        .frame(width: 400)
        .onAppear {
            isFocused = true
        }
    }
}

#Preview {
    ContentView()
        .environmentObject(AppState())
}
"#;
    
    fs::write(dir.join("ContentView.swift"), content)?;
    Ok(())
}

fn create_info_plist(dir: &PathBuf, name: &str) -> Result<()> {
    let content = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>$(DEVELOPMENT_LANGUAGE)</string>
    <key>CFBundleDisplayName</key>
    <string>{}</string>
    <key>CFBundleExecutable</key>
    <string>$(EXECUTABLE_NAME)</string>
    <key>CFBundleIdentifier</key>
    <string>$(PRODUCT_BUNDLE_IDENTIFIER)</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>$(PRODUCT_NAME)</string>
    <key>CFBundlePackageType</key>
    <string>$(PRODUCT_BUNDLE_PACKAGE_TYPE)</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSMinimumSystemVersion</key>
    <string>13.0</string>
    <key>NSHumanReadableCopyright</key>
    <string></string>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
</dict>
</plist>
"#, name);
    
    fs::write(dir.join("Info.plist"), content)?;
    Ok(())
}

fn create_assets_catalog(dir: &PathBuf) -> Result<()> {
    let assets_dir = dir.join("Assets.xcassets");
    fs::create_dir_all(&assets_dir)?;
    
    let contents = r#"{
  "info" : {
    "author" : "xcode",
    "version" : 1
  }
}
"#;
    
    fs::write(assets_dir.join("Contents.json"), contents)?;
    
    let appicon_dir = assets_dir.join("AppIcon.appiconset");
    fs::create_dir_all(&appicon_dir)?;
    
    let appicon_contents = r#"{
  "images" : [
    {
      "idiom" : "mac",
      "scale" : "1x",
      "size" : "16x16"
    },
    {
      "idiom" : "mac",
      "scale" : "2x",
      "size" : "16x16"
    },
    {
      "idiom" : "mac",
      "scale" : "1x",
      "size" : "32x32"
    },
    {
      "idiom" : "mac",
      "scale" : "2x",
      "size" : "32x32"
    },
    {
      "idiom" : "mac",
      "scale" : "1x",
      "size" : "128x128"
    },
    {
      "idiom" : "mac",
      "scale" : "2x",
      "size" : "128x128"
    },
    {
      "idiom" : "mac",
      "scale" : "1x",
      "size" : "256x256"
    },
    {
      "idiom" : "mac",
      "scale" : "2x",
      "size" : "256x256"
    },
    {
      "idiom" : "mac",
      "scale" : "1x",
      "size" : "512x512"
    },
    {
      "idiom" : "mac",
      "scale" : "2x",
      "size" : "512x512"
    }
  ],
  "info" : {
    "author" : "xcode",
    "version" : 1
  }
}
"#;
    
    fs::write(appicon_dir.join("Contents.json"), appicon_contents)?;
    
    Ok(())
}

fn create_bridging_header(dir: &PathBuf, name: &str) -> Result<()> {
    let module_name = name.replace("-", "_");
    let content = format!(r#"//
//  BridgingHeader.h
//  Bridging header for Rust FFI
//

#ifndef BridgingHeader_h
#define BridgingHeader_h

#import "{}_ffiFFI.h"

#endif /* BridgingHeader_h */
"#, module_name);
    
    fs::write(dir.join("BridgingHeader.h"), content)?;
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
