# JFFI

A cross-platform framework for building native applications with Rust business logic and platform-native UIs.

##  Philosophy

**Write your business logic once in Rust. Build native UIs for each platform.**

- Write business logic once in Rust
- Use native UI frameworks (SwiftUI, Jetpack Compose, WinUI, etc.)
- Get truly native performance and platform feel
- Maintain type safety end-to-end via UniFFI

##  Quick Start

### Installation

```bash
# Install JFFI CLI
cargo install --path cli
```

### Create Your First App

```bash
# Create a new app with platform support
jffi new my-app --platforms ios,macos,android,linux,windows

# Navigate and run
cd my-app
jffi run --platform ios      # iOS Simulator
jffi run --platform macos    # macOS app
jffi run --platform android  # Android Emulator
jffi run --platform linux    # Linux GTK app
jffi run --platform windows  # Windows app
jffi run --platform web      # Web browser
```

That's it! The app builds, compiles Rust, generates platform bindings, and launches automatically.

## 📱 Supported Platforms

| Platform | Status | UI Framework | Language |
|----------|--------|--------------|----------|
| iOS | ✅ Ready | SwiftUI | Swift |
| macOS | ✅ Ready | SwiftUI | Swift |
| Android | ✅ Ready | Jetpack Compose | Kotlin |
| Linux | ✅ Ready | GTK 4 + Libadwaita | Python |
| Web | ✅ Ready | Vanilla JS + WASM | JavaScript |
| Windows | ✅ Ready | WinUI 3 | C# |

### 🤖 Android: Automatic ndk-context Support

JFFI automatically handles the UniFFI + JNA + ndk-context incompatibility for Android apps using networking libraries.

**The Problem:**
- UniFFI uses JNA (Java Native Access) which doesn't call `JNI_OnLoad`
- Some Rust crates (like `hickory-resolver` used by `iroh`) need Android context for DNS resolution
- Without initialization, apps crash with: `android context was not initialized`

**JFFI's Solution:**
When building for Android, JFFI automatically:
1. **Detects** if your Rust dependencies use `ndk-context` (via `cargo tree`)
2. **Generates** a JNI bridge (`core/src/android.rs`) to initialize ndk-context
3. **Creates** a Kotlin helper (`JffiAndroidInit.kt`) with `initNdkContext()` method
4. **Injects** the initialization call in `MainActivity.onCreate()`
5. **Updates** `core/Cargo.toml` with `ndk-context` and `jni` dependencies

**Result:** Android apps using networking libraries (iroh, hickory-resolver, etc.) work out of the box with zero configuration.

