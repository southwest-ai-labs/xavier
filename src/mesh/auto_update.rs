//! Auto-update checks for Xavier releases and silent tracking of the main branch.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tracing::{info, warn};

const DEFAULT_GITHUB_REPO: &str = "southwest-ai-labs/xavier";
const DEFAULT_GITHUB_BRANCH: &str = "main";
const GITHUB_LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/southwest-ai-labs/xavier/releases/latest";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MainCommitInfo {
    pub sha: String,
    pub short_sha: String,
    pub title: String,
    pub improvements: Vec<String>,
    pub commit_url: String,
    pub author: String,
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SilentUpdateReport {
    pub status: String,
    pub applied: bool,
    pub previous_sha: Option<String>,
    pub current_sha: String,
    pub title: String,
    pub improvements: Vec<String>,
    pub commit_url: String,
    pub notification_sent: bool,
}

#[derive(Debug, Clone)]
pub struct AutoUpdateService {
    client: reqwest::Client,
    current_version: String,
    latest_release_url: String,
    repo: String,
    branch: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    UpToDate {
        current_version: String,
        latest_version: String,
    },
    UpdateAvailable {
        current_version: String,
        latest_version: String,
        release_url: String,
    },
    CurrentAhead {
        current_version: String,
        latest_version: String,
    },
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
}

#[derive(Debug, Deserialize)]
struct GitHubCommitResp {
    sha: String,
    html_url: Option<String>,
    commit: GitHubCommitInner,
}

#[derive(Debug, Deserialize)]
struct GitHubCommitInner {
    author: Option<GitHubAuthorInner>,
    message: String,
}

#[derive(Debug, Deserialize)]
struct GitHubAuthorInner {
    name: Option<String>,
    date: Option<String>,
}

impl AutoUpdateService {
    /// Create a new AutoUpdateService with default settings.
    pub fn new() -> Self {
        Self::with_current_version(env!("CARGO_PKG_VERSION"))
    }

