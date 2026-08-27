# CI/CD with JFFI

JFFI makes native builds reproducible from `jffi.toml`, but it does not replace
the native SDKs, signing services, or CI secret store. Each target still needs a
compatible host and its platform tooling.

## Recommended pipeline contract

Use two distinct pipeline layers:

1. **Quality gates on pull requests and branch pushes.** Validate configuration,
   run Rust checks and tests, and build the targets supported by each runner.
2. **Distribution on protected version tags or a manual release dispatch.**
   Import protected signing material, run release-readiness checks, bundle, and
   upload only after the quality gates pass.

A Git push is not by itself a release command unless the repository workflow is
explicitly configured that way. A common convention is for `v*` tags to create
store artifacts while pushes to the main branch run non-publishing checks.

## Local preflight

Before pushing a configuration or release change:

```bash
jffi doctor config
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For every intended distribution target, add:

```bash
jffi doctor bundle --platform <platform> --release --profile release
jffi bundle --platform <platform> --profile release --dry-run --print-plan
```

The dry run confirms the bundle plan without creating or uploading a release.
It also leaves generated native projects unchanged. For troubleshooting an
actual bundle, `--print-commands` prints native commands with secrets redacted.

## Runner matrix

| Target | Required runner |
| --- | --- |
| iOS and macOS | macOS with Xcode |
| Android | macOS, Linux, or Windows with Android SDK/NDK |
| Windows | Windows with .NET and Windows build tools |
| Linux | Linux with GTK/Libadwaita and Flatpak tooling |
| Web | macOS, Linux, or Windows with Rust, Node.js, and npm |

Do not claim full release readiness from a single host when that host cannot
execute the native build for every enabled platform.

## Secrets and signing assets

Keep these outside the repository:

- Apple distribution certificates, private keys, provisioning profiles, App
  Store Connect credentials, and notarization credentials
- Android keystores and their passwords
- Windows code-signing certificates and private keys
- Store upload tokens and service-account credentials

`jffi.toml` may contain certificate names, profile names, key aliases, file
paths, and environment-variable names. CI injects the corresponding protected
values only into release jobs. Never print, archive, or cache unlocked signing
material.

## Example GitHub Actions quality gate

This example validates the framework-independent checks. Add platform jobs on
their native runners for the targets your application enables.

```yaml
name: quality

on:
  pull_request:
  push:
    branches: [main]

jobs:
  rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - run: cargo install jffi --locked
      - run: jffi doctor config
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace
```

Pin third-party actions to reviewed commit SHAs in security-sensitive
repositories. The tags above keep the example readable; production policy
should decide the exact pinning strategy.

## Example release shape

```yaml
on:
  push:
    tags: ["v*"]
  workflow_dispatch:
```

Each platform release job should then:

1. Check out the exact tag.
2. Install a pinned JFFI version with `cargo install jffi --version <version> --locked`.
3. Restore or install the required native SDKs.
4. Import signing assets from protected secrets.
5. Run `jffi doctor bundle --platform <platform> --release --profile release`.
6. Run `jffi bundle --platform <platform> --profile release`.
7. Verify signatures and archive the resulting artifact.
8. Upload to the store only from a protected environment.

If the application changes `jffi.toml`, the quality workflow should validate
the change before a release tag is created. External account state—expired
certificates, revoked profiles, store agreements, quotas, and service outages—
can still fail a correctly configured release and must be monitored separately.
