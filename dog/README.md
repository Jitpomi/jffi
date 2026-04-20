# dog

Cross-platform app built with Rust + UniFFI

## Platforms

- ios

## Quick Start

```bash
# Build for your platform
uniffi-app build --platform ios

# Run the app
uniffi-app run --platform ios

# Development mode (auto-rebuild)
uniffi-app dev --platform ios
```

## Project Structure

- `core/` - Business logic (pure Rust)
- `ffi/` - FFI layer (UniFFI exports)
- `platforms/` - Platform-specific UIs

## Development

Edit your business logic in `core/src/lib.rs`. The FFI bindings will be automatically regenerated.

## Adding Features

1. Add logic to `core/src/lib.rs`
2. Expose via FFI in `ffi/src/lib.rs`
3. Rebuild: `uniffi-app build --platform <platform>`
4. Update UI in `platforms/<platform>/`

Built with [UniFFI Framework](https://github.com/mozilla/uniffi-rs)
