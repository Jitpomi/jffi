use anyhow::Result;
use colored::*;
use std::fs;
use std::path::PathBuf;

pub fn create_macos_project(platforms_dir: &PathBuf, name: &str, template: &str) -> Result<()> {
    let macos_dir = platforms_dir.join("macos");
    fs::create_dir_all(&macos_dir)?;
    
    // Create Swift files
    create_app_swift(&macos_dir, name)?;
    create_app_state_swift(&macos_dir, template)?;
    create_content_view_swift(&macos_dir, template)?;
    
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

fn create_app_state_swift(dir: &PathBuf, _template: &str) -> Result<()> {
    let content = r#"import SwiftUI

class AppState: ObservableObject {
    @Published var greeting: String = ""
    let core: Core

    init() {
        let core = Core()
        self.core = core
        self.greeting = core.greeting()
    }
}
"#;
    
    fs::write(dir.join("AppState.swift"), content)?;
    Ok(())
}

fn create_content_view_swift(dir: &PathBuf, _template: &str) -> Result<()> {
    let content = r#"import SwiftUI

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
                appState.greeting = appState.core.greeting()
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

#import "{}_coreFFI.h"

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
