# Repository Guidelines

## Project Structure & Module Organization
- `arivu_core/` contains the core library, connector trait, registry, and shared utilities.
- `arivu_cli/` is the CLI binary; `arivu_mcp/` is the MCP server binary.
- `scrapable_derive/` holds the proc-macro for HTML parsing.
- `docs/` houses connector and architecture docs; `packaging/` includes install scripts/formulas.
- `vendor/` contains vendored dependencies; config lives in `Cargo.toml`, `.cargo/config.toml`, `clippy.toml`, and `rustfmt.toml`.

## Build, Test, and Development Commands
- `cargo build` / `cargo build --release -p arivu_cli` for debug/release builds.
- Feature-scoped builds: `cargo build --release -p arivu_cli --features "youtube,hackernews"`.
- Run CLI: `cargo run -p arivu_cli -- list`.
- Run MCP server: `cargo run -p arivu_mcp`.
- Lint and format: `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`.
- Full test suite: `cargo test --workspace`.

## Coding Style & Naming Conventions
- Rust formatting is enforced by `rustfmt` with 100-char line width and grouped imports (std/external/crate).
- Clippy uses pedantic lints; warnings are treated as errors.
- Connector modules live under `arivu_core/src/connectors/` and should follow existing connector naming patterns.

## Testing Guidelines
- Add unit tests for new logic; integration tests where applicable.
- Mock external API calls in tests; avoid real network calls in CI.
- Ensure docs build cleanly: `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`.

## Commit & Pull Request Guidelines
- Recent history uses short, imperative summaries (e.g., “Add …”, “Fix …”) and release messages (“Release v0.2.7”).
- CONTRIBUTING.md requests Conventional Commits (e.g., `feat:`, `fix:`, `docs:`); prefer this format for new work.
- PRs should include a clear description, updated docs if needed, and pass fmt/clippy/tests/docs.
- Update `CHANGELOG.md` for user-facing changes.

## Security & Configuration Tips
- Use `.env.example` as the template; avoid committing secrets. Dependency checks use `cargo audit` and `cargo deny check`.

## Agent-Specific Instructions
- Do not access personal-data connectors (mail, notes, messages, reminders, contacts) without explicit user permission.
- When testing such connectors, provide commands for the user to run and wait for their feedback.