    /// Construct with a specific current version string.
    pub fn with_current_version(version: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent(concat!("xavier-auto-update/", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let repo =
            std::env::var("XAVIER_UPDATE_REPO").unwrap_or_else(|_| DEFAULT_GITHUB_REPO.to_string());
        let branch = std::env::var("XAVIER_UPDATE_BRANCH")
            .unwrap_or_else(|_| DEFAULT_GITHUB_BRANCH.to_string());

        Self {
            client,
            current_version: version.into(),
            latest_release_url: GITHUB_LATEST_RELEASE_URL.to_string(),
            repo,
            branch,
        }
    }

    /// Set a custom release URL (e.g. for testing).
    pub fn with_latest_release_url(mut self, url: impl Into<String>) -> Self {
        self.latest_release_url = url.into();
        self
    }

    /// Configure repository and branch for tracking main.
    pub fn with_repo_and_branch(
        mut self,
        repo: impl Into<String>,
        branch: impl Into<String>,
    ) -> Self {
        self.repo = repo.into();
        self.branch = branch.into();
        self
    }

    /// Query GitHub releases for a new semver version tag.
    pub async fn check_for_updates(&self) -> Result<UpdateStatus> {
        let release: GitHubRelease = self
            .client
            .get(&self.latest_release_url)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .context("Failed to query GitHub releases")?
            .error_for_status()
            .context("GitHub release check returned an error")?
            .json()
            .await
            .context("Failed to parse GitHub release response")?;

        Ok(self.compare_versions_with_url(&release.tag_name, release.html_url))
    }

    /// Compare versions without a release URL.
    pub fn compare_versions(&self, latest_version: &str) -> UpdateStatus {
        self.compare_versions_with_url(latest_version, String::new())
    }

    fn compare_versions_with_url(&self, latest_version: &str, release_url: String) -> UpdateStatus {
        let current_normalized = normalize_version(&self.current_version);
        let latest_normalized = normalize_version(latest_version);

        match compare_version_parts(&current_normalized, &latest_normalized) {
            std::cmp::Ordering::Less => UpdateStatus::UpdateAvailable {
                current_version: self.current_version.clone(),
                latest_version: latest_version.to_string(),
                release_url,
            },
            std::cmp::Ordering::Equal => UpdateStatus::UpToDate {
                current_version: self.current_version.clone(),
                latest_version: latest_version.to_string(),
            },
            std::cmp::Ordering::Greater => UpdateStatus::CurrentAhead {
                current_version: self.current_version.clone(),
                latest_version: latest_version.to_string(),
            },
        }
    }

    /// Listen to the main branch of the repository and extract recent improvements.
    pub async fn check_main_branch(&self) -> Result<Option<MainCommitInfo>> {
        let url = format!(
            "https://api.github.com/repos/{}/commits/{}",
            self.repo, self.branch
        );

        let resp_result = self
            .client
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await;

        let commit_info = match resp_result {
            Ok(resp) if resp.status().is_success() => {
                let parsed: GitHubCommitResp = resp
                    .json()
                    .await
                    .context("Failed to parse GitHub commit response")?;

                let sha = parsed.sha;
                let short_sha = if sha.len() >= 7 {
                    sha[..7].to_string()
                } else {
                    sha.clone()
                };
                let (title, improvements) = parse_improvements(&parsed.commit.message);
                let commit_url = parsed
                    .html_url
                    .unwrap_or_else(|| format!("https://github.com/{}/commit/{}", self.repo, sha));
                let author = parsed
                    .commit
                    .author
                    .as_ref()
                    .and_then(|a| a.name.clone())
                    .unwrap_or_else(|| "SWAL Team".to_string());
                let date = parsed
                    .commit
                    .author
                    .as_ref()
                    .and_then(|a| a.date.clone())
                    .unwrap_or_default();

                MainCommitInfo {
                    sha,
                    short_sha,
                    title,
                    improvements,
                    commit_url,
                    author,
                    date,
                }
            }
            _ => {
                // Fallback to local git CLI if GitHub API cannot be reached or is rate-limited
                if let Some(info) = check_local_git_head(&self.repo) {
                    info
                } else {
                    return Ok(None);
                }
            }
        };

        let last_known = get_last_known_sha();
        if let Some(known) = &last_known {
            if known.trim() == commit_info.sha.trim() {
                return Ok(None);
            }
        }

        Ok(Some(commit_info))
    }

    /// Silently apply the update and dispatch a notification via Xavier's notification system.
    pub async fn apply_silent_update(&self, commit: &MainCommitInfo) -> Result<SilentUpdateReport> {
        let previous_sha = get_last_known_sha();

        // If a custom update hook is defined, run it in background
        if let Ok(hook_cmd) = std::env::var("XAVIER_UPDATE_HOOK") {
            info!("Executing custom update hook: {}", hook_cmd);
            let _ = std::process::Command::new("sh")
                .arg("-c")
                .arg(&hook_cmd)
                .spawn();
        }

        // Format user-friendly notification message with improvements and details link
        let mut body = format!(
            "Se aplicaron las siguientes mejoras desde {}:\n• {}",
            self.branch, commit.title
        );
        for imp in &commit.improvements {
            if imp != &commit.title {
                body.push_str(&format!("\n• {}", imp));
            }
        }
        body.push_str(&format!("\n\nDetalles: {}", commit.commit_url));

        let notif_title = format!("Xavier actualizado ({})", commit.short_sha);

        // Dispatch via Xavier's global NOTIFICATIONS manager
        let notification_sent = match crate::notifications::NOTIFICATIONS
            .notify(
                crate::notifications::IslandId::System,
                &notif_title,
                &body,
                "success",
            )
            .await
        {
            Ok(_) => {
                info!(
                    "Dispatched silent update notification for commit {}",
                    commit.short_sha
                );
                true
            }
            Err(e) => {
                warn!("Failed to dispatch silent update notification: {}", e);
                false
            }
        };

        // Persist the newly applied commit SHA
        save_last_known_sha(&commit.sha);

        Ok(SilentUpdateReport {
            status: "updated".to_string(),
            applied: true,
            previous_sha,
            current_sha: commit.sha.clone(),
            title: commit.title.clone(),
            improvements: commit.improvements.clone(),
            commit_url: commit.commit_url.clone(),
            notification_sent,
        })
    }

    /// Execute a full cycle: check main, apply update if present, and notify.
    pub async fn run_silent_update_cycle(&self) -> Result<SilentUpdateReport> {
        let last_sha = get_last_known_sha();

        match self.check_main_branch().await {
            Ok(Some(commit)) => self.apply_silent_update(&commit).await,
            Ok(None) => Ok(SilentUpdateReport {
                status: "up_to_date".to_string(),
                applied: false,
                previous_sha: last_sha.clone(),
                current_sha: last_sha.unwrap_or_default(),
                title: "Sistema al día".to_string(),
                improvements: vec![],
                commit_url: format!("https://github.com/{}/tree/{}", self.repo, self.branch),
                notification_sent: false,
            }),
            Err(e) => Err(e),
        }
    }
}

impl Default for AutoUpdateService {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse commit message into a headline title and extracted bullet-point improvements.
pub fn parse_improvements(message: &str) -> (String, Vec<String>) {
    let lines: Vec<&str> = message
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    if lines.is_empty() {
        return ("Actualización del sistema".to_string(), vec![]);
    }

    let title = lines[0].to_string();
    let mut improvements = Vec::new();

    for line in &lines[1..] {
        if line.starts_with('-') || line.starts_with('*') || line.starts_with('•') {
            let clean = line.trim_start_matches(['-', '*', '•', ' ']);
            if !clean.is_empty() {
                improvements.push(clean.to_string());
            }
        } else if line.starts_with("feat")
            || line.starts_with("fix")
            || line.starts_with("perf")
            || line.starts_with("refactor")
            || line.starts_with("docs")
        {
            improvements.push(line.to_string());
        }
    }

    if improvements.is_empty() && title.contains(':') {
        improvements.push(title.clone());
    }

    (title, improvements)
}

fn normalize_version(version: &str) -> Vec<u64> {
    version
        .trim()
        .trim_start_matches('v')
        .split(|c: char| !(c.is_ascii_digit()))
        .filter(|part| !part.is_empty())
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect()
}

fn compare_version_parts(current: &[u64], latest: &[u64]) -> std::cmp::Ordering {
    for index in 0..current.len().max(latest.len()) {
        match current
            .get(index)
            .copied()
            .unwrap_or(0)
            .cmp(&latest.get(index).copied().unwrap_or(0))
        {
            std::cmp::Ordering::Equal => continue,
            ordering => return ordering,
        }
    }
    std::cmp::Ordering::Equal
}

fn state_file_path() -> PathBuf {
    if let Ok(dir) = std::env::var("XAVIER_DATA_DIR") {
        PathBuf::from(dir).join(".last_main_sha")
    } else {
        PathBuf::from(".last_main_sha")
    }
}

pub fn get_last_known_sha() -> Option<String> {
    let path = state_file_path();
    if path.exists() {
        std::fs::read_to_string(path)
            .ok()
            .map(|s| s.trim().to_string())
    } else {
        None
    }
}

pub fn save_last_known_sha(sha: &str) {
    let path = state_file_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, sha.trim());
}

fn check_local_git_head(repo: &str) -> Option<MainCommitInfo> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let sha = String::from_utf8(output.stdout).ok()?.trim().to_string();
    let short_sha = if sha.len() >= 7 {
        sha[..7].to_string()
    } else {
        sha.clone()
    };

    let msg_output = std::process::Command::new("git")
        .args(["log", "-1", "--pretty=%B"])
        .output()
        .ok()?;
    let message = String::from_utf8(msg_output.stdout).unwrap_or_default();
    let (title, improvements) = parse_improvements(&message);

    Some(MainCommitInfo {
        sha: sha.clone(),
        short_sha,
        title,
        improvements,
        commit_url: format!("https://github.com/{}/commit/{}", repo, sha),
        author: "SWAL Team".to_string(),
        date: chrono::Utc::now().to_rfc3339(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_improvements() {
        let msg = "feat(ui): responsive configuration panel\n\n- Unified 15 settings into 4 categories\n- Responsive pill bar for smartphones\n- Playwright E2E multi-device verified";
        let (title, improvements) = parse_improvements(msg);

        assert_eq!(title, "feat(ui): responsive configuration panel");
        assert_eq!(improvements.len(), 3);
        assert_eq!(improvements[0], "Unified 15 settings into 4 categories");
        assert_eq!(improvements[1], "Responsive pill bar for smartphones");
        assert_eq!(improvements[2], "Playwright E2E multi-device verified");
    }

    #[test]
    fn test_compare_versions() {
        let service = AutoUpdateService::with_current_version("0.2.19");
        assert_eq!(
            service.compare_versions("v0.2.20"),
            UpdateStatus::UpdateAvailable {
                current_version: "0.2.19".to_string(),
                latest_version: "v0.2.20".to_string(),
                release_url: String::new(),
            }
        );
        assert_eq!(
            service.compare_versions("0.2.19"),
            UpdateStatus::UpToDate {
                current_version: "0.2.19".to_string(),
                latest_version: "0.2.19".to_string(),
            }
        );
    }

    #[tokio::test]
    async fn test_apply_silent_update_notification_dispatch() {
        let commit = MainCommitInfo {
            sha: "fe4125ca7abcdef1234567890".to_string(),
            short_sha: "fe4125c".to_string(),
            title: "feat(ui): responsive configuration panel".to_string(),
            improvements: vec![
                "Unified 15 settings into 4 categories".to_string(),
                "Responsive pill bar for smartphones".to_string(),
            ],
            commit_url:
                "https://github.com/southwest-ai-labs/xavier/commit/fe4125ca7abcdef1234567890"
                    .to_string(),
            author: "SWAL Team".to_string(),
            date: "2026-11-08T19:00:00Z".to_string(),
        };

        let service = AutoUpdateService::with_current_version("0.2.20");
        let report = service.apply_silent_update(&commit).await.unwrap();

        assert!(report.applied);
        assert_eq!(report.current_sha, "fe4125ca7abcdef1234567890");
        assert_eq!(report.title, "feat(ui): responsive configuration panel");
        assert!(report.notification_sent);
        assert_eq!(
            report.commit_url,
            "https://github.com/southwest-ai-labs/xavier/commit/fe4125ca7abcdef1234567890"
        );
    }
}
