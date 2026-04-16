use anyhow::Result;
use colored::*;
use std::fs;
use std::path::PathBuf;

pub fn create_ios_project(platforms_dir: &PathBuf, name: &str, template: &str) -> Result<()> {
    let ios_dir = platforms_dir.join("ios");
    fs::create_dir_all(&ios_dir)?;
    
    // Create Swift files
    create_app_swift(&ios_dir, name)?;
    create_app_state_swift(&ios_dir, template)?;
    create_content_view_swift(&ios_dir, template)?;
    
    // Create Info.plist
    create_info_plist(&ios_dir, name)?;
    
    // Create Assets.xcassets
    create_assets_catalog(&ios_dir)?;
    
    // Create BridgingHeader
    create_bridging_header(&ios_dir, name)?;
    
    // Create Xcode project
    let project_root = platforms_dir.parent().unwrap();
    crate::xcode::create_xcode_project(project_root, name)?;
    
    println!("  {} platforms/ios/", "✓".green());
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
        }}
    }}
}}
"#, app_name);
    
    fs::write(dir.join(format!("{}App.swift", app_name)), content)?;
    Ok(())
}

fn create_app_state_swift(dir: &PathBuf, template: &str) -> Result<()> {
    let content = match template {
        "hello" => r#"import SwiftUI

class AppState: ObservableObject {
    @Published var greeting: String = ""
    private let ffiApp: FfiApp?

    init() {
        if ProcessInfo.processInfo.environment["XCODE_RUNNING_FOR_PREVIEWS"] == "1" {
            self.ffiApp = nil
            self.greeting = "Hello from Preview"
            return
        }

        let app = FfiApp()
        self.ffiApp = app
        self.greeting = app.greeting()
    }

    func refresh() {
        guard let ffiApp = ffiApp else {
            self.greeting = "Hello from Preview"
            return
        }
        self.greeting = ffiApp.greeting()
    }
}
"#,
        "counter" => r#"import SwiftUI

class AppState: ObservableObject {
    @Published var count: Int64 = 0
    private let ffiApp: FfiApp?

    init() {
        if ProcessInfo.processInfo.environment["XCODE_RUNNING_FOR_PREVIEWS"] == "1" {
            self.ffiApp = nil
            self.count = 42
            return
        }

        let app = FfiApp()
        self.ffiApp = app
        self.count = app.get()
    }

    func inc() {
        guard let ffiApp = ffiApp else {
            self.count += 1
            return
        }
        self.count = ffiApp.inc()
    }

    func dec() {
        guard let ffiApp = ffiApp else {
            self.count -= 1
            return
        }
        self.count = ffiApp.dec()
    }

    func reset() {
        guard let ffiApp = ffiApp else {
            self.count = 0
            return
        }
        self.count = ffiApp.reset()
    }
}
"#,
        _ => r#"import SwiftUI

class AppState: ObservableObject {
    @Published var items: [ItemViewModel] = []
    private let ffiApp: FfiApp?
    
    init() {
        if ProcessInfo.processInfo.environment["XCODE_RUNNING_FOR_PREVIEWS"] == "1" {
            self.ffiApp = nil
            self.items = []
            return
        }

        let app = FfiApp()
        self.ffiApp = app
        self.items = app.getItems()
    }
    
    func addItem(title: String) {
        guard let ffiApp = ffiApp else { return }
        let id = UUID().uuidString
        self.items = ffiApp.addItem(id: id, title: title)
    }
    
    func toggleItem(id: String) {
        guard let ffiApp = ffiApp else { return }
        self.items = ffiApp.toggleItem(id: id)
    }
    
    func deleteItem(id: String) {
        guard let ffiApp = ffiApp else { return }
        self.items = ffiApp.deleteItem(id: id)
    }
}

extension ItemViewModel: Identifiable {}
"#,
    };
    
    fs::write(dir.join("AppState.swift"), content)?;
    Ok(())
}

fn create_content_view_swift(dir: &PathBuf, template: &str) -> Result<()> {
    let content = match template {
        "hello" => r#"import SwiftUI

struct ContentView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        VStack(spacing: 16) {
            Text(appState.greeting)
                .font(.title)
                .fontWeight(.semibold)
                .multilineTextAlignment(.center)
                .padding(.horizontal)

            Button("Refresh") {
                appState.refresh()
            }
            .buttonStyle(.borderedProminent)
        }
        .padding()
    }
}

#Preview {
    ContentView()
        .environmentObject(AppState())
}
"#,
        "counter" => r#"import SwiftUI

struct ContentView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        VStack(spacing: 24) {
            Text("Counter")
                .font(.largeTitle)
                .fontWeight(.bold)

            Text("\(appState.count)")
                .font(.system(size: 64, weight: .bold, design: .rounded))

            HStack(spacing: 12) {
                Button("-") { appState.dec() }
                    .buttonStyle(.borderedProminent)

                Button("Reset") { appState.reset() }
                    .buttonStyle(.bordered)

                Button("+") { appState.inc() }
                    .buttonStyle(.borderedProminent)
            }
        }
        .padding()
    }
}

#Preview {
    ContentView()
        .environmentObject(AppState())
}
"#,
        _ => r#"import SwiftUI

struct ContentView: View {
    @EnvironmentObject var appState: AppState
    @State private var showingAddItem = false
    
