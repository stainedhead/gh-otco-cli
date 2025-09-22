# GH OTCO CLI

<!-- Badges -->
[![CI](https://github.com/stainedhead/gh-otco-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/stainedhead/gh-otco-cli/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/stainedhead/gh-otco-cli/branch/main/graph/badge.svg)](https://codecov.io/gh/stainedhead/gh-otco-cli)

A cross-platform Rust CLI to explore and research GitHub data via the REST API. The CLI mirrors GitHub’s resource hierarchy (org, repo, issues, prs, actions, security) and produces results in JSON, YAML, CSV, PSV, or table format. API access logic is modular and reusable as a separate crate.

## Quickstart
- Build: `cargo build`
- Auth (PAT): `cargo run -- auth login --token <PAT>`
- Who am I: `cargo run -- auth whoami --output table`
- Rate limit: `cargo run -- meta rate-limit --output json`

## Usage Examples
- List org repos: `cargo run -- org repos my-org --type public --pages 2 --output table`
- List issues: `cargo run -- issues list my-org/my-repo --state open --labels bug --fields number,title,state --sort -number`
- List PRs: `cargo run -- prs list my-org/my-repo --state all --output yaml`
- Actions runs: `cargo run -- actions runs my-org/my-repo --branch main --status completed --all --fields id,conclusion,head_branch`
- Security (Dependabot): `cargo run -- security dependabot my-org/my-repo --severity high,critical --output json`

Global output controls: `--output json|yaml|csv|psv|table`, `--fields a,b,c`, `--sort field|-field`, `--limit N`, `--all` (page-through).

## Configuration & Auth
- Precedence: config file < env < CLI.
  - Files: `./gh-otco.{toml|yaml|json}` or `~/.gh-otco.{toml|yaml|json}`
  - Env: `GITHUB_API_URL`, `GITHUB_TOKEN`, `OTEL_EXPORTER_OTLP_ENDPOINT`
  - CLI: `--api-url`, `--output`, etc.
- Manage config: `cargo run -- config init`, `config get <key>`, `config set <key> <value>`.
- Credentials: PAT stored securely via OS keychain (`keyring`). OAuth device flow planned.

## Project Layout
- Workspace crates:
  - `crates/api` (`gh-otco-api`): reusable GitHub REST client
  - `crates/cli` (`gh-otco-cli`): CLI, config, auth, output
- Docs: see `AGENTS.md`, `docs/product-requirements-doc.md`, and `docs/technical-design-doc.md`.

## Development
- Format: `cargo fmt --all`
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`
- Test: `cargo test --workspace`
- Optional OTEL: build with `--features otel` and set `OTEL_EXPORTER_OTLP_ENDPOINT`.

## Platform Support
macOS, Linux, and Windows. Paths and credential storage use cross-platform crates.

## Command Reference
The table below is auto-generated from clap.

<!-- AUTO-GENERATED: COMMANDS BEGIN -->
<!-- AUTO-GENERATED: COMMANDS END -->

Regenerate: `cargo run -p gh-otco-cli -- docs readme`

## Token Scopes
- Auth: `auth whoami` works without special scopes on public data; for private user data use `read:user`.
- Org: listing org repos may require `read:org` for private org data.
- Repo: private repositories require `repo` scope.
- Issues/PRs: private repo data requires `repo`; public-only works anonymously but is rate-limited.
- Actions: `actions:read` (fine-grained) or `repo` (classic) for private repos.
- Security:
  - Dependabot alerts: `security_events` (private); public repos may require no extra scope.
  - Code scanning: `security_events` (read access).
  - Secret scanning: `security_events` (read access) where enabled.
