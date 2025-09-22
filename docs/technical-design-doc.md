# Technical Design Document

## Architecture
- Workspace with crates:
  - `gh-otco-api`: reusable GitHub REST client (async, `reqwest` + `serde`).
  - `gh-otco-cli`: CLI layer (`clap`) calling API crate; handles config, auth, I/O.
- Async runtime: `tokio`.
- HTTP headers: `User-Agent: gh-otco-cli`, `Accept: application/vnd.github+json`.

## Configuration & Auth
- Precedence: config file (`./gh-otco.{toml|yaml|json}` → `~/.gh-otco.{toml|yaml|json}`) < env (`GITHUB_API_URL`, `GITHUB_TOKEN`) < CLI flags (`--api-url`, `--output`).
- Commands: `config init|get|set` for `github.api_url`, `output.format`, `pagination.per_page`.
- Tokens: PAT via `keyring` (per-host service `gh-otco::<host>`). `auth login|logout|whoami`.
- OAuth device flow: planned; stored via `keyring` when implemented.

## CLI Command Model
- Top-level groups: `auth`, `meta`, `org`, `repo`, `issues`, `prs`, `actions`, `security`, `config`.
- Patterns mirror GitHub resources; read-first posture.
- Global flags: `--output`, `--fields`, `--sort`, `--limit`, `--all`, `--log-level`.

## Output & Formatting
- Formats: JSON, YAML, CSV, PSV, table. Array outputs normalized to rows with projection (`--fields`), sort (`--sort`), limit (`--limit`).
- Table: `comfy-table`; CSV/PSV: `csv` crate; serialization via `serde`.

## API Client
- Base URL default `https://api.github.com`; override for GHES.
- Endpoints implemented:
  - Meta: `/rate_limit`, `/user`.
  - Org repos: `/orgs/{org}/repos`.
  - Repo: issues `/repos/{o}/{r}/issues` (filters: state, labels, assignee, milestone, since), pulls `/repos/{o}/{r}/pulls` (filters: state, draft, base).
  - Actions: workflows `/actions/workflows`, runs `/actions/runs` (filters: branch, status, conclusion).
  - Security: dependabot `/dependabot/alerts`, code scanning `/code-scanning/alerts`, secret scanning `/secret-scanning/alerts`.
- Pagination: `per_page`, `page`, with `--all` to page until empty; default page cap to prevent runaway.
- Error handling: `thiserror` domain errors in API; `anyhow` at CLI boundary with context.

## Observability
- Logging: `tracing` + `tracing-subscriber` with env filter; no timestamps by default.
- OpenTelemetry: optional feature `otel` enabling OTLP exporter via `OTEL_EXPORTER_OTLP_ENDPOINT`.

## Cross-Platform Considerations
- Paths: `home`/`dirs` for config discovery; no Unix-specific syscalls.
- Credentials: `keyring` supports macOS Keychain, Windows Credential Manager, and Secret Service on Linux.

## Testing Strategy
- Unit tests: colocated `#[cfg(test)]` for helpers (config resolution, output projection, parsing).
- Integration tests: mock HTTP (`httpmock`/`wiremock`) for pagination and error scenarios; avoid live API.
- CI: GitHub Actions matrix (Linux/macOS/Windows) running build, fmt, clippy, and tests.

## Risks & Mitigations
- Rate limits: add backoff on 403/429 using headers; provide `--all` with caution.
- Schema drift: prefer typed models for stable surfaces; default to `serde_json::Value` for pass-throughs.
- Permissions: document scopes per command; degrade with clear errors when insufficient.