**References:**
- [UniFFI uses JNA](https://mozilla.github.io/uniffi-rs/latest/kotlin/gradle.html)
- [JNA doesn't call JNI_OnLoad](https://github.com/java-native-access/jna/issues/1019)
- [ndk-context docs](https://docs.rs/ndk-context/latest/ndk_context/)
- [UniFFI issue #1778](https://github.com/mozilla/uniffi-rs/issues/1778)

## 🏗️ Project Structure

```
my-app/
├── core/                    # Rust business logic + UniFFI exports
│   ├── src/lib.rs
│   └── Cargo.toml
│
├── ffi-web/                 # WASM FFI wrapper (present when web is enabled)
│   └── src/lib.rs
│
├── platforms/
│   ├── ios/                 # iOS SwiftUI app (auto-generated Xcode project)
│   ├── android/             # Android Jetpack Compose app
│   ├── macos/               # macOS SwiftUI app
│   ├── linux/               # GTK 4 Python app
│   ├── windows/             # WinUI 3 C# app
│   └── web/                 # Vanilla JS + Vite frontend
│
├── jffi.toml                # Framework configuration
├── Cargo.toml               # Workspace manifest
└── Makefile                 # Convenience commands
```

##  Development Workflow

### 1. Write Business Logic + Expose via UniFFI

`core/src/lib.rs`:

```rust
use uniffi;

#[derive(uniffi::Object)]
pub struct Core {
    // Pure computation, no UI state
}

#[uniffi::export]
impl Core {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {}
    }

    pub fn greeting(&self) -> String {
        "Hello from JFFI".to_string()
    }

    pub fn process_data(&self, input: String) -> String {
        format!("Processed: {}", input)
    }
}

uniffi::setup_scaffolding!();
```

### 2. Build & Run

**For iOS:**
```bash
jffi run --platform ios              # Simulator
jffi run --platform ios --device     # Physical device
```

**For Android:**
```bash
jffi run --platform android          # Emulator (auto-starts)
```

**For macOS:**
```bash
jffi run --platform macos
```

**For Linux:**
```bash
jffi run --platform linux
```

**For Web:**
```bash
jffi run --platform web
```

**For Windows:**
```bash
jffi run --platform windows
```

This automatically:
- **iOS/macOS**: Compiles Rust, generates Swift bindings, creates Xcode project, builds and launches
- **Android**: Compiles Rust for 3 architectures, generates Kotlin bindings, starts emulator, builds APK, installs and launches app
- **Linux**: Compiles Rust, generates Python bindings, installs dependencies (GTK 4, build tools), builds and launches GTK app
- **Web**: Compiles Rust to WASM, generates JavaScript bindings, installs npm dependencies, starts Vite dev server
- **Windows**: Compiles Rust, generates C# bindings with uniffi-bindgen-cs, builds with MSBuild/dotnet, launches WinUI 3 app

### 3. Use in Native UI

`platforms/ios/ContentView.swift`:

```swift
Button("Refresh") {
    greeting = core.greeting()
}
```

The generated bindings make Rust functions available natively in Swift, Kotlin, C#, Python, and JavaScript.

## ⚡ Hot Reload

JFFI works seamlessly with native IDE hot reload plus automatic Rust rebuilding.

### iOS/macOS Workflow

```bash
# Start Rust file watcher
jffi dev --platform ios

# In Xcode:
# 1. Open platforms/ios/*.xcodeproj in Xcode
# 2. Run the app (Cmd+R)
# 3. Edit Swift files → Xcode hot reloads automatically ⚡
# 4. Edit Rust files → Watcher rebuilds dylib → Press Cmd+B in Xcode
```

### Android Workflow

```bash
# Start Rust file watcher + Android Studio
jffi dev --platform android

# In Android Studio:
# 1. Press ▶️ to run the app
# 2. Edit Kotlin files → Compose hot reloads automatically ⚡
# 3. Edit Rust files → Watcher rebuilds .so → Rebuild in Android Studio
```

### Linux Workflow

```bash
# Start Rust file watcher
jffi dev --platform linux

# The app will auto-restart when Rust code changes
# 1. Edit Python/GTK files → Save and re-run
# 2. Edit Rust files → Watcher rebuilds .so → App auto-restarts ⚡
```

### Web Workflow

```bash
# Start Rust file watcher + Vite dev server
jffi dev --platform web

# Vite provides hot reload for JS/CSS changes
# 1. Edit HTML/JS/CSS → Vite hot reloads instantly ⚡
# 2. Edit Rust files → Watcher rebuilds WASM → Refresh browser
```

### How It Works

**Native UI Changes:**
- **iOS/macOS**: Edit `.swift` files → Xcode hot reloads instantly
- **Android**: Edit `.kt` files → Compose hot reloads automatically
- **Web**: Edit `.js`/`.css` files → Vite hot reloads instantly
- Use native IDE features (SwiftUI previews, Compose previews, etc.)
- Full debugging support

**Rust Changes:**
- Edit any `.rs` file in `core/` or `ffi/`
- File watcher rebuilds Rust library automatically
- Rebuild in native IDE to use new Rust code
- App updates with new business logic

### Best of Both Worlds

✅ **Native IDE experience** - Use all platform IDE features
✅ **Native UI previews** - SwiftUI/Compose previews work
✅ **Native hot reload** - Instant UI updates
✅ **Rust auto-rebuild** - No manual cargo commands
✅ **Full debugging** - Native debuggers work normally

##  CLI Commands

```bash
# Create new project
jffi new <name> --platforms <platforms>

# Build for platform
jffi build --platform <platform>
jffi build --platform ios --device          # Build for physical device

# Run on platform (builds automatically)
jffi run --platform <platform>
jffi run --platform ios --device            # Run on physical device

# Development mode (auto-rebuild on changes)
jffi dev --platform <platform>

# Add platform to existing project
jffi add <platform>

# Remove platform from existing project
jffi remove <platform>

# List available platforms
jffi platforms
```

##  How It Works

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

##  Configuration

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

##  Why JFFI?

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
- **Android**: Android Studio, Android SDK, Android Emulator (auto-configured)
- **macOS**: Xcode
- **Linux**: GTK 4, Libadwaita, Python 3 (auto-installed by setup script)
- **Web**: Node.js, npm (for Vite dev server)
- **Windows**: .NET SDK 8.0+, Visual Studio Build Tools or MSBuild

### Building the CLI

```bash
cargo build --package jffi
cargo run --package jffi -- --help
```

## Roadmap

- [x] CLI tool foundation
- [x] iOS support with SwiftUI
- [x] macOS support with SwiftUI
- [x] Android support with Jetpack Compose
- [x] Linux support with GTK 4 + Python
- [x] Web support with Vanilla JS + WASM
- [x] Windows support with WinUI 3 + C#
- [x] Automatic Xcode project generation
- [x] Automatic Android project generation
- [x] One-command build and run (iOS, macOS, Android, Linux, Windows)
- [x] Hot reload for iOS (Xcode-native workflow)
- [x] Hot reload for Android (Android Studio-native workflow)
- [x] Hot reload for Linux (auto-restart workflow)
- [x] Hot reload for Web (Vite hot reload)
- [x] Automatic emulator/simulator management
- [x] Auto-install build dependencies (targets, NDK, GTK, WASM, wasm-bindgen, uniffi-bindgen-cs, etc.)

## 🤝 Contributing

Early-stage framework. Contributions welcome!

**High priority:**
- Additional platform features and improvements
- State persistence (SQLite/local storage)
- Advanced UI components and patterns
- Hot reload for Windows (file watcher workflow)

## 📄 License
GPL-3.0

##  Acknowledgments

- [UniFFI](https://github.com/mozilla/uniffi-rs) - FFI bindings generator
- The Rust community

---

**Built with  and Rust**
