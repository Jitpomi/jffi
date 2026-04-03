# JFFI

A cross-platform framework for building native applications with Rust business logic and platform-native UIs.

## 🎯 Philosophy

**Write your business logic once in Rust. Build native UIs for each platform.**

- Write business logic once in Rust
- Use native UI frameworks (SwiftUI, Jetpack Compose, WinUI, etc.)
- Get truly native performance and platform feel
- Maintain type safety end-to-end via UniFFI

## 🚀 Quick Start

### Installation

```bash
# Install the CLI tool
cargo install --path cli
```

### Create Your First App

```bash
# Create a new app with iOS support
jffi new my-app --platforms ios

# Navigate and run
cd my-app
jffi run --platform ios
```

That's it! The app builds, compiles Rust, generates Swift bindings, and launches in the iOS Simulator automatically.

## 📱 Supported Platforms

| Platform | Status | UI Framework | Language |
|----------|--------|--------------|----------|
| iOS | ✅ Ready | SwiftUI | Swift |
| Android | 🚧 Coming Soon | Jetpack Compose | Kotlin |
| macOS | 🚧 Coming Soon | SwiftUI | Swift |
| Windows | 🚧 Coming Soon | WinUI 3 | C# |
| Linux | 🚧 Coming Soon | GTK 4 | C/Python |
| Web | 🚧 Coming Soon | HTML/JS | JavaScript |

## 🏗️ Project Structure

```
my-app/
├── core/                    # Pure Rust business logic
│   ├── src/lib.rs          # Your app logic here
│   └── Cargo.toml
│
├── ffi/                     # FFI layer (auto-scaffolded)
│   ├── src/lib.rs          # UniFFI exports
│   └── Cargo.toml
│
├── platforms/
│   └── ios/                # iOS SwiftUI app
│       ├── *App.swift
│       ├── AppState.swift
│       ├── ContentView.swift
│       └── *.xcodeproj     # Auto-generated
│
└── jffi.toml               # Framework configuration
```

## 💡 Development Workflow

### 1. Write Business Logic (Once)

`core/src/lib.rs`:

```rust
pub struct App {
    items: Vec<Item>,
}

impl App {
    pub fn add_item(&mut self, id: String, title: String) {
        self.items.push(Item { id, title, completed: false });
    }
}
```

### 2. Expose via FFI (Auto-scaffolded)

`ffi/src/lib.rs`:

```rust
#[derive(uniffi::Object)]
pub struct FfiApp {
    app: Mutex<App>,
}

#[uniffi::export]
impl FfiApp {
    pub fn add_item(&self, id: String, title: String) -> Vec<ItemViewModel> {
        let mut app = self.app.lock().unwrap();
        app.add_item(id, title);
        app.get_items().iter().map(ItemViewModel::from).collect()
    }
}
```

### 3. Build & Run

```bash
jffi run --platform ios
```

This automatically:
- Compiles Rust for iOS Simulator
- Generates Swift bindings via UniFFI
- Creates Xcode project
- Builds with xcodebuild
- Launches in iOS Simulator

### 4. Use in Native UI

`platforms/ios/ContentView.swift`:

```swift
Button("Add Item") {
    appState.addItem(id: UUID().uuidString, title: newItem)
}
```

The generated bindings make Rust functions available in Swift!

## ⚡ Hot Reload

JFFI includes **true hot reload** - your Rust changes appear instantly without restarting the app or losing state.

```bash
# Start hot reload mode
jffi dev --platform ios
```

**Edit any Rust file** → Changes appear in < 2 seconds! ⚡

### How It Works

- **Dynamic dylib reloading** - iOS app reloads the Rust library without restarting
- **File watching** - Automatically detects changes in `core/` and `ffi/`
- **State preservation** - App state is maintained across reloads
- **Instant UI refresh** - SwiftUI views update automatically

### What Gets Hot Reloaded

✅ **Instantly reloaded:**
- Rust business logic changes
- Function implementations
- Data structure modifications

❌ **Requires restart:**
- New FFI functions
- FFI signature changes
- SwiftUI changes

## 🔧 CLI Commands

```bash
# Create new project
jffi new <name> --platforms <platforms>

# Build for platform
jffi build --platform <platform>

# Run on platform (builds automatically)
jffi run --platform <platform>

# Development mode (auto-rebuild on changes)
jffi dev --platform <platform>

# Add platform to existing project
jffi add <platform>

# List available platforms
jffi platforms
```

## 🔄 How It Works

```
┌─────────────────────────────────────────┐
│     Your Rust Business Logic           │
│     (core/src/lib.rs)                   │
└─────────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────────┐
│     UniFFI FFI Layer                    │
│     (ffi/src/lib.rs)                    │
└─────────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────────┐
│     Auto-Generated Bindings             │
│     (Swift, Kotlin, C#, etc.)           │
└─────────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────────┐
│     Native Platform UI                  │
│     (SwiftUI, Compose, WinUI, etc.)     │
└─────────────────────────────────────────┘
```

## 📚 Configuration

`jffi.toml`:

```toml
[package]
name = "my-app"
version = "0.1.0"

[platforms]
enabled = ["ios"]

[platforms.ios]
deployment_target = "16.0"
bundle_id = "com.example.myapp"
```

## 🆚 Why JFFI?

| Feature | JFFI | Flutter | React Native |
|---------|------|---------|--------------|
| Business Logic | Rust | Dart | JavaScript |
| UI | Native | Cross-platform | Near-native |
| Performance | Native | Good | Good |
| Platform Feel | Native | Consistent | Near-native |
| Type Safety | End-to-end | Strong | Weak |

**Use JFFI when:**
- You want truly native UI and performance
- You have platform-specific design requirements
- You want to leverage native UI libraries
- You need Rust for business logic (performance, safety)

## 🛠️ Development

### Prerequisites

- Rust toolchain
- **iOS**: Xcode, iOS Simulator
- **Android**: Android Studio, NDK (coming soon)
- **macOS**: Xcode (coming soon)
- **Windows**: Visual Studio (coming soon)
- **Linux**: GTK libraries (coming soon)
- **Web**: wasm-pack (coming soon)

### Building the CLI

```bash
cargo build --package jffi
cargo run --package jffi -- --help
```

## 🗺️ Roadmap

- [x] CLI tool foundation
- [x] iOS support with SwiftUI (fully working!)
- [x] Automatic Xcode project generation
- [x] One-command build and run
- [x] Hot reload for iOS (true hot reload with state preservation!)
- [ ] Android support with Kotlin
- [ ] macOS support
- [ ] Windows support with C#
- [ ] Linux support with GTK
- [ ] Web support with WASM
- [ ] Hot reload for other platforms

## 🤝 Contributing

Early-stage framework. Contributions welcome!

**High priority:**
- Android template with Kotlin bindings
- macOS template
- Windows template with C# bindings
- Linux GTK template
- Web WASM integration

## 📄 License

MIT

## 🙏 Acknowledgments

- [UniFFI](https://github.com/mozilla/uniffi-rs) - FFI bindings generator
- The Rust community

---

**Built with ❤️ and Rust**
