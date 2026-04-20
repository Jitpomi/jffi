# rt

Cross-platform app built with Rust + UniFFI

## Platforms

- ios

## Quick Start

```bash
# Build for your platform
jffi build --platform ios

# Run the app
jffi run --platform ios

# Development mode (auto-rebuild)
jffi dev --platform ios
```

## Project Structure

- `core/` - Business logic (pure Rust)
- `ffi-web/` - WASM FFI layer (for web platform)
- `platforms/` - Platform-specific UIs

## Development

Edit your business logic in `core/src/lib.rs`. The FFI bindings will be automatically regenerated.

## Adding Features

1. Add logic to `core/src/lib.rs`
2. Expose via `#[uniffi::export]`
3. Rebuild: `jffi build --platform <platform>`
4. Update UI in `platforms/<platform>/`

Built with [JFFI](https://github.com/yourusername/jffi)
