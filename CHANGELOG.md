# `jffi` Changelog

## v0.3.7 — Bug Fix

### 🐛 Windows MSIX Version Fix
- **Fixed Windows MSIX package identity always being `X.Y.Z.0`**: The 4th component of the MSIX version was hardcoded to `.0` regardless of `build_number` or `GITHUB_RUN_NUMBER`. This caused Microsoft Store submissions to fail with a "duplicate package identity" error whenever you tried to submit a new build with the same semver but a different CI run number.
- **Fix**: The 4th MSIX version component now uses `version_code` (which is auto-injected from `GITHUB_RUN_NUMBER` in CI). A build tagged `v1.6.1-b78` on CI run #78 will now produce `MSIX version 1.6.1.78` — unique and accepted by Partner Center.
- All other platforms (Android `versionCode`, iOS/macOS `CURRENT_PROJECT_VERSION`) were already correct and are unaffected.

---

## v0.3.3 — Bug Fix

### 🐛 Android ABI Fix
- **Fixed x86 crash bug**: Removed `x86` from the `hello` template's `abiFilters` in `build.gradle.kts`. The Rust core was never compiled for x86, causing a crash on x86 devices (and emulators) because the x86 `.so` directory existed in the AAB without `libseyfr_core.so`.
- **Hardened generated gradle**: `jffi-bundle.gradle` now uses `abiFilters.clear()` + `abiFilters.addAll([...])` instead of the additive `abiFilters` Groovy method, ensuring the correct ABI list from `jffi.toml` is applied definitively.

---

# `jffi` v1.0.0 Comprehensive Changelog

This release marks the first stable `1.0.0` version of the `jffi` framework. We have completely transformed the framework from a local development tool into a robust, production-grade release orchestrator. 

This changelog exhaustively details every feature, fix, and template improvement introduced over the last 33 commits since version `0.3.2`.

---

## 🚀 The Release Orchestrator
*Version 1.0.0 introduces an entirely new suite of top-level commands to securely package and release native apps to app stores.*

- **`jffi bundle` Command**: Added a brand new command that orchestrates the compilation, resource generation, and artifact packaging for all platforms simultaneously.
- **`jffi doctor` Command**: Added an automated environment verification tool that scans your host machine for missing compilers, SDKs, and platform requirements (e.g., Xcode, Android Studio, Windows Kits SDK).
- **`jffi icons` Command**: Added an automated cross-platform icon generation pipeline. `jffi` will now natively convert your single high-res `jffi.toml` source icon into `.ico`, `.icns`, `.png`, and Android adaptive vector drawables.
- **Platform-Specific Icon Overrides**: `jffi.toml` now natively supports platform-specific icon configuration overrides to serve different app icons for Windows vs macOS vs iOS, etc.
- **Auto-Fixing Core Privacy**: Added automated AST patching to dynamically detect and fix private modules in `core/src/lib.rs` that UniFFI previously dropped silently.
- **Release Run Flag**: Added `--release` flag support to `jffi run` to test highly optimized native binaries directly from the CLI without fully bundling.

## ⚙️ CI/CD & Cross-Platform Versioning
*Complete decoupling of version management from source code for seamless GitHub Actions integration.*

- **Zero-Touch CI Build Numbers**: `jffi` now dynamically intercepts `GITHUB_RUN_NUMBER` from the CI environment and automatically overrides native configurations (`versionCode`, `CURRENT_PROJECT_VERSION`, etc.) in memory before bundling.
- **Cross-Platform Auto-Versioning**: `jffi.toml` acts as the definitive source of truth. Version strings and build numbers are dynamically string-replaced into `core/Cargo.toml`, Android, iOS, macOS, and Windows source files on every build.
- **Store-Readiness Validation**: The bundle command now actively validates your `jffi.toml` configuration to ensure you have correct bundle IDs, provisioning profiles, and code-signing identifiers before it permits a release build.

## 🍎 Apple (iOS & macOS)
*Overhauled the entire Apple build pipeline to support automated CI code-signing.*

