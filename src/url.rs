use crate::source::{fetch_url, BoxError, Source};
use serde::Deserialize;

const PROTO_GITHUB: &str = "github://";
const PROTO_GITLAB: &str = "gitlab://";

/// Attempt to resolve a GitHub/GitLab README from a path that may use the
/// `github://user/repo` or `gitlab://user/repo` pseudo-protocols, or a plain
/// https://github.com/... URL. Returns `None` if the path is not a
/// GitHub/GitLab URL.
pub fn readme_url(path: &str) -> Option<Result<Source, BoxError>> {
    if path.starts_with(PROTO_GITHUB) {
        let raw = path.strip_prefix(PROTO_GITHUB).unwrap();
        let url = github_readme_url(raw)?;
        Some(fetch_source_from_url(&url))
    } else if path.starts_with(PROTO_GITLAB) {
        let raw = path.strip_prefix(PROTO_GITLAB).unwrap();
        let url = gitlab_readme_url(raw)?;
        Some(fetch_source_from_url(&url))
    } else if let Some(u) = is_github_url(path) {
        Some(find_github_readme(&u))
    } else if let Some(u) = is_gitlab_url(path) {
        Some(find_gitlab_readme(&u))
    } else {
        None
    }
}

fn github_readme_url(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.len() != 2 { return None; }
    Some(format!("https://github.com/{}/{}", parts[0], parts[1]))
}

fn gitlab_readme_url(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.len() != 2 { return None; }
    Some(format!("https://gitlab.com/{}/{}", parts[0], parts[1]))
}

fn is_github_url(path: &str) -> Option<String> {
    if path.starts_with("https://github.com/") || path.starts_with("http://github.com/") {
        Some(path.to_owned())
    } else {
        None
    }
}

fn is_gitlab_url(path: &str) -> Option<String> {
    if path.starts_with("https://gitlab.com/") || path.starts_with("http://gitlab.com/") {
        Some(path.to_owned())
    } else {
        None
    }
}

fn fetch_source_from_url(url: &str) -> Result<Source, BoxError> {
    // Recursively resolve through the readme_url helper (handles github:// → https://)
    if let Some(result) = readme_url(url) {
        return result;
    }
    fetch_url(url)
}

/// GitHub API response for repository readme endpoint.
#[derive(Deserialize)]
struct GithubReadme {
    download_url: Option<String>,
}

fn find_github_readme(url: &str) -> Result<Source, BoxError> {
    let parsed = ::url::Url::parse(url)?;
    let path = parsed.path().trim_start_matches('/');
    let (owner, rest) = path
        .split_once('/')
        .ok_or_else(|| format!("invalid GitHub URL: {}", url))?;
    // strip any extra path segments — we only need owner/repo
    let repo = rest.split('/').next().unwrap_or(rest);

    let api_url = format!(
        "https://api.{}/repos/{}/{}/readme",
        parsed.host_str().unwrap_or("github.com"),
        owner,
        repo
    );

    let resp = ureq::get(&api_url).call()?;
    if resp.status() != 200 {
        return Err("can't find README in GitHub repository".into());
    }
    let body = resp.into_string()?;
    let readme: GithubReadme = serde_json::from_str(&body)?;
    let download_url = readme
        .download_url
        .ok_or("no download_url in GitHub readme response")?;
    fetch_url(&download_url)
}

/// GitLab API response for project endpoint.
#[derive(Deserialize)]
struct GitlabProject {
    readme_url: Option<String>,
}

fn find_gitlab_readme(url: &str) -> Result<Source, BoxError> {
    let parsed = ::url::Url::parse(url)?;
    let path = parsed.path().trim_start_matches('/');
    let (owner, rest) = path
        .split_once('/')
        .ok_or_else(|| format!("invalid GitLab URL: {}", url))?;
    let repo = rest.split('/').next().unwrap_or(rest);

    let project_path = ::url::form_urlencoded::byte_serialize(
        format!("{}/{}", owner, repo).as_bytes(),
    )
    .collect::<String>();

    let api_url = format!(
        "https://{}/api/v4/projects/{}",
        parsed.host_str().unwrap_or("gitlab.com"),
        project_path
    );

    let resp = ureq::get(&api_url).call()?;
    if resp.status() != 200 {
        return Err("can't find README in GitLab repository".into());
    }
    let body = resp.into_string()?;
    let project: GitlabProject = serde_json::from_str(&body)?;
    let readme_url = project
        .readme_url
        .ok_or("no readme_url in GitLab project response")?;
    let raw_url = readme_url.replace("/blob/", "/raw/");
    fetch_url(&raw_url)
}
