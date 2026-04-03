use anyhow::Result;
use colored::*;
use std::fs;
use std::path::PathBuf;

pub fn create_ios_project(platforms_dir: &PathBuf, name: &str) -> Result<()> {
    let ios_dir = platforms_dir.join("ios");
    fs::create_dir_all(&ios_dir)?;
    
    // Create Swift files
    create_app_swift(&ios_dir, name)?;
    create_app_state_swift(&ios_dir)?;
    create_content_view_swift(&ios_dir)?;
    
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

fn create_app_state_swift(dir: &PathBuf) -> Result<()> {
    let content = r#"import SwiftUI

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
"#;
    
    fs::write(dir.join("AppState.swift"), content)?;
    Ok(())
}

fn create_content_view_swift(dir: &PathBuf) -> Result<()> {
    let content = r#"import SwiftUI

struct ContentView: View {
    @EnvironmentObject var appState: AppState
    @State private var newItemTitle = ""
    @State private var showingAddItem = false
    
    var body: some View {
        NavigationView {
            List {
                ForEach(appState.items, id: \.id) { item in
                    HStack {
                        Image(systemName: item.completed ? "checkmark.circle.fill" : "circle")
                            .foregroundColor(item.completed ? .green : .gray)
                            .onTapGesture {
                                appState.toggleItem(id: item.id)
                            }
                        
                        Text(item.title)
                            .strikethrough(item.completed)
                    }
                }
                .onDelete { indexSet in
                    for index in indexSet {
                        appState.deleteItem(id: appState.items[index].id)
                    }
                }
            }
            .navigationTitle("Items")
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button(action: { showingAddItem = true }) {
                        Image(systemName: "plus")
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

struct AddItemView: View {
    @EnvironmentObject var appState: AppState
    @Binding var isPresented: Bool
    @State private var title = ""
    
    var body: some View {
        NavigationView {
            Form {
                TextField("Title", text: $title)
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
