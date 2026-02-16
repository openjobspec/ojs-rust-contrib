# Contributing to OJS Rust Contrib

Thank you for your interest in contributing to OJS Rust Contrib!

## Adding a New Integration Crate

1. **Create a directory** named `ojs-{framework}/` at the workspace root.

2. **Add a `Cargo.toml`** with workspace metadata inheritance:
   ```toml
   [package]
   name = "ojs-{framework}"
   version = "0.1.0"
   edition.workspace = true
   rust-version.workspace = true
   license.workspace = true
   repository.workspace = true
   ```

3. **Required files:**
   - `README.md` — Crate name, installation, quick usage, API summary, link to examples
   - `Cargo.toml` — Package manifest with `ojs` + framework dependencies
   - `src/lib.rs` — Public API re-exports
   - `tests/integration.rs` — Integration tests (use mock HTTP, no real OJS backend)
   - `examples/` — Complete runnable example with its own `Cargo.toml` and Docker Compose

4. **Register the crate** in the workspace `Cargo.toml` `members` list.

5. **Update the root `README.md`** status table with your new integration.

6. **Update the CI matrix** in `.github/workflows/ci.yml`.

## Crate Guidelines

- Keep dependencies minimal: only the framework + OJS SDK.
- Use idiomatic Rust patterns: `Result` types, trait-based abstractions, zero-cost abstractions.
- Provide middleware or extractors that inject `ojs::Client` into the framework's request handling.
- Support graceful shutdown by integrating with the framework's shutdown hooks.
- Tests should use mock HTTP servers — no real OJS backend required.
- All crates use the `tokio` runtime.

## Example Guidelines

Each example should include:
- `docker-compose.yml` with `ojs-backend-redis` and Redis for integration demos
- `Cargo.toml` with path dependencies pointing to the parent crate
- `src/main.rs` — HTTP server that enqueues jobs
- `src/worker.rs` (if applicable) — Worker that processes jobs
- `README.md` — Prerequisites, setup, and run instructions

## Code Style

- Run `cargo fmt` before submitting.
- Run `cargo clippy` and resolve all warnings.
- Document all public types and functions with doc comments.
- Keep exported APIs small and focused.

## Pull Request Process

1. Fork the repository and create a feature branch.
2. Ensure all tests pass: `cargo test --workspace`
3. Ensure linting passes: `cargo clippy --workspace -- -D warnings`
4. Ensure formatting: `cargo fmt --all -- --check`
5. Submit a pull request with a clear description.