    var body: some View {
        NavigationView {
            ScrollView {
                VStack(spacing: 24) {
                    // Header Stats
                    HStack(spacing: 16) {
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
                    .padding(.horizontal)
                    
                    // Tasks List
                    VStack(alignment: .leading, spacing: 12) {
                        Text("Tasks")
                            .font(.title2)
                            .fontWeight(.bold)
                            .padding(.horizontal)
                        
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
                            .padding(.horizontal)
                        }
                    }
                }
                .padding(.vertical)
            }
            .background(Color(.systemGroupedBackground))
            .navigationTitle("Today")
            .navigationBarTitleDisplayMode(.large)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button(action: { showingAddItem = true }) {
                        Image(systemName: "plus.circle.fill")
                            .font(.title2)
                            .foregroundStyle(.blue)
                    }
                }
            }
            .sheet(isPresented: $showingAddItem) {
                AddItemView(isPresented: $showingAddItem)
                    .environmentObject(appState)
            }
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
                .fill(Color(.systemBackground))
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
                    .fill(Color(.systemBackground))
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
            
            Text("Tap + to add your first task")
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
        NavigationView {
            Form {
                TextField("Title", text: $title)
                    .onSubmit {
                        if !title.isEmpty {
                            appState.addItem(title: title)
                            isPresented = false
                        }
                    }
            }
            .navigationTitle("New Item")
            .navigationBarItems(
                leading: Button("Cancel") {
                    isPresented = false
                },
                trailing: Button("Add") {
                    appState.addItem(title: title)
                    isPresented = false
                }
                .disabled(title.isEmpty)
            )
        }
    }
}

#Preview {
    ContentView()
        .environmentObject(AppState())
}
"#,
    };
    
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
    <key>LSRequiresIPhoneOS</key>
    <true/>
    <key>UIApplicationSceneManifest</key>
    <dict>
        <key>UIApplicationSupportsMultipleScenes</key>
        <false/>
    </dict>
    <key>UIApplicationSupportsIndirectInputEvents</key>
    <true/>
    <key>UILaunchScreen</key>
    <dict/>
    <key>UIRequiredDeviceCapabilities</key>
    <array>
        <string>armv7</string>
    </array>
    <key>UISupportedInterfaceOrientations</key>
    <array>
        <string>UIInterfaceOrientationPortrait</string>
        <string>UIInterfaceOrientationLandscapeLeft</string>
        <string>UIInterfaceOrientationLandscapeRight</string>
    </array>
    <key>UISupportedInterfaceOrientations~ipad</key>
    <array>
        <string>UIInterfaceOrientationPortrait</string>
        <string>UIInterfaceOrientationPortraitUpsideDown</string>
        <string>UIInterfaceOrientationLandscapeLeft</string>
        <string>UIInterfaceOrientationLandscapeRight</string>
    </array>
</dict>
</plist>
"#, name);
    
    fs::write(dir.join("Info.plist"), content)?;
    Ok(())
}

fn create_assets_catalog(dir: &PathBuf) -> Result<()> {
    let assets_dir = dir.join("Assets.xcassets");
    fs::create_dir_all(&assets_dir)?;
    
    // Contents.json
    let contents = r#"{
  "info" : {
    "author" : "xcode",
    "version" : 1
  }
}
"#;
    fs::write(assets_dir.join("Contents.json"), contents)?;
    
    // AppIcon
    let appicon_dir = assets_dir.join("AppIcon.appiconset");
    fs::create_dir_all(&appicon_dir)?;
    
    let appicon_contents = r#"{
  "images" : [
    {
      "idiom" : "iphone",
      "scale" : "2x",
      "size" : "20x20"
    },
    {
      "idiom" : "iphone",
      "scale" : "3x",
      "size" : "20x20"
    },
    {
      "idiom" : "iphone",
      "scale" : "2x",
      "size" : "29x29"
    },
    {
      "idiom" : "iphone",
      "scale" : "3x",
      "size" : "29x29"
    },
    {
      "idiom" : "iphone",
      "scale" : "2x",
      "size" : "40x40"
    },
    {
      "idiom" : "iphone",
      "scale" : "3x",
      "size" : "40x40"
    },
    {
      "idiom" : "iphone",
      "scale" : "2x",
      "size" : "60x60"
    },
    {
      "idiom" : "iphone",
      "scale" : "3x",
      "size" : "60x60"
    },
    {
      "idiom" : "ios-marketing",
      "scale" : "1x",
      "size" : "1024x1024"
    }
  ],
  "info" : {
    "author" : "xcode",
    "version" : 1
  }
}
"#;
    fs::write(appicon_dir.join("Contents.json"), appicon_contents)?;
    
    // AccentColor
    let accent_dir = assets_dir.join("AccentColor.colorset");
    fs::create_dir_all(&accent_dir)?;
    
    let accent_contents = r#"{
  "colors" : [
    {
      "idiom" : "universal"
    }
  ],
  "info" : {
    "author" : "xcode",
    "version" : 1
  }
}
"#;
    fs::write(accent_dir.join("Contents.json"), accent_contents)?;
    
    Ok(())
}

fn create_bridging_header(dir: &PathBuf, name: &str) -> Result<()> {
    let module_name = name.replace("-", "_");
    let content = format!(r#"//
//  BridgingHeader.h
//
//  Bridging header for Rust FFI library
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
