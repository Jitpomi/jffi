use anyhow::Result;
use std::path::Path;

/// Generate hot reload support files for the platform
pub fn generate_hotreload_support(platform: &str, project_name: &str, platform_dir: &Path) -> Result<()> {
    match platform {
        "ios" => generate_ios_hotreload(project_name, platform_dir),
        _ => Ok(()), // Other platforms don't have hot reload yet
    }
}

fn generate_ios_hotreload(project_name: &str, ios_dir: &Path) -> Result<()> {
    use std::fs;
    
    let module_name = project_name.replace("-", "_");
    
    // Generate HotReloadManager.swift
    let hotreload_manager = format!(r#"//
//  HotReloadManager.swift
//  Hot Reload Support for JFFI
//

import Foundation
import Combine

class HotReloadManager: ObservableObject {{
    static let shared = HotReloadManager()
    
    @Published var reloadTrigger = UUID()
    private var fileWatcher: DispatchSourceFileSystemObject?
    private var dylibHandle: UnsafeMutableRawPointer?
    
    private let dylibPath: String
    private let stateFile: String
    
    private init() {{
        // Get the dylib path from the project root
        let projectRoot = FileManager.default.currentDirectoryPath
        self.dylibPath = "\(projectRoot)/../../target/aarch64-apple-ios-sim/debug/lib{}_ffi.dylib"
        self.stateFile = "\(projectRoot)/../../.jffi/hotreload_state.json"
        
        setupFileWatcher()
        loadDylib()
    }}
    
    private func setupFileWatcher() {{
        guard let url = URL(string: dylibPath) else {{ return }}
        
        let descriptor = open(url.path, O_EVTONLY)
        guard descriptor >= 0 else {{ return }}
        
        let source = DispatchSource.makeFileSystemObjectSource(
            fileDescriptor: descriptor,
            eventMask: .write,
            queue: DispatchQueue.main
        )
        
        source.setEventHandler {{ [weak self] in
            self?.reloadDylib()
        }}
        
        source.setCancelHandler {{
            close(descriptor)
        }}
        
        source.resume()
        self.fileWatcher = source
    }}
    
    private func loadDylib() {{
        guard let handle = dlopen(dylibPath, RTLD_NOW | RTLD_LOCAL) else {{
            print("Failed to load dylib: \(String(cString: dlerror()))")
            return
        }}
        self.dylibHandle = handle
        print("✓ Loaded dylib")
    }}
    
    private func reloadDylib() {{
        print("🔄 Hot reloading...")
        
        // Unload old dylib
        if let handle = dylibHandle {{
            dlclose(handle)
        }}
        
        // Small delay to ensure file write is complete
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {{ [weak self] in
            self?.loadDylib()
            
            // Trigger UI refresh
            self?.reloadTrigger = UUID()
            
            print("✓ Hot reload complete!")
        }}
    }}
    
    deinit {{
        fileWatcher?.cancel()
        if let handle = dylibHandle {{
            dlclose(handle)
        }}
    }}
}}

// Extension to make any view hot-reloadable
extension View {{
    func hotReloadable() -> some View {{
        self.id(HotReloadManager.shared.reloadTrigger)
    }}
}}
"#, module_name);
    
    fs::write(ios_dir.join("HotReloadManager.swift"), hotreload_manager)?;
    
    Ok(())
}
