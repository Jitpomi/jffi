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
# Install the released JFFI CLI
cargo install jffi --locked
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

### Check Environment Readiness

Before building or bundling for new platforms, use `jffi doctor` to reconcile and verify that your development environment has all the required tools, SDKs, and compilers installed. Pass `--no-setup` or `--offline` for validation without installation.

```bash
# Check enabled platforms that this host can build
jffi doctor bundle

# Validate jffi.toml without checking SDKs or development tools
jffi doctor config

# Check readiness for a specific platform
jffi doctor bundle --platform android

# Explicitly install missing development tools for a platform
jffi setup --platform android

# Keep builds non-mutating in controlled environments
jffi build --platform ios --no-setup

# Disable network access and automatic managed-tool setup
jffi build --platform ios --offline
```

JFFI automatically reconciles `uniffi-bindgen` to the exact UniFFI version resolved by the project's `Cargo.lock`, then resumes `build`, `run`, `debug`, or `dev`. Use `--no-setup`, `--offline`, or `JFFI_NO_SETUP=1` when tool installation must remain explicit.

### Bundle Your App for Distribution

Once your application is ready, JFFI can orchestrate packaging for the formats it currently supports: Android APK/AAB, macOS app/DMG/PKG, iOS IPA, Windows MSIX, Linux Flatpak, and web distributions.

Real bundle operations reconcile the selected platform's prerequisites automatically. On Linux this includes the configured Flatpak SDK and runtime, not only `flatpak-builder`. Use the global `--no-setup` or `--offline` flag when installation must be disabled; plan and dry-run modes remain read-only.

Configured source icons must be PNG, JPEG, or ICO files. A configured but missing or unreadable icon is treated as a build error.

```bash
# Bundle for macOS (creates a .dmg)
jffi bundle --platform macos

# Bundle for Windows (creates MSIX packages)
jffi bundle --platform windows

# Bundle for Linux (creates a Flatpak)
jffi bundle --platform linux --no-sign  # Or configure a Linux GPG signing profile

# Bundle for Android (creates an .aab or .apk)
jffi bundle --platform android

# Advanced options (code signing, notarization, specific formats)
jffi bundle --platform macos --format app --profile release --notarize
```

Bundle one platform per invocation. This keeps native-host requirements and
release artifacts explicit; `jffi bundle --platform all` is intentionally not
supported.

## 📱 Supported Platforms

| Platform | Status | UI Framework | Language |
|----------|--------|--------------|----------|
| iOS | ✅ Ready | SwiftUI | Swift |
| macOS | ✅ Ready | SwiftUI | Swift |
| Android | ✅ Ready | Jetpack Compose | Kotlin |
| Linux | ✅ Ready | GTK 4 + Libadwaita | Python |
| Web | ✅ Ready | Vanilla JS + WASM | JavaScript |
| Windows | ✅ Ready | WinUI 3 | C# |

### 💻 Development Host Compatibility

JFFI currently requires the native host for building and running most desktop/mobile platforms. Android and Web are cross-platform by default.

| Target Platform | Mac Host | Windows Host | Linux Host |
|-----------------|----------|--------------|------------|
| **iOS**         | ✅ Native | ❌ No       | ❌ No       |
| **macOS**       | ✅ Native | ❌ No       | ❌ No       |
| **Android**     | ✅ Native | ✅ Native    | ✅ Native    |
| **Windows**     | ❌ No     | ✅ Native    | ❌ No       |
| **Linux**       | ❌ No     | ❌ No       | ✅ Native    |
| **Web**         | ✅ Native | ✅ Native    | ✅ Native    |

*Note: Android and Web can be developed from any host. Other platforms currently require their respective native host for UI development and execution.*

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
6. **Configures** 16 KB page alignment for all Rust libraries (`.cargo/config.toml`)
7. **Sets** `android:extractNativeLibs="true"` for prebuilt AAR dependencies (JNA, ML Kit)

**Result:** Android apps using networking libraries (iroh, hickory-resolver, etc.) work out of the box with zero configuration.

**Note on 16 KB Alignment:**
- Your Rust code: ✅ Automatically aligned via linker flags
- Prebuilt AARs (JNA, ML Kit): ❌ Not aligned (upstream issue)
- Solution: `extractNativeLibs="true"` extracts libraries at install time (official Android approach)

