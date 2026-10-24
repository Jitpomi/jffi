# Native workflows

JFFI keeps Rust business logic shared and uses each platform's native UI and
release tools. Equivalent commands should honor the same requested build profile,
but their native artifacts differ.

| Command | Contract |
| --- | --- |
| `build` | Compile the Rust core and generate native bindings. Apple emits XCFrameworks; Android emits JNI libraries; Linux emits a library and Python bindings. Windows also compiles its C# app. |
| `run` | Build the requested profile, build the native UI where needed, and launch that output. No fallback to debug output for `--release`. |
| `debug` | Run a debug build with verbose diagnostics. It does not attach a breakpoint debugger. Use Xcode, Android Studio, Visual Studio, LLDB, or GDB for debugger attachment as appropriate. |
| `dev` | Watch Rust sources and rebuild bindings. Xcode, Android Studio, and Visual Studio require native rebuild/relaunch actions; Linux restarts the application after Rust or Python UI changes. Native IDE preview/reload features are not guaranteed by JFFI. |
| `bundle` | Build release distribution artifacts, applying the selected signing profile unless `--no-sign` is explicit. |

## Device selection and release runs

```sh
jffi run --platform macos --release
jffi run --platform linux --release
jffi run --platform windows --release
jffi run --platform ios --release --device-id <SIMULATOR-UDID>
jffi run --platform ios --device --device-id <DEVICE-UDID>
jffi run --platform android --device --device-id <ADB-SERIAL>
```

Use `xcrun simctl list devices available`, `xcrun devicectl list devices`, or
`adb devices` to discover identifiers. iOS simulator selection includes iPads,
filters by runtime, and uses the selected UDID for installation and launch.
Physical iOS deployment requires signing configured in the Xcode project and a
version of Xcode that provides `devicectl`. Its identifier must also be usable as
an Xcode destination ID. Android physical devices must be connected and authorized. Set `JFFI_ANDROID_STUDIO`
to the IDE executable if JFFI cannot discover it on your host.
An explicit Android serial may select a physical device without `--device`.
Ambiguous Android device selections require `--device-id`.

The bundled Gradle 8.13 template can run with JDK 17 or 21. If the system Java is
newer than the wrapper supports, set `JAVA_HOME` to a compatible JDK (Android
Studio's bundled runtime is another option). JFFI does not change the system JDK.

Android `run --release` builds the release variant with the development signing
key so it can be installed locally. Distribution signing remains the job of
`bundle` and its selected signing profile. SDK environment variables are preserved.

Apple launch products live under `target/jffi/xcode/<platform>/Build/Products`,
with the requested Debug/Release configuration. Windows launch and packaging use a fresh JFFI-owned `bin/<architecture>/<configuration>/jffi`
directory, shared with the native build. C# builds stage the matching Rust DLL for
each architecture and use an explicit runtime identifier.
Linux requires a Linux host; use a Linux VM or CI runner from macOS or Windows.

Apple `build --device` prepares a combined iOS XCFramework containing simulator
and device slices. `macos-arm64` and `macos-x64` remain compatibility aliases for the universal
library. Use `--platform macos` and `bundle.macos.targets` for native distribution
architecture selection. `--deploy` is specific to Windows;
it builds exactly `bundle.windows.targets` and fails if any requested toolchain
is unavailable.

## Distribution

Windows bundling builds every configured architecture in Release before packaging
and checks signed MSIX packages using SignTool. Each architecture stages its own
DLL, uses an explicit runtime, and builds into a clean owned output directory.
The configured publisher is synchronized into the MSIX manifest. Local runs
register/update the app in place instead of uninstalling it and clearing its data.
Windows Rust watch mode follows the host architecture; it reports locked DLL
reload failures rather than silently continuing. Android packages are copied to
`target/bundle/android`; signed APKs are checked using `apksigner`, and signed AABs
using `jarsigner`. Set `JFFI_APKSIGNER` to override SDK build-tool discovery.
Apple archives honor configured deployment targets and signing-profile entitlements.
Windows minimum versions are synchronized into both the project and MSIX manifest;
the Linux template requires GTK 4.0.
Signed app bundles are checked with `codesign`; IPA exports must exist and be nonempty.
Unsigned iOS bundling produces an `.xcarchive`, since IPA export requires signing.
Use `--format xcarchive --no-sign` explicitly; `--format ipa --no-sign` is rejected.
Unsigned macOS app/DMG/PKG exports use the archived app directly.

Linux signing uses a GPG key available to the native tooling:

```toml
[bundle.signing.profiles.release.linux]
gpg_key = "YOUR-GPG-KEY-ID"
# gpg_homedir = "/path/to/protected/keyring"
```

Flatpak Builder signs the repository commit with this profile. Single-file bundle
export preserves the signed commit and embeds the exported public key with
`--gpg-keys`; `gpg` must be installed. Each bundle build starts with a fresh
generated repository to keep earlier signatures out of an unsigned build.
Use `--no-sign` for an intentionally unsigned Flatpak. Native signing failures
propagate to JFFI; successful signing alone does not certify store acceptance.
Apple notarization remains a macOS-specific operation.

`bundle.resources` and `bundle.license_file` are currently rejected if set, rather
than silently ignored. Include resources/licenses through the native platform
projects. The screenshot command resizes supplied PNGs from `playstore-assets`
into Apple dimensions; it does not capture live applications.

## Verification coverage

Unit tests cover profile paths, device selection, architecture mappings, unsigned
versus signed configuration, and missing/empty package outputs. Native generated-app
CI jobs compile Apple UI, build an Android APK, and build unsigned Apple, Windows,
and Linux packages on native hosts. The macOS job also includes Windows in the generated
project to exercise the shared core's Windows-compatible UniFFI version.

CI definitions are checks to run, not evidence that a particular run succeeded.
Physical devices, production certificates, store submissions, and native IDE
breakpoint attachment require manual validation. watchOS, tvOS, and visionOS
remain a separate platform expansion; this work establishes consistent behavior
for the existing platforms first.