- **Dynamic Xcode Synchronization**: `MARKETING_VERSION` and `CURRENT_PROJECT_VERSION` are now automatically injected into `project.pbxproj`.
- **Code Signing Integrity**: Patched the `bundle` command to reliably inject `CODE_SIGN_IDENTITY` and `PROVISIONING_PROFILE_SPECIFIER` directly into `xcodebuild` during release pipelines, bypassing local Keychain access bugs.
- **Template Info.plist Fixes**: Fixed `Info.plist` template generation bugs that caused missing metadata and launch failures upon project initialization.

## 🪟 Windows (WinUI 3 & MSIX)
*Massive stability improvements to C# native packaging, caching, and MSBuild compilation.*

- **Advanced Executable Discovery**: Added intelligent auto-discovery paths in Windows Kits to natively locate `makeappx.exe` and `signtool.exe` without hardcoding SDK versions.
- **Multi-Path Icon Fallbacks**: The `MainWindow.xaml.cs` template now includes robust recursive icon loading logic to prevent runtime crashes if `Assets/app.ico` fails to load.
- **Automated `app.ico` WinUI Integration**: Rebuilt the Windows pipeline to automatically generate `app.ico` files and cleanly integrate automated icon setups in the WinUI boilerplate.
- **Automated App Uninstall**: Fixed deployment caching bugs (Error `0x80073CFB`) by automatically enforcing `Remove-AppxPackage` on conflicting local versions before deploying new development builds.
- **Publish Settings Overhauled**: Disabled `PublishReadyToRun` and `PublishTrimmed` in MSBuild to prevent C# native AOT compiler crashes against complex Rust interop layers.
- **Manifest Synchronization**: `Package.appxmanifest` dynamically stays in sync with `jffi.toml` versions.
- **Assets Directory Copying**: Ensured `Assets` folders are copied strictly to output directories during MSBuild generation.

## 🐧 Linux (GTK4 & AppImage)
*Transformed the Linux target from local-only into a containerized deployment target.*

- **First-Class Flatpak Support**: Introduced scaffolding and build hooks for native Flatpak distribution.
- **Advanced Python Automation**: Supported Linux dependency setup on macOS hosts, and automated Python `requirements.txt` installations.
- **PEP 668 Management**: Implemented fallback `--break-system-packages` logic for strictly managed Python environments during dependency initialization.
- **Cairo & Gir Integration**: Added explicit Cairo and Gir dependencies for robust GTK4 setup on raw virtual machines.
- **Headless Display Toggles**: Improved Linux display detection logic and added the ability to securely disable headless mode for CI/CD runners.

## 🤖 Android (Gradle & APK/AAB)
*Fixed silent JNI bugs and improved Gradle scaffolding.*

- **Automatic Wrapper Generation**: Added automated Gradle wrapper generation, ensuring cross-platform host parity without requiring a pre-installed local Gradle installation.
- **Dynamic Gradle Bundling**: Added `jffi-bundle.gradle` application out of the box to unify signingConfigs and release builds natively.

## 🌐 Web (Vite & Wasm)
*Transformed the Web build output into a production-ready SPA.*

- **SEO & Meta Tag Synchronization**: Running `jffi build` on a web target now parses OpenGraph tags, descriptions, themes, and SEO elements from `jffi.toml` and flawlessly injects them into the `<head>` of `index.html`.
- **Configurable Vite Bindings**: Replaced hardcoded `wasm` output paths with the dynamically generated crate name to ensure seamless import resolution inside generic Vite scaffolding.
- **Customizable Dev Server**: Added configurable web/Vite option support directly to `jffi.toml`.

## 🛠️ Internal C# Bindings & Rust Core
- **Automated C# Interface Prefix Bug Fixes**: Hardened the internal C# bindings by dynamically applying a regex patch that enforces correct callback interfaces directly after `uniffi-bindgen-cs` runs.
- **Callback-Specific Targeting**: Refined the C# patcher to strictly target *only* callback interfaces, preventing illegal syntax mutations on non-callback interfaces.
- **Borrow Checker Panic Patch**: Fixed a critical borrow checker panic inside the C# patching logic during Windows MSIX packaging.
- **MakeAppx Traversal**: Recursively un-nested directory traversals for `AppxManifest.xml` discovery to guarantee MSIX output validity.