**Note on Production Bundling & Signing Environment:**
- When running `jffi bundle --platform android`, JFFI's build orchestrator automatically generates a production signing configuration block derived from your `jffi.toml` profiles.
- **Secure Interactive Prompts** *(recommended)*: If the signing environment variables are not pre-set, `jffi bundle` will securely prompt you for your keystore and key passwords at runtime with masked input — no plaintext exposure in shell history:
  ```
  🔑 Env var JFFI_ANDROID_STORE_PASSWORD not set. Keystore password is required for signing.
  Enter Android Keystore Password (JFFI_ANDROID_STORE_PASSWORD): ••••••••••••••
  🔑 Env var JFFI_ANDROID_KEY_PASSWORD not set. Key password is required for signing.
  Enter Android Key Password (JFFI_ANDROID_KEY_PASSWORD): ••••••••••••••
  ```
- **CI/CD (non-interactive)**: For automated pipelines, export the variables before invoking the command:
  ```bash
  export JFFI_ANDROID_STORE_PASSWORD=your_keystore_password
  export JFFI_ANDROID_KEY_PASSWORD=your_key_password
  jffi bundle --platform android
  ```
  > ⚠️ Never hard-code passwords in scripts committed to version control. Use CI secret management (e.g. GitHub Actions Secrets, GitLab CI Variables, Fastlane Match).

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
jffi run --platform ios --device ipad                  # Available iPad simulator
jffi run --platform ios --device iphone                # Available iPhone simulator
jffi run --platform ios --device "iPad Pro 11-inch (M4)" # Exact simulator name
jffi run --platform ios --device --device-id <UDID>     # Physical device
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

> **Note on Web Builds & C Compilers:** `jffi` automatically unsets `CC_wasm32_unknown_unknown` to prevent build failures if you have it set in your shell profile (e.g. for `esp32` development). If you need to override it, you can run: `CC_wasm32_unknown_unknown=clang jffi dev --platform web`

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
jffi run --platform ios --device --device-id <UDID>            # Run on physical device

# Run a debug build with verbose JFFI output and Rust backtraces
jffi debug --platform <platform>
jffi debug --platform ios --device --device-id <UDID>          # Verbose diagnostics on physical device

# Development mode (auto-rebuild on changes)
jffi dev --platform <platform>

# Add platform to existing project
jffi add <platform>

# Remove platform from existing project
jffi remove <platform>

# Show available commands and platform arguments
jffi --help
```

`jffi build`, `jffi run`, and `jffi doctor` do not install system packages. JFFI
may reconcile managed Rust tools such as `uniffi-bindgen` unless `--no-setup` or
`--offline` is set. Use `jffi setup --platform <platform>` when you want JFFI to
install missing platform tools. Use `jffi new ... --no-build` to scaffold a
project without installing tools or running its initial build.

New configuration files declare `schema_version = 1`. Unknown configuration fields are rejected so misspelled settings cannot be silently ignored.

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

`jffi.toml` is the checked-in source of truth for application metadata,
enabled platforms, native target settings, bundle formats, icons, and signing
*references*. Credentials and private signing material stay in the platform
keychain, CI secret store, or environment variables.

See the focused guides for the complete operating model:

- [Configuration reference and ownership](https://github.com/Jitpomi/jffi/blob/main/docs/configuration.md)
- [CI/CD and release pipelines](https://github.com/Jitpomi/jffi/blob/main/docs/ci-cd.md)
- [Apple signing and multi-target applications](https://github.com/Jitpomi/jffi/blob/main/docs/apple-signing.md)

`jffi.toml`:

```toml
[package]
name = "my-app"
version = "0.1.0"
release_date = "2026-08-28"

[platforms]
enabled = ["ios"]

