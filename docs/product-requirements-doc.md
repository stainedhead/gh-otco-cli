# Product Requirements Document (PRD)

## Summary
- Product name: GH OTCO CLI
- Value proposition: A cross-platform Rust CLI for exploring and researching GitHub data via the REST API, with reusable API logic for other apps.
- Target users: Developers, SREs, and data-minded engineers analyzing org/repo activity.
- Success metrics: Time-to-answer for queries (<3s for common ops), coverage of core REST resources (org/repo/issues/PRs/actions/security), export versatility (JSON/YAML/CSV/PSV/table), adoption and CI usage.
- Non-goals: Git operations; replicating all `gh` features; UI/desktop app.

## Overview
- Problem statement: Developers need fast, scriptable introspection of GitHub orgs/repos/issues/PRs/actions/security with consistent output for analysis and automation.
- Context: CLI-first; API logic must be reusable as a crate for future tools/services. Must support macOS, Linux, Windows.
- Key user stories:
  - As a developer, I can list/filter/export repos, issues, and PRs for an org or user to analyze health and activity.
  - As an SRE, I can retrieve Actions workflow runs and failures across repos to troubleshoot and report.
  - As a security engineer, I can query Dependabot and code scanning alerts and export them for dashboards.
- Assumptions/constraints: Offline not required; GitHub.com and GHES supported via configurable host; handle pagination and rate limits gracefully; output selectable format.
- Dependencies/risks: GitHub REST API rate limits/scopes; OAuth device flow; secure token storage varies by OS; enterprise instances may have different API bases.

## Architectural Decisions
- Approach: Library-first. Separate crates/modules: `api` (GitHub REST client) and `cli` (command parsing, I/O). Thin binaries call into the API crate within a Cargo workspace.
- Command parsing: `clap` with subcommands mirroring GitHub hierarchy: `org`, `repo`, `team`, `user`, `issues`, `prs`, `actions`, `security`, `rate-limit`, `meta`.
- HTTP/client: Prefer `octocrab` for REST ergonomics or `reqwest` + typed models via `serde` if finer control is needed. Async runtime: `tokio`.
- Config/credentials: File → env → CLI override order (CLI has highest precedence). Config file searched in `./gh-otco.{toml|yaml|json}` then `~/.gh-otco.{toml|yaml|json}`.
- Auth: PAT via OS keychain using `keyring`. Optional OAuth2 device flow using `oauth2` crate; `login` stores token securely; `logout` removes it.
- Output: `--output json|yaml|csv|psv|table`; field selection via `--fields`; stable schemas via `serde` models. CSV/PSV via `csv` crate; table via `comfy-table`.
- Observability: `tracing` + `tracing-subscriber` with OpenTelemetry (`tracing-opentelemetry`, `opentelemetry-otlp`). Configurable `--log-level` and OTEL envs. OTEL exporter behind an optional feature (`--features otel`).
- Errors: `thiserror` for domain errors; `anyhow` at CLI boundary with rich context; retry/backoff for 5xx/secondary rate limits.
- Portability: Use `dirs`/`home` for paths; no Unix-only features. Tested on macOS/Linux/Windows.

## Features
- Auth & Config
  - `login` (device flow or PAT prompt), `logout`, `whoami`.
  - Config precedence: file < env < CLI. Keys: `github.host`, `auth.method`, `output.format`, `pagination.per_page`, `enterprise.api_base`, etc.
  - `config` command: `config init`, `config get <key>`, `config set <key> <value>`.
  - Accept env defaults: `GITHUB_TOKEN`, `GITHUB_API_URL`, `OTEL_EXPORTER_OTLP_ENDPOINT`.

- Org
  - List orgs, get org, members, teams, repos. Summaries: counts by visibility/language.
  - Example: `otco org repos <org> --visibility public --output table`.

- Repo
  - List/find repos; details (topics, languages), branch protection, releases, traffic (views/clones), contributors.
  - Example: `otco repo list <org> --topics rust --fields name,stars,archived`.

- Issues
  - List/filter by state, labels, assignees, milestones; counts and aging buckets.
  - Example: `otco issues list <org>/<repo> --state open --since 30d --output csv`.

- PRs
  - List/filter by state, draft, review status; mergeability; lead time stats (created→merged); reviewers summary.
  - Example: `otco prs list <org>/<repo> --state open --review-required`.

- Actions
  - List workflows and runs; filter by conclusion/branch; get artifacts; rerun (optional); usage summary per repo.
  - Example: `otco actions runs <org>/<repo> --workflow build.yml --last 50`.

- Security
  - Dependabot alerts, code scanning alerts, secret scanning alerts (where authorized). Export-friendly schemas.
  - Example: `otco security alerts <org>/<repo> --severity high,critical --output json`.

- Users/Teams
  - Get user; list teams and members; repo access audit for a team or user.

- Meta & Rate Limits
  - Show REST/Graph rate limits and remaining; API metadata; suggest backoff when near limits.

- Formatting & Export
  - `--output` formats: json, yaml, csv, psv, table; `--fields` selection; `--sort`/`--limit` controls; `--all` to page through; `--file` to write output.

- Reliability
  - Pagination handling, backoff on `403` secondary rate limits, partial-failure reporting with structured errors.

Open questions
- Should we support GraphQL v4 for advanced queries later?
- Which minimal token scopes are required per command by default vs prompting to elevate?
- Do we include write actions (e.g., label creation), or read-only v1 release scope?

References
- Product docs live in `docs/`. PRD: `docs/product-requirements-doc.md`. Technical design: `docs/technical-design-doc.md` (kept current with implementation details).
