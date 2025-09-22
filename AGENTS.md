# Repository Guidelines
This document outlines best practices for organizing, developing, testing, and maintaining Rust CLI applications in this repository. Following these guidelines helps ensure code quality, consistency, and ease of collaboration.

## Product Documentation
The documentation for this project lives in the `docs/` directory.
- `docs/product-requirements-doc.md`: Product Requirements Document (PRD). Keep this current and use it as context for all changes.
- `docs/technical-design-doc.md`: Technical design details for developers, architects, and SREs. Keep this current as the system evolves.
- `README.md`: First entry point for new contributors and product stakeholders. Keep usage, setup, and contribution details current.

## Repository Layout
- `crates/api` (`gh-otco-api`): Reusable GitHub REST client for HTTP, headers, pagination helpers, and minimal models.
- `crates/cli` (`gh-otco-cli`): CLI commands (clap), config/env precedence, secure token storage, formatting, and observability.
- `docs/`: Product (PRD) and technical design documentation.
- `.github/workflows/`: CI for build, lint, test, and coverage reports.
- `Cargo.toml`: Workspace manifest and members.
- `README.md` / `AGENTS.md`: Usage, contribution, and guidelines.

## Core Crates
- `clap`: Command definitions, parsing, help UX.
- `reqwest` (+ `rustls`): HTTP client for GitHub REST.
- `serde` (`serde_json`, `serde_yaml`): Models and serialization.
- `tokio`: Async runtime for HTTP/I/O.
- `tracing`, `tracing-subscriber`: Structured logs and filtering.
- `tracing-opentelemetry`, `opentelemetry-otlp` (feature `otel`): OTEL export.
- `keyring`: OS-native secure token storage.
- `csv`, `comfy-table`: Delimited and tabular output.
- `anyhow`, `thiserror`: Ergonomic and domain error handling.

## Project Structure & Module Organization
- `src/`: Main library and CLI entry (`main.rs`).
- `src/bin/`: Additional binaries (e.g., `src/bin/otco.rs`).
- `tests/`: Integration tests (`tests/*.rs`).
- `examples/`, `benches/`: Optional examples/benchmarks.
- `Cargo.toml`: Package metadata and dependencies.

Suggested layout for a multi-bin CLI:
- `src/lib.rs` for shared logic; thin binaries in `src/bin/*.rs` call into the library.

## Build, Test, and Development Commands
- `cargo build`: Compile in debug mode.
- `cargo run -- <args>`: Run the CLI locally (e.g., `cargo run -- --help`).
- `cargo test`: Run unit + integration tests.
- `cargo fmt --all`: Format code with rustfmt.
- `cargo clippy --all-targets -- -D warnings`: Lint with Clippy and deny warnings.
- `cargo doc --open`: Build and open docs.

## Coding Style & Naming Conventions
- Use rustfmt defaults (4-space indent, max width per config).
- Naming: `snake_case` for functions/modules/files, `CamelCase` for types/traits, `SCREAMING_SNAKE_CASE` for constants.
- Keep binaries minimal; place reusable code in `lib.rs`.
- Prefer `anyhow`/`thiserror` for error handling and `tracing`/`log` for diagnostics when applicable.

## Testing Guidelines
- We practice TDD. Write a failing test first (red), implement the minimal code to pass (green), then refactor while keeping tests green.
- Unit tests colocated in modules via `#[cfg(test)] mod tests { ... }`.
- Integration tests in `tests/`, one file per feature (e.g., `tests/commands_test.rs`).
- Cover command parsing, config precedence, pagination, output formatting, and error paths.
- Run locally: `cargo test`; add `-- --nocapture` to see output.
- Coverage (optional, local): `cargo llvm-cov --workspace` or `cargo tarpaulin` (Linux). Do not merge features without accompanying tests.

## Commit & Pull Request Guidelines
- Commits: concise, imperative subject (≤72 chars), include scope when helpful (e.g., `feat(parser): support labels`).
- Reference related issues with `Closes #123` in the body.
- PRs: clear description, rationale, before/after behavior, CLI examples, and screenshots for output changes.
- Ensure CI passes: build, tests, `fmt`, and `clippy`.

## Security & Configuration Tips
- Never commit secrets. Use environment variables for tokens (e.g., `GITHUB_TOKEN`).
- Validate user input and handle network errors gracefully.
- Document required environment/config in the PR description and `README` updates when adding features.