[platforms.ios]
deployment_target = "16.0"
bundle_id = "com.example.myapp"
```

Apps containing extensions must map every signed bundle identifier to its own
provisioning profile. JFFI writes this mapping into the iOS export options and
lets the Xcode project retain target-specific archive signing settings:

```toml
[bundle.signing.profiles.ios-appstore.apple]
team_id = "YOUR_TEAM_ID"
method = "app-store-connect"
signing_certificate = "Apple Distribution"
provisioning_profiles = { "com.example.myapp" = "My App Store Profile", "com.example.myapp.ShareExtension" = "My Share Extension Profile" }
```

`provisioning_profile = "Profile Name"` remains supported for single-target
apps. Do not use both forms unless the mapping is intended to take precedence.

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
- **Linux**: compiler, pkg-config, GTK 4, Libadwaita, Python 3, and Flatpak Builder (reconciled by `jffi setup --platform linux` on supported package managers)
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
- [ ] Full cross-compilation support (build any platform from any host)
    - [ ] macOS -> Windows (via MinGW/Wine)
    - [ ] Linux -> Windows (via MinGW/Wine)
    - [ ] Linux -> macOS/iOS (via osxcross)
    - [ ] Windows -> Linux (via WSL or cross-rs)

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

### Native workflow behavior

See [native workflows](docs/native-workflows.md) for build outputs, release runs,
device selection, signing, development reload behavior, and validation coverage.

### Selecting a device for run and debug

Both commands accept the same selection options:

```bash
jffi run --platform ios --device ipad
jffi debug --platform ios --device iphone
jffi run --platform ios --device "iPad Pro 11-inch (M4)"
jffi run --platform android --device emulator
jffi debug --platform android --device Pixel_Tablet
jffi run --platform android --device-id emulator-5554
jffi run --platform ios --device physical --device-id <UDID>
jffi run --platform android --device physical --device-id <ADB-SERIAL>
```

For Android, use an exact AVD name from `emulator -list-avds` or a connected
serial from `adb devices`. A named AVD is reused if already running, or started
if stopped. `--device emulator` uses an existing emulator, or starts the first
available AVD. Multiple connected matches require an explicit selection.

For iOS, family selection prefers a booted matching simulator; exact names and
UDIDs are also accepted. Bare `--device` still means a physical device.
Do not combine a named `--device` selector with `--device-id`.
macOS, Windows, Linux, and web run locally and do not accept device selectors.

### Automatic web bindings

Web builds regenerate `ffi-web/src/lib.rs` from the compiled UniFFI API, including
exported methods, objects, and standalone functions. JavaScript uses camelCase names, so Rust's
`get_count()` becomes `core.getCount()`. `run`, `build`, and Rust watch-mode
rebuilds use the same generator; new web projects start with a generated wrapper.

The generator creates matching WASM exports and TypeScript declarations for
UniFFI records, enums (including variant payloads), errors, exported object
references, type aliases, callback traits, UniFFI custom types, and nested `Option`, `Vec`, `HashMap`, and `BTreeMap`
values. No serde annotations need to be added to the native core: generated
wire conversion code lives in `ffi-web`. The required web-crate dependencies are
reconciled automatically.

| Rust API | JavaScript / TypeScript |
| --- | --- |
| record fields | Typed plain objects with camelCase field names |
| enum | Tagged union: `{ tag: "Variant", value: ... }` |
| `Option<T>` | `T \| null` |
| `Vec<T>` | `Array<T>` |
| byte buffers (`Vec<u8>`) | `Uint8Array` (`JffiBytes`) |
| `SystemTime` | `bigint` nanoseconds since the Unix epoch (`JffiTimestamp`) |
| `Duration` | Nonnegative `bigint` nanoseconds (`JffiDuration`) |
| maps | `Map<K, V>` |
| `u64`, `i64` | `bigint`, preserving all integer bits |
| `Arc<ExportedObject>` | Live typed JS class with shared Rust state; call `.free()` when done |
| `Result<T, E>` | Returns `T`; throws `Error` with typed exported error data in `.details` |
| async methods and callbacks | `Promise<T>` |
| callback traits (`Box<dyn ...>` / `Arc<dyn ...>`) | Typed JS interfaces, callable in both directions |
| `custom_type!` / `custom_newtype!` | Named TS alias of the builtin representation, with native UniFFI validation and normalization |

Parameterized constructors are supported; alternate Rust constructors become
static camelCase factories. Async constructors, including `new`, become async
static factories (`await Class.new(...)`). Objects do not need a constructor
named `new`. Custom conversions reuse the core's `FfiConverter`
implementation with the same UniFFI version and tag; their logic is not duplicated
in JavaScript. Remote types and the default `Into`/`TryFrom` converter form are
supported too.

JavaScript callback objects must implement the exported camelCase methods. Rust
can retain them, return native trait implementations to JavaScript, or nest them
in records. JavaScript method receivers are preserved. `self: Arc<Self>` receivers
and callbacks with more than eight arguments are supported too. Async callback methods
accept Promises; both local and Send future modes are supported when matching the
core's UniFFI/async-trait configuration. Callbacks execute on the WASM instance's
event loop; this does not add browser worker/native-thread support.

A declared callback error is thrown as `Error` with its exported error value in
`.details`. Unexpected callback exceptions and invalid return values follow
UniFFI's native `LiftReturn` policy, including the core's
`From<UnexpectedUniFFICallbackError>` implementation. An infallible callback must
honor its return/exception contract, just as on native platforms.

Standalone `#[uniffi::export] pub fn` and async functions become top-level
camelCase JavaScript exports with generated TypeScript signatures, using the
same type and error conversions as object methods. A Core object is optional.

