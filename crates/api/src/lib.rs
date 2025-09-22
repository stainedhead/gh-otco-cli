use reqwest::header::{HeaderMap, HeaderValue, HeaderName, ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;
use std::time::Duration;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("url parse error: {0}")]
    Url(#[from] url::ParseError),
}

#[derive(Clone)]
pub struct GitHubClient {
    base_url: Url,
    client: reqwest::Client,
    token: Option<String>,
}

impl GitHubClient {
    pub fn new(base_url: Option<String>, token: Option<String>) -> Result<Self, ApiError> {
        let base = base_url
            .unwrap_or_else(|| "https://api.github.com".to_string());
        let base_url = Url::parse(&base)?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
        Ok(Self { base_url, client, token })
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("gh-otco-cli"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/vnd.github+json"));
        headers.insert(
            HeaderName::from_static("x-github-api-version"),
            HeaderValue::from_static("2022-11-28"),
        );
        if let Some(t) = &self.token {
            let value = format!("Bearer {}", t);
            if let Ok(val) = HeaderValue::from_str(&value) {
                headers.insert(AUTHORIZATION, val);
            }
        }
        headers
    }

    fn url(&self, path: &str) -> Result<Url, ApiError> {
        Ok(self.base_url.join(path)?)
    }

    pub async fn rate_limit(&self) -> Result<RateLimit, ApiError> {
        let url = self.url("/rate_limit")?;
        let res = self
            .client
            .get(url)
            .headers(self.headers())
            .send()
            .await?
            .error_for_status()?;
        Ok(res.json::<RateLimit>().await?)
    }

    pub async fn current_user(&self) -> Result<User, ApiError> {
        let url = self.url("/user")?;
        let res = self
            .client
            .get(url)
            .headers(self.headers())
            .send()
            .await?
            .error_for_status()?;
        Ok(res.json::<User>().await?)
    }

    async fn get_json(&self, path: &str, params: &[(&str, String)]) -> Result<serde_json::Value, ApiError> {
        let url = self.url(path)?;
        let res = self
            .client
            .get(url)
            .headers(self.headers())
            .query(&params)
            .send()
            .await?
            .error_for_status()?;
        Ok(res.json::<serde_json::Value>().await?)
    }

    async fn get_all_pages_array(
        &self,
        path: &str,
        params: Vec<(&str, String)>,
        per_page: u32,
        max_pages: Option<u32>,
    ) -> Result<Vec<serde_json::Value>, ApiError> {
        let mut page = 1u32;
        let mut out = Vec::new();
        let max_pages = max_pages.unwrap_or(10); // guard to avoid accidental huge fetches
        loop {
            let mut q = params.clone();
            q.push(("per_page", per_page.to_string()));
            q.push(("page", page.to_string()));
            let v = self.get_json(path, &q).await?;
            match v {
                serde_json::Value::Array(mut arr) => {
                    let len = arr.len();
                    out.append(&mut arr);
                    if len == 0 || page >= max_pages { break; }
                }
                _ => break,
            }
            page += 1;
        }
        Ok(out)
    }

    pub async fn list_org_repos(
        &self,
        org: &str,
        kind: Option<&str>, // all, public, private, forks, sources, member
        per_page: u32,
        max_pages: Option<u32>,
    ) -> Result<Vec<serde_json::Value>, ApiError> {
        let mut params = Vec::new();
        if let Some(k) = kind { params.push(("type", k.to_string())); }
        let path = format!("/orgs/{org}/repos");
        self.get_all_pages_array(&path, params, per_page, max_pages).await
    }

    pub async fn list_repo_issues(
        &self,
        owner: &str,
        repo: &str,
        state: Option<&str>, // open, closed, all
        labels: Option<&str>,
        assignee: Option<&str>,
        milestone: Option<&str>,
        since: Option<&str>, // ISO 8601
        per_page: u32,
        max_pages: Option<u32>,
    ) -> Result<Vec<serde_json::Value>, ApiError> {
        let mut params = Vec::new();
        if let Some(s) = state { params.push(("state", s.to_string())); }
        if let Some(l) = labels { params.push(("labels", l.to_string())); }
        if let Some(a) = assignee { params.push(("assignee", a.to_string())); }
        if let Some(m) = milestone { params.push(("milestone", m.to_string())); }
        if let Some(since) = since { params.push(("since", since.to_string())); }
        let path = format!("/repos/{owner}/{repo}/issues");
        self.get_all_pages_array(&path, params, per_page, max_pages).await
    }

    pub async fn list_repo_pulls(
        &self,
        owner: &str,
        repo: &str,
        state: Option<&str>, // open, closed, all
        draft: Option<bool>,
        base: Option<&str>,
        per_page: u32,
        max_pages: Option<u32>,
    ) -> Result<Vec<serde_json::Value>, ApiError> {
        let mut params = Vec::new();
        if let Some(s) = state { params.push(("state", s.to_string())); }
        if let Some(d) = draft { params.push(("draft", d.to_string())); }
        if let Some(b) = base { params.push(("base", b.to_string())); }
        let path = format!("/repos/{owner}/{repo}/pulls");
        self.get_all_pages_array(&path, params, per_page, max_pages).await
    }

    // Actions: list workflows in a repo
    pub async fn list_repo_workflows(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<serde_json::Value, ApiError> {
        let path = format!("/repos/{owner}/{repo}/actions/workflows");
        self.get_json(&path, &[]).await
    }

    // Actions: list workflow runs in a repo with filters
    pub async fn list_repo_workflow_runs(
        &self,
        owner: &str,
        repo: &str,
        branch: Option<&str>,
        status: Option<&str>, // queued, in_progress, completed
        conclusion: Option<&str>, // success, failure, etc.
        per_page: u32,
        max_pages: Option<u32>,
    ) -> Result<Vec<serde_json::Value>, ApiError> {
        let mut params = Vec::new();
        if let Some(b) = branch { params.push(("branch", b.to_string())); }
        if let Some(s) = status { params.push(("status", s.to_string())); }
        if let Some(c) = conclusion { params.push(("conclusion", c.to_string())); }
        let path = format!("/repos/{owner}/{repo}/actions/runs");
        self.get_all_pages_array(&path, params, per_page, max_pages).await
    }

    // Security: Dependabot alerts (repo-level)
    pub async fn list_dependabot_alerts(
        &self,
        owner: &str,
        repo: &str,
        state: Option<&str>,   // open, dismissed, fixed
        severity: Option<&str>, // low, medium, high, critical
        per_page: u32,
        max_pages: Option<u32>,
    ) -> Result<Vec<serde_json::Value>, ApiError> {
        let mut params = Vec::new();
        if let Some(s) = state { params.push(("state", s.to_string())); }
        if let Some(sv) = severity { params.push(("severity", sv.to_string())); }
        let path = format!("/repos/{owner}/{repo}/dependabot/alerts");
        self.get_all_pages_array(&path, params, per_page, max_pages).await
    }

    // Security: Code scanning alerts (repo-level)
    pub async fn list_codescanning_alerts(
        &self,
        owner: &str,
        repo: &str,
        state: Option<&str>, // open, dismissed, fixed
        severity: Option<&str>, // error, warning, note
        per_page: u32,
        max_pages: Option<u32>,
    ) -> Result<Vec<serde_json::Value>, ApiError> {
        let mut params = Vec::new();
        if let Some(s) = state { params.push(("state", s.to_string())); }
        if let Some(sv) = severity { params.push(("severity", sv.to_string())); }
        let path = format!("/repos/{owner}/{repo}/code-scanning/alerts");
        self.get_all_pages_array(&path, params, per_page, max_pages).await
    }

    // Security: Secret scanning alerts (repo-level)
    pub async fn list_secret_scanning_alerts(
        &self,
        owner: &str,
        repo: &str,
        state: Option<&str>, // open, resolved
        secret_type: Option<&str>,
        per_page: u32,
        max_pages: Option<u32>,
    ) -> Result<Vec<serde_json::Value>, ApiError> {
        let mut params = Vec::new();
        if let Some(s) = state { params.push(("state", s.to_string())); }
        if let Some(t) = secret_type { params.push(("secret_type", t.to_string())); }
        let path = format!("/repos/{owner}/{repo}/secret-scanning/alerts");
        self.get_all_pages_array(&path, params, per_page, max_pages).await
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RateLimit {
    pub rate: serde_json::Value,
    pub resources: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub login: String,
    pub id: u64,
}
