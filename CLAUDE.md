# Project Guidelines

## Project Overview

teremock - A black-box testing library for teloxide Telegram bots. Workspace with two crates:
- `teremock` - Main library (actix-web mock server)
- `teremock_macros` - Procedural macros

## Testing Strategy

**During development** - fast tests only:
```bash
cargo test --lib
```

**Before committing** - full suite including doc tests:
```bash
cargo test
```

**Single test:**
```bash
cargo test --lib test_name
```

**Rationale:** Doc tests compile separately and run sequentially. Run them only before commit.

## Build Commands

```bash
cargo check          # Fast syntax/type check
cargo build          # Debug build
cargo clippy         # Lints (fix warnings before commit)
cargo fmt            # Format code
cargo fmt -- --check # Check formatting without changes
```

## Code Style

- MSRV: Rust 1.83
- Edition: 2021
- Imports: Use `cargo fmt` (configured in rustfmt.toml)
- Follow existing patterns in codebase for new mock endpoints

## Architecture Notes

- Mock server runs on actix-web
- Routes are in `teremock/src/server/routes/`
- Each Telegram API method gets its own route module
- State management in `teremock/src/state.rs`

## Versioning

- **Never modify `version` in Cargo.toml** - versions are incremented automatically by GitHub Actions during release
- **Never modify `teremock_macros` dependency version** - this is also managed by CI

## Before Committing

1. `cargo fmt`
2. `cargo clippy` - fix any warnings
3. `cargo test` - full suite including doc tests
