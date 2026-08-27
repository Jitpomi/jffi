# `jffi` Changelog

## v0.4.8 — Release Diagnostics and Preview Safety

- Make bundle dry runs and plan printing non-mutating by skipping native project
  synchronization and icon generation.
- Reject unknown, disabled, and aggregate bundle targets before planning;
  distribution remains one explicit platform per invocation.
- Make bundle doctor default to enabled platforms supported by the current host
  instead of attempting every native SDK on one machine.
- Make release diagnostics fail on missing configured signing credentials,
  certificates, identities, and tools instead of producing warning-only green
  results.
- Use the selected signing profile's configured environment-variable names
  during Android release validation.
- Add regression coverage for CLI command parsing, doctor platform selection,
  preview purity, enabled bundle targets, and documentation TOML examples.
- Audit generated web projects in CI, pin their Vite version, and default CORS
  to disabled for new projects.
- Upgrade the pinned checkout action to its Node 24-based release.
- Document `jffi.toml` ownership, supported configuration fields, secret
  boundaries, and safe bundle-plan inspection.
- Document the recommended native CI runner matrix and separation between
  quality gates and protected tag releases.
- Document Apple single-target and multi-target signing, including dedicated
  provisioning-profile mappings for app extensions.
- Link the focused guides from the packaged README using URLs that work from
  both GitHub and crates.io.

## v0.4.7 — Multi-Target Apple Signing

- Support provisioning-profile mappings keyed by bundle identifier so iOS
  applications and their extensions can be archived and exported together.
- Preserve the singular `provisioning_profile` setting for single-target apps.

## v0.4.6 — Configuration-Driven Windows Setup

- Windows requirement checks now validate only the Rust targets selected by `bundle.windows.targets`.
- Unsupported or empty Windows target selections fail before compilation.

## v0.4.5 — Configuration-Driven Android Setup

- Android requirement checks now install and validate only the Rust targets selected by `platforms.android.abis`.
- Unknown or empty Android ABI selections fail with an actionable configuration error.
- This keeps `jffi setup`, `jffi build`, and `jffi bundle` on one ABI source of truth.

## v0.4.4 — Native Release Hygiene

- Upgrade the generated web template to Vite 8.2.2 and declare its Node.js
  runtime floor, removing three known Vite development-server vulnerabilities.
- Generate Linux icons using `bundle.linux.app_id` instead of leaving obsolete
  `org.jffi.App` assets in branded applications.
- Generate Android signing configuration with assignment syntax supported by
  current and upcoming Gradle releases.
- Stop recommending `useLegacyPackaging = true` for every Android project with
  an `ndk-context` dependency. JFFI already emits 16 KB linker alignment for
  Rust libraries; release pipelines must validate all packaged ELF dependencies
  rather than silently switching to compressed legacy JNI packaging.
- Preserve an existing authored `core/src/android.rs` bridge during Android
  builds instead of rewriting it with generated formatting on every run.

## v0.4.3 — Reliable Android Native Packaging

- Prevent stale and hashed Rust dependency libraries from accumulating in Android `jniLibs` output.
- Package only the project's primary JNI library and the required Android C++ runtime for each configured ABI.
- Strip debug sections from packaged JNI copies while preserving symbols in Cargo's target directory, reducing SEYFR's debug APK from 1.8 GB to 71 MB.
- Use the supported `cargo ndk` subcommand entrypoint for setup and diagnostic checks and correct its build argument ordering.
- Include the underlying `adb` output when APK installation fails instead of returning an opaque error.

## v0.4.2 — Managed UniFFI Toolchain Reconciliation

- Automatically install or replace `uniffi-bindgen` when its version does not exactly match the project's resolved UniFFI dependency.
- Continue the original `jffi build`, `run`, `debug`, or `dev` operation after reconciliation succeeds.
- Add global `--no-setup` and `--offline` controls for CI and other environments where automatic tool installation is not appropriate.
- Verify the installed binary after reconciliation and fail clearly if Cargo did not produce the required exact version.

## v0.4.1 — Release Trust & Framework Hardening

- Added `jffi debug` for verbose debug builds with Rust backtraces.
- Made `jffi doctor bundle --release` fail on invalid configuration instead of printing a false-green completion, and added `--profile` for named signing profiles.
- Made Apple entitlement synchronization idempotent and removed accumulated indentation/blank-line corruption.
- Recognize modern Android `packaging.jniLibs.useLegacyPackaging = true` configuration instead of recommending the deprecated manifest attribute.
- Make `build --all` fail when any enabled platform fails instead of returning a false-green result.
- Reject unfinished bundle formats and advertise only formats JFFI currently produces.
- Require icon generation, entitlement synchronization, signing, notarization, and Flatpak export to succeed.
- Verify Windows MSIX and Linux Flatpak artifacts exist before reporting success.
- Stop rewriting Rust source modules as an implicit side effect of `jffi build`.
- Add strict formatting, Clippy, test, package, and generated-project smoke-test gates in CI.
- Embed project templates in the installed binary and add truthful `jffi new --no-build` scaffolding semantics.
- Add explicit `jffi setup --platform` installation; normal build and diagnostic commands no longer install tools or system packages.
- Match `uniffi-bindgen` exactly to the project's pinned UniFFI dependency.
- Reject unknown configuration fields under schema version 1.
- Make web export, icon generation, and release signing validation fail on missing outputs or credentials.

## v0.3.11 — Package Sync & Rename Across All Platforms

- Implemented automatic, end-to-end Kotlin/Java source package and directory refactoring on Android when the package name changes.
- Implemented Linux GTK application ID synchronization in `app.py`.
- Implemented Windows Package.appxmanifest `<Identity Name="..." />` synchronization.

## v0.3.10 — Android Native Debug Symbols

- `jffi bundle --platform android` now honors `bundle.android.split_debug_symbols` and produces `target/bundle/android/native-debug-symbols.zip` for Google Play native crash and ANR symbolication.
- Prebuilt Rust `.so` libraries are archived directly when Gradle cannot generate the symbol archive itself.

## v0.3.9 — Android Release Configuration Fix

- Android config sync now uses `platforms.android.target_sdk`, then falls back to `bundle.android.compile_sdk`, instead of silently targeting API 35.
- An explicit `bundle.build_number` now wins over `GITHUB_RUN_NUMBER`. The CI run number is used only when no build number is configured.
- New Android projects now compile and target API 36 by default.

---

## v0.3.8 — Bug Fix

### 🐛 Windows MSIX Version Format Corrected
- **Fixed MSIX revision rejection by Microsoft Store**: v0.3.7 placed `version_code` (CI run number) as the 4th MSIX version component (e.g. `1.6.1.97`). Microsoft Store rejects any package where the revision (4th component) is non-zero: *"Apps are not allowed to have a Version with a revision number other than zero"*.
- **Correct format is `Major.Minor.{version_code}.0`**: CI build number is now placed in the **3rd (Build) component**; 4th is always `0`. e.g. version `1.6.1`, CI run `#99` → MSIX `1.6.99.0` ✅

---

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

# Historical v1.0.0 Planning Draft

This preserved planning draft described the intended stable `1.0.0` scope. It is not a released version; the authoritative release history is the versioned list above.

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
