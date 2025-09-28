# Repository Guidelines

## Project Structure & Module Organization
- `src/`: Rust crate entry points and helpers (`main.rs`, `lib.rs`, `common.rs`).
- `tests/`: async integration tests (`*_test.rs`).
- `masm/`: Miden Assembly assets (`accounts/`, `scripts/`, `notes/`, `auth/`).
- `scripts/`: helper shells to run a local node and tests.
- Runtime data: `keystore/` and `store.sqlite3` (created at run time).

## Build, Test, and Development Commands
- Build: `cargo build` (use `--release` for optimized binaries).
- Run demo: `cargo run --release` (connects to Miden Testnet by default).
- Unit/IT tests: `cargo test -- --nocapture` (show logs); when a local node is required use: `scripts/start_node_and_test.sh`.
- Optional lint/format: `cargo fmt --all` and `cargo clippy -- -D warnings`.

## Coding Style & Naming Conventions
- Rust edition 2024; follow `rustfmt` defaults (4‑space indent, trailing commas, ~100 cols).
- Naming: modules/files `snake_case` (e.g., `utils.rs`), types `CamelCase`, functions/vars `snake_case`.
- MASM files use `snake_case.masm` under domain folders (e.g., `masm/scripts/resolve_name.masm`).
- Keep functions focused and async-safe; prefer `?` for error propagation.

## Testing Guidelines
- Framework: `tokio::test` for async integration tests in `tests/`.
- Naming: `*_test.rs` with descriptive function names.
- Network: some tests hit Testnet (`Endpoint::testnet()`); for local workflows use scripts in `scripts/` (RPC defaults to `127.0.0.1:57291`).
- Deterministic IT run: `cargo test --release -- --nocapture --test-threads=1`.

## Commit & Pull Request Guidelines
- Commits: short imperative subject (≤72 chars), optional body with rationale.
  - Examples: `fix storage slot write`, `add note consumption test`.
- PRs: include purpose, key changes, test instructions, and linked issues. Add logs/screenshots when relevant.
- Pre-push: `cargo fmt --all && cargo clippy -- -D warnings && cargo test`.

## Security & Configuration Tips
- Keystore/store: `common::delete_keystore_and_store()` wipes `./keystore` and `./store.sqlite3`. Use carefully; do not run on machines with valuable keys.
- Scripts env: `RPC_HOST`, `RPC_PORT`, `READY_TIMEOUT_SEC` can tune `scripts/start_node_and_test.sh`.
- Secrets: never commit keys or SQLite artifacts; `.gitignore` already excludes them.

## Architecture Overview
- Thin Rust layer over `miden-client` with helpers to assemble MASM (`masm/`) into libraries/scripts and drive transactions against a node (Testnet by default).

## References
- Miden VM & MASM docs: https://0xmiden.github.io/miden-docs/imported/miden-vm/src/intro/main.html
- Useful links for miden: https://0xmiden.github.io/miden-docs/imported/awesome-miden/index.html