The exported contract is read from the compiled WASM library's UniFFI metadata
(0.29.x and 0.31.x), after Rust resolves macros, includes, features, build scripts,
and conditional compilation. The generated adapter calls UniFFI's exported C ABI
symbols and uses the UniFFI wire format, object ownership rules, callback tables,
and future protocol. It does not import the original Rust types or call Rust
methods by source path. Private modules, private exported types/fields/functions,
renamed exports, and arbitrary compiler-expanded legacy macros therefore need no
public reexports or manual web adapters. This includes macros defined in dependencies.

Exported dependency types are included from compiled metadata, including custom
types reached through transitive or renamed dependencies and separate workspaces.
The adapter links Core's existing dependency graph; it does not add direct imports
of those dependencies. Custom validation/normalization and unexpected callback
error conversion run in the original UniFFI scaffolding. Conflicting public type
names are qualified with their metadata namespace. Only APIs actually exported
by UniFFI are included; ordinary private Rust helpers remain internal.

Disabled declarations disappear from JavaScript and `.d.ts` on the next build.
Generic APIs and external types without UniFFI metadata/custom mappings require
an explicit adapter. Platform-specific Rust APIs must themselves work on wasm32;
code generation cannot supply native OS facilities or browser worker support.
On Rust's `wasm32-unknown-unknown` target, `SystemTime` cannot represent pre-epoch
timestamps; those inputs return a type error. Nanoseconds and large durations
otherwise remain exact. UniFFI 0.31.1 requires its
`wasm-unstable-single-threaded` feature for WASM futures; the suite tests that
supported mode and tests both local and Send future modes with 0.29.4.

The compatibility suite compiles real WASM, generates and loads native UniFFI
Python bindings, compares normalized results across both runtimes, type-checks
valid/invalid TypeScript calls, tests native Rust behavior, and checks removal of
exports after feature/profile/source changes. CI runs it for 0.29.4 and 0.31.1:

```bash
JFFI_TYPESCRIPT_TSC=/path/to/typescript/lib/tsc.js \
  cargo test -p jffi --test web_bindings_wasm -- --ignored --nocapture
# To exercise the current metadata format, put matching uniffi-bindgen on PATH:
JFFI_COMPAT_UNIFFI_VERSION=0.31.1 JFFI_TYPESCRIPT_TSC=/path/to/typescript/lib/tsc.js \
  cargo test -p jffi --test web_bindings_wasm -- --ignored --nocapture
```

It requires Node, Python, the wasm32 Rust target, wasm-pack, matching
uniffi-bindgen, and wasm-bindgen-cli 0.2.129. TypeScript checks run when
`JFFI_TYPESCRIPT_TSC` is supplied; CI always supplies it. An optional absolute
`JFFI_WASM_TEST_TARGET_DIR` reuses a build cache. Regular unit tests do not install
or invoke these tools. The suite defines tested compatibility coverage rather
than claiming every possible Rust/UniFFI API has been verified.

The original greeting-only JFFI scaffold is migrated automatically. Custom
wrappers are preserved: back up and remove `ffi-web/src/lib.rs` to opt into
generation. Edit exported methods in `core/src`, not the generated wrapper.
UI elements and event handlers remain application code.
