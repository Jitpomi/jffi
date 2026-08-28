# JFFI configuration

`jffi.toml` is the application's checked-in configuration contract. JFFI reads
it before generating, building, running, debugging, or bundling native targets.
Unknown fields are rejected, which makes misspelled or obsolete settings fail
early instead of being silently ignored.

## What belongs in `jffi.toml`

- Package name, semantic version, and platform build number
- Enabled platforms and their deployment targets
- Bundle identifiers, application groups, and store metadata
- Native architectures and Android ABIs
- Bundle formats and shared icon sources
- Names or paths that refer to signing assets
- Names of environment variables that hold credentials

Do not put passwords, private keys, certificates, provisioning-profile
contents, or other secret values in this file. Store them in the native
keychain, a CI secret store, or protected environment variables.

## Minimal configuration

```toml
schema_version = 1

[package]
name = "my-app"
version = "1.0.0"
version_code = 1
release_date = "2026-08-28"

[platforms]
enabled = ["ios", "macos", "android", "linux", "windows", "web"]

[platforms.ios]
deployment_target = "16.0"
bundle_id = "com.example.myapp"

[platforms.macos]
deployment_target = "13.0"

[platforms.android]
min_sdk = 26
target_sdk = 36
package = "com.example.myapp"

[platforms.linux]
gtk_version = "4.0"

[platforms.windows]
min_version = "10.0.19041.0"

[platforms.web]
target = "es2020"
port = 5173
```

Only sections for enabled platforms need application-specific values. Run
`jffi doctor config` after every configuration change.

## Bundle metadata and icons

```toml
[bundle]
name = "My App"
identifier = "com.example.myapp"
version = "1.0.0"
build_number = 1
homepage = "https://example.com"

[bundle.icons]
source = "store-assets/app-icon.png"
generate = true

[bundle.macos]
formats = ["app", "dmg"]
targets = ["aarch64-apple-darwin", "x86_64-apple-darwin"]
minimum_system_version = "13.0"

[bundle.ios]
format = "ipa"
destination = "generic/platform=iOS"
export_method = "app-store-connect"

[bundle.android]
formats = ["aab", "apk"]
abis = ["arm64-v8a", "armeabi-v7a", "x86_64"]
min_sdk = 26
compile_sdk = 36

[bundle.windows]
formats = ["msix"]
targets = ["x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"]

[bundle.linux]
formats = ["flatpak"]
app_id = "com.example.MyApp"

[bundle.web]
dist_dir = "dist"
build_command = "npm run build"
wasm_opt = true
base_path = "/"
```

`bundle.version` and `bundle.build_number` are the canonical native release
values. Before a build, run, development session, or non-preview bundle, JFFI
synchronizes them into Android Gradle metadata, Apple Xcode projects, the Rust
core crate, the Windows MSIX manifest and `.csproj`, and Linux AppStream
metadata. Do not maintain independent platform versions by hand.

The configured source icon must be PNG, JPEG, or ICO and must exist when JFFI
builds. `jffi icons --platform all` regenerates platform-specific assets.

## Signing profiles

Signing profiles are named, non-secret descriptions selected with
`jffi bundle --profile <name>`. The active default can be declared under
`[bundle.signing]`.

```toml
[bundle.signing]
profile = "release"

[bundle.signing.profiles.release.android]
keystore_path = "signing/upload.jks"
key_alias = "upload"
store_password_env = "JFFI_ANDROID_STORE_PASSWORD"
key_password_env = "JFFI_ANDROID_KEY_PASSWORD"

[bundle.signing.profiles.release.windows]
certificate_thumbprint = "CERTIFICATE_THUMBPRINT"
```

For Apple configuration, including applications with extensions, see
[Apple signing](apple-signing.md).

## Validation and safe inspection

Use this sequence before committing a configuration change:

```bash
jffi doctor config
jffi doctor bundle --platform ios --release --profile release
jffi bundle --platform ios --profile release --dry-run --print-plan
```

Repeat the bundle checks for each release platform. A dry run validates and
explains the planned work without synchronizing native project files, producing
an artifact, or uploading a release. Use `--print-commands` during an actual
bundle operation when native command diagnostics are required; secrets are
redacted.

In controlled or disconnected environments, use the global options before the
subcommand:

```bash
jffi --no-setup build --platform ios
jffi --offline build --platform ios
```

`--no-setup` prevents managed-tool installation or reconciliation. `--offline`
also disables network access for Cargo.

## Configuration versus generated native files

Treat `jffi.toml` as authoritative for fields JFFI manages. Platform-native
source code, entitlements, manifests, and project files can still contain
platform-specific behavior that has no JFFI setting. Review generated diffs
after upgrades and never use `jffi.toml` to store credentials merely to make a
pipeline self-contained.
