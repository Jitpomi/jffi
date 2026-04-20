# cat

Cross-platform app built with Rust + UniFFI

## Platforms

- android

## Quick Start

```bash
# Build for your platform
jffi build --platform android

# Run the app
jffi run --platform android

# Development mode (auto-rebuild)
jffi dev --platform android
```

## Project Structure

- `core/` - Business logic (pure Rust)
- `ffi/` - FFI layer (UniFFI exports)
- `platforms/` - Platform-specific UIs

## Development

Edit your business logic in `core/src/lib.rs`. The FFI bindings will be automatically regenerated.

## Adding Features

1. Add logic to `core/src/lib.rs`
2. Expose via `#[uniffi::export]` in `core/src/lib.rs`
3. Rebuild: `jffi build --platform <platform>`
4. Update UI in `platforms/<platform>/`

Built with [UniFFI Framework](https://github.com/mozilla/uniffi-rs)
