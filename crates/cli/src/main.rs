use anyhow::{Context, Result};
use clap::{Command, CommandFactory, Parser, Subcommand, ValueEnum};
use comfy_table::{presets::UTF8_FULL, Table};
use gh_otco_api::GitHubClient;
use home::home_dir;
use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::{Path, PathBuf}};
use tracing::warn;
use tracing_subscriber::{fmt, EnvFilter};
#[cfg(feature = "otel")]
use tracing_subscriber::Registry;
#[cfg(feature = "otel")]
use tracing_subscriber::prelude::*;
#[cfg(feature = "otel")]
use tracing_opentelemetry::OpenTelemetryLayer;
#[cfg(feature = "otel")]
use opentelemetry::sdk::{self, trace as sdktrace};

#[derive(Copy, Clone, Debug, ValueEnum)]
enum OutputFormat {
    Json,
    Yaml,
    Csv,
    Psv,
    Table,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct FileConfig {
    #[serde(default)]
    github: GitHubSection,
    #[serde(default)]
    output: OutputSection,
    #[serde(default)]
    pagination: PaginationSection,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct GitHubSection {
    #[serde(default = "default_api_url")] // default to public GitHub
    api_url: String,
    #[serde(default)]
    host: Option<String>,
}

fn default_api_url() -> String { "https://api.github.com".into() }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct OutputSection {
    #[serde(default = "default_output_format")] 
    format: String,
}

fn default_output_format() -> String { "table".into() }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PaginationSection {
    #[serde(default)]
    per_page: Option<u32>,
}

#[derive(Parser, Debug)]
#[command(name = "otco", version, about = "GitHub data exploration CLI")] 
struct Cli {
    /// Path to config file (toml|yaml|json)
    #[arg(long)]
    config: Option<PathBuf>,

    /// Output format (overrides config/env)
    #[arg(long, value_enum)]
    output: Option<OutputFormat>,

    /// GitHub API base URL (public or GHES)
    #[arg(long)]
    api_url: Option<String>,

    /// Log level (error,warn,info,debug,trace)
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Enable paging through all results (array outputs)
    #[arg(long, global = true, default_value_t = false)]
    all: bool,

    /// Projected fields (comma-separated) applied to array outputs
    #[arg(long, global = true)]
    fields: Option<String>,

    /// Sort by field (prefix with '-' for descending)
    #[arg(long, global = true)]
    sort: Option<String>,

    /// Limit number of rows in array outputs
    #[arg(long, global = true)]
    limit: Option<usize>,

    /// Write output to a file instead of stdout
    #[arg(long, global = true)]
    output_file: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Authentication and token management
    Auth {
        #[command(subcommand)]
        cmd: AuthCmd,
    },
    /// Show API metadata and rate limits
    Meta {
        #[command(subcommand)]
        cmd: MetaCmd,
    },
    /// Organization-level commands
    Org {
        #[command(subcommand)]
        cmd: OrgCmd,
    },
    /// Repository discovery commands
    Repo {
        #[command(subcommand)]
        cmd: RepoCmd,
    },
    /// Repository issues
    Issues {
        #[command(subcommand)]
        cmd: IssuesCmd,
    },
    /// Repository pull requests
    Prs {
        #[command(subcommand)]
        cmd: PrsCmd,
    },
    /// GitHub Actions
    Actions {
        #[command(subcommand)]
        cmd: ActionsCmd,
    },
    /// Security alerts
    Security {
        #[command(subcommand)]
        cmd: SecurityCmd,
    },
    /// Configuration management
    Config {
        #[command(subcommand)]
        cmd: ConfigCmd,
    },
    /// Generate docs from clap definitions
    Docs {
        #[command(subcommand)]
        cmd: DocsCmd,
    },
}

#[derive(Subcommand, Debug)]
enum AuthCmd {
    /// Log in using a Personal Access Token (PAT) or device flow (future)
    Login {
        /// PAT token (will prompt if omitted)
        #[arg(long)]
        token: Option<String>,
        /// Use OAuth Device Flow (prints user code and URL)
        #[arg(long, default_value_t = false)]
        device: bool,
        /// API URL host key for storage (defaults to derived host)
        #[arg(long)]
        host: Option<String>,
    },
    /// Remove stored credentials
    Logout {
        /// API URL host key for storage (defaults to derived host)
        #[arg(long)]
        host: Option<String>,
    },
    /// Show current user
    Whoami,
}

#[derive(Subcommand, Debug)]
enum MetaCmd {
    /// Display GitHub API rate-limit information
    RateLimit,
}

#[derive(Subcommand, Debug)]
enum OrgCmd {
    /// List repositories within an organization
    Repos {
        /// Organization login
        org: String,
        /// Repo type: all, public, private, forks, sources, member
        #[arg(long, value_parser = ["all","public","private","forks","sources","member"].into_iter().collect::<Vec<_>>())]
        r#type: Option<String>,
        /// Per-page (1-100)
        #[arg(long, default_value_t = 100)]
        per_page: u32,
        /// Max pages to fetch
        #[arg(long, default_value_t = 1)]
        pages: u32,
    },
}

#[derive(Subcommand, Debug)]
enum RepoCmd {
    /// List repositories for an org (alias of org repos)
    List {
        /// Organization login
        org: String,
        /// Repo type: all, public, private, forks, sources, member
        #[arg(long)]
        r#type: Option<String>,
        /// Per-page (1-100)
        #[arg(long, default_value_t = 100)]
        per_page: u32,
        /// Max pages to fetch
        #[arg(long, default_value_t = 1)]
        pages: u32,
    },
}

#[derive(Subcommand, Debug)]
enum IssuesCmd {
    /// List issues for a repository
    List {
        /// Repository in the form owner/name
        repo: String,
        /// State: open, closed, all
        #[arg(long)]
        state: Option<String>,
        /// Comma-separated labels
        #[arg(long)]
        labels: Option<String>,
        /// Assignee username
        #[arg(long)]
        assignee: Option<String>,
        /// Milestone title or number
        #[arg(long)]
        milestone: Option<String>,
        /// Updated since (ISO8601, e.g. 2024-01-01T00:00:00Z)
        #[arg(long)]
        since: Option<String>,
        /// Per-page (1-100)
        #[arg(long, default_value_t = 100)]
        per_page: u32,
        /// Max pages to fetch
        #[arg(long, default_value_t = 1)]
        pages: u32,
    },
}

#[derive(Subcommand, Debug)]
enum PrsCmd {
    /// List pull requests for a repository
    List {
        /// Repository in the form owner/name
        repo: String,
        /// State: open, closed, all
        #[arg(long)]
        state: Option<String>,
        /// Include draft PRs only if true
        #[arg(long)]
        draft: Option<bool>,
        /// Base branch filter
        #[arg(long)]
        base: Option<String>,
        /// Per-page (1-100)
        #[arg(long, default_value_t = 100)]
        per_page: u32,
        /// Max pages to fetch
        #[arg(long, default_value_t = 1)]
        pages: u32,
    },
}

#[derive(Subcommand, Debug)]
enum ActionsCmd {
    /// List workflows in a repository
    Workflows {
        /// Repository in the form owner/name
        repo: String,
    },
    /// List workflow runs with filters
    Runs {
        /// Repository in the form owner/name
        repo: String,
        /// Filter by branch
        #[arg(long)]
        branch: Option<String>,
        /// Status: queued, in_progress, completed
        #[arg(long)]
        status: Option<String>,
        /// Conclusion: success, failure, etc.
        #[arg(long)]
        conclusion: Option<String>,
        /// Per-page (1-100)
        #[arg(long, default_value_t = 100)]
        per_page: u32,
        /// Max pages to fetch
        #[arg(long, default_value_t = 1)]
        pages: u32,
    },
}

#[derive(Subcommand, Debug)]
enum SecurityCmd {
    /// Dependabot alerts
    Dependabot {
        /// Repository in the form owner/name
        repo: String,
        #[arg(long)]
        state: Option<String>,
        #[arg(long)]
        severity: Option<String>,
        #[arg(long, default_value_t = 100)]
        per_page: u32,
        #[arg(long, default_value_t = 1)]
        pages: u32,
    },
    /// Code scanning alerts
    CodeScanning {
        /// Repository in the form owner/name
        repo: String,
        #[arg(long)]
        state: Option<String>,
        #[arg(long)]
        severity: Option<String>,
        #[arg(long, default_value_t = 100)]
        per_page: u32,
        #[arg(long, default_value_t = 1)]
        pages: u32,
    },
    /// Secret scanning alerts
    SecretScanning {
        /// Repository in the form owner/name
        repo: String,
        #[arg(long)]
        state: Option<String>,
        #[arg(long = "type")]
        secret_type: Option<String>,
        #[arg(long, default_value_t = 100)]
        per_page: u32,
        #[arg(long, default_value_t = 1)]
        pages: u32,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigCmd {
    /// Initialize a config file in the current directory
    Init {
        /// Format: yaml|toml|json
        #[arg(long, default_value = "yaml")]
        format: String,
    },
    /// Get a config value by key (e.g., github.api_url)
    Get { key: String },
    /// Set a config value by key
    Set {
        key: String,
        value: String,
        /// Optional explicit config path
        #[arg(long)]
        path: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum DocsCmd {
    /// Print command reference as Markdown
    Md,
    /// Update README.md section between AUTO-GENERATED markers
    Readme,
}

fn init_tracing(level: &str) {
    let env_filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("info"));
    #[cfg(feature = "otel")]
    {
        if let Ok(endpoint) = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
            let exporter = opentelemetry_otlp::new_exporter().tonic().with_endpoint(endpoint);
            let tracer = opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_exporter(exporter)
                .with_trace_config(sdktrace::config().with_sampler(sdktrace::Sampler::AlwaysOn))
                .install_batch(opentelemetry::runtime::Tokio)
                .expect("install otel tracer");
            let otel = OpenTelemetryLayer::new(tracer);
            let subscriber = Registry::default().with(env_filter).with(fmt::layer().without_time()).with(otel);
            tracing::subscriber::set_global_default(subscriber).expect("set global subscriber");
            return;
        }
    }
    fmt().with_env_filter(env_filter).without_time().init();
}

fn load_file_config(path: Option<PathBuf>) -> Result<FileConfig> {
    let candidates = if let Some(p) = path {
        vec![p]
    } else {
        let mut v = Vec::new();
        let cwd = std::env::current_dir()?;
        for ext in ["toml", "yaml", "yml", "json"] {
            v.push(cwd.join(format!("gh-otco.{ext}")));
        }
        if let Some(home) = home_dir() {
            for ext in ["toml", "yaml", "yml", "json"] {
                v.push(home.join(format!(".gh-otco.{ext}")));
            }
        }
        v
    };

    for p in candidates {
        if p.exists() {
            let content = fs::read_to_string(&p)
                .with_context(|| format!("reading config file: {}", p.display()))?;
            let cfg: FileConfig = match p.extension().and_then(|s| s.to_str()).unwrap_or("") {
                "toml" => toml::from_str(&content)?,
                "yaml" | "yml" => serde_yaml::from_str(&content)?,
                "json" => serde_json::from_str(&content)?,
                _ => serde_yaml::from_str(&content).or_else(|_| toml::from_str(&content))?,
            };
            return Ok(cfg);
        }
    }
    Ok(FileConfig::default())
}

#[derive(Debug, Clone)]
struct ResolvedConfig {
    api_url: String,
    output: OutputFormat,
    token: Option<String>,
}

fn resolve_config(cli: &Cli, file: &FileConfig) -> ResolvedConfig {
    // File (lowest) → env → CLI (highest)
    let file_api = file.github.api_url.clone();
    let env_api = std::env::var("GITHUB_API_URL").ok();
    let api_url = cli
        .api_url
        .clone()
        .or(env_api)
        .unwrap_or(file_api);

    let file_output = file.output.format.to_lowercase();
    let env_output = std::env::var("OTCO_OUTPUT").ok().unwrap_or(file_output);
    let output = cli.output.unwrap_or_else(|| match env_output.as_str() {
        "json" => OutputFormat::Json,
        "yaml" => OutputFormat::Yaml,
        "csv" => OutputFormat::Csv,
        "psv" => OutputFormat::Psv,
        _ => OutputFormat::Table,
    });

    let token = std::env::var("GITHUB_TOKEN").ok();

    ResolvedConfig { api_url, output, token }
}

fn key_service(host: &str) -> String { format!("gh-otco::{host}") }

fn derive_host_from_url(api_url: &str) -> String {
    url::Url::parse(api_url)
        .ok()
        .and_then(|u| u.host_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "api.github.com".to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(&cli.log_level);

    let file_cfg = load_file_config(cli.config.clone())?;
    let mut cfg = resolve_config(&cli, &file_cfg);

    // Merge token from keyring if not present
    if cfg.token.is_none() {
        let host = derive_host_from_url(&cfg.api_url);
        if let Ok(entry) = Entry::new(&key_service(&host), "default") {
            if let Ok(t) = entry.get_password() { cfg.token = Some(t); }
        }
    }

    match cli.command {
        Commands::Auth { cmd } => match cmd {
            AuthCmd::Login { token, device, host } => {
                let host = host.unwrap_or_else(|| derive_host_from_url(&cfg.api_url));
                if device {
                    println!("OAuth device flow not yet implemented. Use --token for now.");
                    return Ok(());
                }
                let token = match token.or(cfg.token) {
                    Some(t) => t,
                    None => {
                        eprintln!("Enter GitHub PAT (input hidden):");
                        rpassword::prompt_password("PAT> ")?
                    }
                };
                let entry = Entry::new(&key_service(&host), "default")?;
                entry.set_password(&token)?;
                println!("Stored token for host {host}");
            }
            AuthCmd::Logout { host } => {
                let host = host.unwrap_or_else(|| derive_host_from_url(&cfg.api_url));
                match Entry::new(&key_service(&host), "default").and_then(|e| e.delete_password()) {
                    Ok(_) => println!("Removed token for host {host}"),
                    Err(e) => println!("No token removed for {host}: {e}"),
                }
            }
            AuthCmd::Whoami => {
                let client = GitHubClient::new(Some(cfg.api_url.clone()), cfg.token.clone())?;
                match client.current_user().await {
                    Ok(user) => output_any(&user, cfg.output, cli.output_file.as_deref())?,
                    Err(e) => {
                        warn!(error = %e, "failed to fetch user");
                        return Err(e.into());
                    }
                }
            }
        },
        Commands::Meta { cmd } => match cmd {
            MetaCmd::RateLimit => {
                let client = GitHubClient::new(Some(cfg.api_url.clone()), cfg.token.clone())?;
                match client.rate_limit().await {
                    Ok(rl) => output_any(&rl, cfg.output, cli.output_file.as_deref())?,
                    Err(e) => {
                        warn!(error = %e, "failed to fetch rate limit");
                        return Err(e.into());
                    }
                }
            }
        },
        Commands::Org { cmd } => match cmd {
            OrgCmd::Repos { org, r#type, per_page, pages } => {
                let client = GitHubClient::new(Some(cfg.api_url.clone()), cfg.token.clone())?;
                let repos = client
                    .list_org_repos(&org, r#type.as_deref(), per_page, if cli.all { Some(u32::MAX) } else { Some(pages) })
                    .await?;
                output_array_with_projection(&repos, cfg.output, cli.fields.as_deref(), cli.sort.as_deref(), cli.limit, cli.output_file.as_deref())?;
            }
        },
        Commands::Repo { cmd } => match cmd {
            RepoCmd::List { org, r#type, per_page, pages } => {
                let client = GitHubClient::new(Some(cfg.api_url.clone()), cfg.token.clone())?;
                let repos = client
                    .list_org_repos(&org, r#type.as_deref(), per_page, if cli.all { Some(u32::MAX) } else { Some(pages) })
                    .await?;
                output_array_with_projection(&repos, cfg.output, cli.fields.as_deref(), cli.sort.as_deref(), cli.limit, cli.output_file.as_deref())?;
            }
        },
        Commands::Issues { cmd } => match cmd {
            IssuesCmd::List { repo, state, labels, assignee, milestone, since, per_page, pages } => {
                let (owner, name) = split_repo(&repo)?;
                let client = GitHubClient::new(Some(cfg.api_url.clone()), cfg.token.clone())?;
                let issues = client
                    .list_repo_issues(&owner, &name, state.as_deref(), labels.as_deref(), assignee.as_deref(), milestone.as_deref(), since.as_deref(), per_page, if cli.all { Some(u32::MAX) } else { Some(pages) })
                    .await?;
                output_array_with_projection(&issues, cfg.output, cli.fields.as_deref(), cli.sort.as_deref(), cli.limit, cli.output_file.as_deref())?;
            }
        },
        Commands::Prs { cmd } => match cmd {
            PrsCmd::List { repo, state, draft, base, per_page, pages } => {
                let (owner, name) = split_repo(&repo)?;
                let client = GitHubClient::new(Some(cfg.api_url.clone()), cfg.token.clone())?;
                let prs = client
                    .list_repo_pulls(&owner, &name, state.as_deref(), draft, base.as_deref(), per_page, if cli.all { Some(u32::MAX) } else { Some(pages) })
                    .await?;
                output_array_with_projection(&prs, cfg.output, cli.fields.as_deref(), cli.sort.as_deref(), cli.limit, cli.output_file.as_deref())?;
            }
        },
        Commands::Actions { cmd } => match cmd {
            ActionsCmd::Workflows { repo } => {
                let (owner, name) = split_repo(&repo)?;
                let client = GitHubClient::new(Some(cfg.api_url.clone()), cfg.token.clone())?;
                let workflows = client.list_repo_workflows(&owner, &name).await?;
                output_any(&workflows, cfg.output, cli.output_file.as_deref())?;
            }
            ActionsCmd::Runs { repo, branch, status, conclusion, per_page, pages } => {
                let (owner, name) = split_repo(&repo)?;
                let client = GitHubClient::new(Some(cfg.api_url.clone()), cfg.token.clone())?;
                let runs = client
                    .list_repo_workflow_runs(&owner, &name, branch.as_deref(), status.as_deref(), conclusion.as_deref(), per_page, if cli.all { Some(u32::MAX) } else { Some(pages) })
                    .await?;
                output_array_with_projection(&runs, cfg.output, cli.fields.as_deref(), cli.sort.as_deref(), cli.limit, cli.output_file.as_deref())?;
            }
        },
        Commands::Security { cmd } => match cmd {
            SecurityCmd::Dependabot { repo, state, severity, per_page, pages } => {
                let (owner, name) = split_repo(&repo)?;
                let client = GitHubClient::new(Some(cfg.api_url.clone()), cfg.token.clone())?;
                let alerts = client
                    .list_dependabot_alerts(&owner, &name, state.as_deref(), severity.as_deref(), per_page, if cli.all { Some(u32::MAX) } else { Some(pages) })
                    .await?;
                output_array_with_projection(&alerts, cfg.output, cli.fields.as_deref(), cli.sort.as_deref(), cli.limit, cli.output_file.as_deref())?;
            }
            SecurityCmd::CodeScanning { repo, state, severity, per_page, pages } => {
                let (owner, name) = split_repo(&repo)?;
                let client = GitHubClient::new(Some(cfg.api_url.clone()), cfg.token.clone())?;
                let alerts = client
                    .list_codescanning_alerts(&owner, &name, state.as_deref(), severity.as_deref(), per_page, if cli.all { Some(u32::MAX) } else { Some(pages) })
                    .await?;
                output_array_with_projection(&alerts, cfg.output, cli.fields.as_deref(), cli.sort.as_deref(), cli.limit, cli.output_file.as_deref())?;
            }
            SecurityCmd::SecretScanning { repo, state, secret_type, per_page, pages } => {
                let (owner, name) = split_repo(&repo)?;
                let client = GitHubClient::new(Some(cfg.api_url.clone()), cfg.token.clone())?;
                let alerts = client
                    .list_secret_scanning_alerts(&owner, &name, state.as_deref(), secret_type.as_deref(), per_page, if cli.all { Some(u32::MAX) } else { Some(pages) })
                    .await?;
                output_array_with_projection(&alerts, cfg.output, cli.fields.as_deref(), cli.sort.as_deref(), cli.limit, cli.output_file.as_deref())?;
            }
        },
        Commands::Config { cmd } => match cmd {
            ConfigCmd::Init { format } => {
                let (path, fmt) = default_config_path_with_format(Some(format))?;
                if path.exists() {
                    println!("Config already exists at {}", path.display());
                } else {
                    let cfg = FileConfig::default();
                    write_config(&path, &cfg, &fmt)?;
                    println!("Created config at {}", path.display());
                }
            }
            ConfigCmd::Get { key } => {
                let cfg = load_file_config(cli.config.clone())?;
                if let Some(val) = get_config_key(&cfg, &key) {
                    println!("{}", val);
                } else {
                    eprintln!("Key not found: {key}");
                }
            }
            ConfigCmd::Set { key, value, path } => {
                let (path, fmt) = if let Some(p) = path { let f = infer_format(&p); (p, f) } else { default_config_path_with_format(None)? };
                let mut cfg = load_file_config(Some(path.clone())).unwrap_or_default();
                if set_config_key(&mut cfg, &key, &value).is_err() {
                    anyhow::bail!("Unknown or unsupported key: {key}");
                }
                write_config(&path, &cfg, &fmt)?;
                println!("Updated {}", path.display());
            }
        },
        Commands::Docs { cmd } => match cmd {
            DocsCmd::Md => {
                let md = generate_markdown_from_clap();
                println!("{}", md);
            }
            DocsCmd::Readme => {
                let md = generate_markdown_from_clap();
                let readme_path = find_readme().unwrap_or_else(|| PathBuf::from("README.md"));
                let content = fs::read_to_string(&readme_path)?;
                let begin = "<!-- AUTO-GENERATED: COMMANDS BEGIN -->";
                let end = "<!-- AUTO-GENERATED: COMMANDS END -->";
                let new_content = if let (Some(bi), Some(ei)) = (content.find(begin), content.find(end)) {
                    let before = &content[..bi + begin.len()];
                    let after = &content[ei..];
                    format!("{}\n\n{}\n\n{}", before, md, after)
                } else {
                    format!("{}\n\n{}\n\n{}\n{}\n", content, begin, md, end)
                };
                fs::write(&readme_path, new_content)?;
                println!("Updated {}", readme_path.display());
            }
        },
    }

    #[cfg(feature = "otel")]
    {
        // flush traces if enabled
        opentelemetry::global::shutdown_tracer_provider();
    }
    Ok(())
}

#[allow(dead_code)]
fn output_one(map: &BTreeMap<&str, String>, fmt: OutputFormat) -> Result<()> {
    match fmt {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(map)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(map)?);
        }
        OutputFormat::Csv | OutputFormat::Psv => {
            let mut wtr = csv::WriterBuilder::new()
                .delimiter(match fmt { OutputFormat::Csv => b',', _ => b'|' })
                .from_writer(std::io::stdout());
            wtr.write_record(map.keys().cloned())?;
            wtr.write_record(map.values().cloned())?;
            wtr.flush()?;
        }
        OutputFormat::Table => {
            let mut table = Table::new();
            table.load_preset(UTF8_FULL);
            table.set_header(map.keys().cloned().collect::<Vec<_>>());
            table.add_row(map.values().cloned().collect::<Vec<_>>());
            println!("{table}");
        }
    }
    Ok(())
}

fn output_any<T: Serialize>(value: &T, fmt: OutputFormat, out_path: Option<&Path>) -> Result<()> {
    match fmt {
        OutputFormat::Json => {
            let s = serde_json::to_string_pretty(value)?;
            write_out(&s, out_path)?;
        }
        OutputFormat::Yaml => {
            let s = serde_yaml::to_string(value)?;
            write_out(&s, out_path)?;
        }
        OutputFormat::Csv | OutputFormat::Psv | OutputFormat::Table => {
            // Try to render arrays of objects; fallback to JSON
            let v = serde_json::to_value(value)?;
            if let Some(arr) = v.as_array() {
                let rows = normalize_records(arr);
                match fmt {
                    OutputFormat::Table => write_out(&table_to_string(&rows), out_path)?,
                    OutputFormat::Csv | OutputFormat::Psv => write_out(&delimited_to_string(&rows, fmt)?, out_path)?,
                    _ => unreachable!(),
                }
            } else {
                let s = serde_json::to_string_pretty(&v)?;
                write_out(&s, out_path)?;
            }
        }
    }
    Ok(())
}

fn output_array_with_projection(
    arr: &Vec<serde_json::Value>,
    fmt: OutputFormat,
    fields: Option<&str>,
    sort: Option<&str>,
    limit: Option<usize>,
    out_path: Option<&Path>,
) -> Result<()> {
    let mut rows = normalize_records(arr);
    if let Some(fcsv) = fields {
        let want: Vec<String> = fcsv.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        rows = rows
            .into_iter()
            .map(|mut r| {
                r.retain(|k, _| want.iter().any(|w| w == k));
                r
            })
            .collect();
    }
    if let Some(s) = sort {
        let desc = s.starts_with('-');
        let key = s.trim_start_matches('-').to_string();
        rows.sort_by(|a, b| a.get(&key).cmp(&b.get(&key)));
        if desc { rows.reverse(); }
    }
    if let Some(l) = limit { if rows.len() > l { rows.truncate(l); } }
    match fmt {
        OutputFormat::Json => write_out(&serde_json::to_string_pretty(&rows)?, out_path)?,
        OutputFormat::Yaml => write_out(&serde_yaml::to_string(&rows)?, out_path)?,
        OutputFormat::Csv | OutputFormat::Psv => write_out(&delimited_to_string(&rows, fmt)?, out_path)?,
        OutputFormat::Table => write_out(&table_to_string(&rows), out_path)?,
    }
    Ok(())
}

fn normalize_records(arr: &[serde_json::Value]) -> Vec<BTreeMap<String, String>> {
    let mut keys: BTreeMap<String, ()> = BTreeMap::new();
    for item in arr {
        if let Some(obj) = item.as_object() {
            for k in obj.keys() {
                keys.insert(k.clone(), ());
            }
        }
    }
    let header: Vec<String> = keys.into_keys().collect();
    arr.iter()
        .map(|item| {
            let mut row = BTreeMap::new();
            let obj = item.as_object().cloned().unwrap_or_default();
            for k in &header {
                let s = obj.get(k).map(render_value).unwrap_or_default();
                row.insert(k.clone(), s);
            }
            row
        })
        .collect()
}

fn write_delimited(rows: &[BTreeMap<String, String>], fmt: OutputFormat) -> Result<()> {
    let headers: Vec<String> = rows
        .get(0)
        .map(|r| r.keys().cloned().collect())
        .unwrap_or_default();
    let mut wtr = csv::WriterBuilder::new()
        .delimiter(match fmt { OutputFormat::Csv => b',', _ => b'|' })
        .from_writer(std::io::stdout());
    if !headers.is_empty() {
        wtr.write_record(headers.clone())?;
    }
    for row in rows {
        let record: Vec<String> = headers.iter().map(|h| row.get(h).cloned().unwrap_or_default()).collect();
        wtr.write_record(record)?;
    }
    wtr.flush()?;
    Ok(())
}

fn print_table(rows: &[BTreeMap<String, String>]) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    if let Some(first) = rows.first() {
        table.set_header(first.keys().cloned().collect::<Vec<_>>());
    }
    for row in rows {
        table.add_row(row.values().cloned().collect::<Vec<_>>());
    }
    println!("{table}");
}

fn split_repo(s: &str) -> Result<(String, String)> {
    let mut parts = s.splitn(2, '/');
    let owner = parts.next().unwrap_or("");
    let name = parts.next().unwrap_or("");
    if owner.is_empty() || name.is_empty() {
        anyhow::bail!("expected <owner>/<repo>, got '{s}'");
    }
    Ok((owner.to_string(), name.to_string()))
}

fn render_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => String::new(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        _ => v.to_string(),
    }
}

fn write_out(s: &str, out_path: Option<&Path>) -> Result<()> {
    if let Some(p) = out_path { fs::write(p, s)?; } else { println!("{}", s); }
    Ok(())
}

fn delimited_to_string(rows: &[BTreeMap<String, String>], fmt: OutputFormat) -> Result<String> {
    let headers: Vec<String> = rows
        .get(0)
        .map(|r| r.keys().cloned().collect())
        .unwrap_or_default();
    let mut buf: Vec<u8> = Vec::new();
    let mut wtr = csv::WriterBuilder::new()
        .delimiter(match fmt { OutputFormat::Csv => b',', _ => b'|' })
        .from_writer(&mut buf);
    if !headers.is_empty() {
        wtr.write_record(headers.clone())?;
    }
    for row in rows {
        let record: Vec<String> = headers.iter().map(|h| row.get(h).cloned().unwrap_or_default()).collect();
        wtr.write_record(record)?;
    }
    wtr.flush()?;
    drop(wtr);
    Ok(String::from_utf8_lossy(&buf).to_string())
}

fn table_to_string(rows: &[BTreeMap<String, String>]) -> String {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    if let Some(first) = rows.first() {
        table.set_header(first.keys().cloned().collect::<Vec<_>>());
    }
    for row in rows {
        table.add_row(row.values().cloned().collect::<Vec<_>>());
    }
    format!("{}", table)
}

fn find_readme() -> Option<PathBuf> {
    if let Ok(ws) = std::env::var("CARGO_WORKSPACE_ROOT") {
        let p = PathBuf::from(ws).join("README.md");
        if p.exists() { return Some(p); }
    }
    let mut cur = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").ok()?);
    for _ in 0..5 {
        let candidate = cur.join("README.md");
        if candidate.exists() { return Some(candidate); }
        if !cur.pop() { break; }
    }
    None
}

fn infer_format(path: &PathBuf) -> String {
    match path.extension().and_then(|s| s.to_str()).unwrap_or("") {
        "toml" => "toml".into(),
        "json" => "json".into(),
        _ => "yaml".into(),
    }
}

fn default_config_path_with_format(prefer: Option<String>) -> Result<(PathBuf, String)> {
    let cwd = std::env::current_dir()?;
    let fmt = prefer.unwrap_or_else(|| "yaml".to_string());
    let filename = match fmt.as_str() { "toml" => "gh-otco.toml", "json" => "gh-otco.json", _ => "gh-otco.yaml" };
    Ok((cwd.join(filename), fmt))
}

fn write_config(path: &PathBuf, cfg: &FileConfig, fmt: &str) -> Result<()> {
    let content = match fmt {
        "toml" => toml::to_string_pretty(cfg)?,
        "json" => serde_json::to_string_pretty(cfg)?,
        _ => serde_yaml::to_string(cfg)?,
    };
    fs::write(path, content)?;
    Ok(())
}

fn get_config_key(cfg: &FileConfig, key: &str) -> Option<String> {
    match key {
        "github.api_url" => Some(cfg.github.api_url.clone()),
        "output.format" => Some(cfg.output.format.clone()),
        "pagination.per_page" => cfg.pagination.per_page.map(|v| v.to_string()),
        _ => None,
    }
}

fn set_config_key(cfg: &mut FileConfig, key: &str, value: &str) -> Result<()> {
    match key {
        "github.api_url" => cfg.github.api_url = value.to_string(),
        "output.format" => cfg.output.format = value.to_string(),
        "pagination.per_page" => cfg.pagination.per_page = value.parse().ok(),
        _ => anyhow::bail!("unknown key"),
    }
    Ok(())
}

fn generate_markdown_from_clap() -> String {
    let cmd = Cli::command();
    // Ensure derived help strings are built
    let mut out = String::new();
    out.push_str("# Command Reference\n\n");
    out.push_str("Generated from clap definitions.\n\n");
    out.push_str("| Command | Description |\n|---|---|\n");
    // Walk subcommands recursively
    fn walk(cmd: &Command, prefix: &str, out: &mut String) {
        for sc in cmd.get_subcommands() {
            let name = sc.get_name();
            let full = if prefix.is_empty() { format!("otco {}", name) } else { format!("{} {}", prefix, name) };
            let about = sc.get_about().map(|s| s.to_string()).unwrap_or_default();
            out.push_str(&format!("| `{}` | {} |\n", full, about.replace('|', "\\|")));
            walk(sc, &full, out);
        }
    }
    walk(&cmd, "otco", &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_repo_ok_and_err() {
        assert_eq!(split_repo("o/r").unwrap(), ("o".into(), "r".into()));
        assert!(split_repo("oops").is_err());
    }

    #[test]
    fn default_config_paths_and_infer() {
        let (p, fmt) = default_config_path_with_format(Some("toml".into())).unwrap();
        assert!(p.ends_with("gh-otco.toml"));
        assert_eq!(fmt, "toml");
        assert_eq!(infer_format(&PathBuf::from("/tmp/x.json")), "json");
        assert_eq!(infer_format(&PathBuf::from("/tmp/x.txt")), "yaml");
    }

    #[test]
    fn resolve_config_precedence() {
        // Clear envs then set
        for k in ["GITHUB_API_URL", "OTCO_OUTPUT"] { std::env::remove_var(k); }
        let cli = Cli::parse_from(["otco", "--log-level", "warn", "meta", "rate-limit"]);
        let mut file = FileConfig::default();
        file.github.api_url = "https://api.example".into();
        file.output.format = "yaml".into();
        let r = resolve_config(&cli, &file);
        assert_eq!(r.api_url, "https://api.example");
        assert!(matches!(r.output, OutputFormat::Yaml));

        std::env::set_var("GITHUB_API_URL", "https://env.example");
        std::env::set_var("OTCO_OUTPUT", "json");
        let r = resolve_config(&cli, &file);
        assert_eq!(r.api_url, "https://env.example");
        assert!(matches!(r.output, OutputFormat::Json));

        let cli2 = Cli::parse_from(["otco", "--api-url", "https://cli.example", "--output", "yaml", "meta", "rate-limit"]);
        let r = resolve_config(&cli2, &file);
        assert_eq!(r.api_url, "https://cli.example");
        assert!(matches!(r.output, OutputFormat::Yaml));
    }

    #[test]
    fn normalize_records_flattens_headers() {
        let arr = vec![
            serde_json::json!({"a":1, "b":"x"}),
            serde_json::json!({"b":"y", "c":true})
        ];
        let rows = normalize_records(&arr);
        let headers: Vec<_> = rows[0].keys().cloned().collect();
        assert!(headers.contains(&"a".into()));
        assert!(headers.contains(&"b".into()));
        assert!(headers.contains(&"c".into()));
    }

    #[test]
    fn docs_markdown_contains_commands() {
        let md = generate_markdown_from_clap();
        assert!(md.contains("otco auth"));
        assert!(md.contains("otco issues"));
    }
}
