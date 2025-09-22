# Technical Design Document

## Repository Layout
- `crates/api` (`gh-otco-api`): Reusable GitHub REST client. Handles HTTP, headers, pagination helpers, and minimal models.
- `crates/cli` (`gh-otco-cli`): CLI layer with `clap`, config/env/CLI precedence, secure token storage, output formatting, and observability.
- `docs/`: Product docs (PRD) and technical design docs.
- `.github/workflows/`: CI pipelines (build, lint, test, coverage).
- `Cargo.toml`: Workspace manifest defining members and resolver.
- `README.md` / `AGENTS.md`: Usage, contributor, and repo guidelines.

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
- Formats: JSON, YAML, CSV, PSV, table. Array outputs normalized to rows with projection (`--fields`), sort (`--sort`), limit (`--limit`). Output can be written to files via `--output-file`.
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
- Headers: add `Accept: application/vnd.github+json`, `User-Agent: gh-otco-cli`, `X-GitHub-Api-Version: 2022-11-28`. HTTP client timeout: 30s.

## Core Crates
- `clap`: Command definitions, parsing, and help UX.
- `reqwest` + `rustls`: HTTP client with TLS for REST calls.
- `serde` (+ `serde_json`, `serde_yaml`): Serialization of inputs/outputs and models.
- `tokio`: Async runtime for HTTP and I/O tasks.
- `tracing`, `tracing-subscriber`: Structured logs and filters.
- `tracing-opentelemetry`, `opentelemetry-otlp` (feature `otel`): Export traces to OTLP.
- `keyring`: OS-native secure storage for tokens.
- `csv`, `comfy-table`: Delimited and tabular output formatting.
- `anyhow`, `thiserror`: Error ergonomics and domain errors.

## Observability
- Logging: `tracing` + `tracing-subscriber` with env filter; no timestamps by default.
- OpenTelemetry: optional feature `otel` enabling OTLP exporter via `OTEL_EXPORTER_OTLP_ENDPOINT`.
- Shutdown: tracer provider is flushed on exit (when feature enabled).

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
