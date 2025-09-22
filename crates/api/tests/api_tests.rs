use gh_otco_api::GitHubClient;
use httpmock::prelude::*;

#[tokio::test]
async fn rate_limit_includes_headers_and_parses() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET)
            .path("/rate_limit")
            .header("user-agent", "gh-otco-cli")
            .header("accept", "application/vnd.github+json")
            .header("authorization", "Bearer testtoken");
        then.status(200)
            .json_body(serde_json::json!({"rate": {}, "resources": {}}));
    });

    let client = GitHubClient::new(Some(server.url("").to_string()), Some("testtoken".into())).unwrap();
    let _ = client.rate_limit().await.unwrap();
    m.assert();
}

#[tokio::test]
async fn current_user_works() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET).path("/user");
        then.status(200).json_body(serde_json::json!({"login":"octo","id":1}));
    });
    let client = GitHubClient::new(Some(server.url("").to_string()), None).unwrap();
    let user = client.current_user().await.unwrap();
    assert_eq!(user.login, "octo");
    assert_eq!(user.id, 1);
    m.assert();
}

#[tokio::test]
async fn org_repos_paginates() {
    let server = MockServer::start();
    let m1 = server.mock(|when, then| {
        when.method(GET)
            .path("/orgs/myorg/repos")
            .query_param("per_page", "2")
            .query_param("page", "1");
        then.status(200).json_body(serde_json::json!([{"name":"a"},{"name":"b"}]));
    });
    let m2 = server.mock(|when, then| {
        when.method(GET)
            .path("/orgs/myorg/repos")
            .query_param("per_page", "2")
            .query_param("page", "2");
        then.status(200).json_body(serde_json::json!([{"name":"c"}]));
    });

    let client = GitHubClient::new(Some(server.url("").to_string()), None).unwrap();
    let repos = client
        .list_org_repos("myorg", None, 2, Some(2))
        .await
        .unwrap();
    let names: Vec<_> = repos
        .into_iter()
        .map(|v| v.get("name").and_then(|x| x.as_str()).unwrap().to_string())
        .collect();
    assert_eq!(names, vec!["a", "b", "c"]);
    m1.assert();
    m2.assert();
}

