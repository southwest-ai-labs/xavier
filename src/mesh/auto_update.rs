//! Auto-update checks for Xavier releases and silent tracking of the main branch.
//!
//! Includes Blue-Green binary slots (A/B), pre-flight canary sandbox validation,
//! ETXTBSY protection, and autonomous Rollback Sentinel with a 30s Health Gate.

pub mod sentinel;
pub mod slots;

pub use sentinel::{
    HealthGateDecision, RollbackSentinel, SentinelConfig, SentinelState, SentinelStatus,
};
pub use slots::{BinarySlot, CanaryReport, SlotConfig, SlotManager, SwapMode};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
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
    slot_manager: Option<SlotManager>,
    sentinel: Option<RollbackSentinel>,
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
            slot_manager: None,
            sentinel: None,
        }
    }

    /// Attach SlotManager and RollbackSentinel for Blue-Green atomic swap and health gating.
    pub fn with_resilience(
        mut self,
        slot_manager: SlotManager,
        sentinel: RollbackSentinel,
    ) -> Self {
        self.slot_manager = Some(slot_manager);
        self.sentinel = Some(sentinel);
        self
    }

    pub fn slot_manager(&self) -> Option<&SlotManager> {
        self.slot_manager.as_ref()
    }

    pub fn sentinel(&self) -> Option<&RollbackSentinel> {
        self.sentinel.as_ref()
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

        // If resilient SlotManager and Sentinel are attached, perform staged canary check and atomic swap
        if let (Some(slot_mgr), Some(sentinel)) = (&self.slot_manager, &self.sentinel) {
            if let Ok(cand_path_str) = std::env::var("XAVIER_UPDATE_CANDIDATE_PATH") {
                let cand_path = PathBuf::from(cand_path_str);
                if cand_path.exists() {
                    info!(
                        "AutoUpdateService: Verifying pre-flight canary in sandbox for {:?}",
                        cand_path
                    );
                    let canary = slot_mgr.verify_binary_preflight(
                        &cand_path,
                        &["--version"],
                        Duration::from_secs(5),
                    )?;
                    info!(
                        "AutoUpdateService: Canary pre-flight PASSED in {}ms",
                        canary.duration_ms
                    );

                    let cand_slot = slot_mgr.candidate_slot()?;
                    let active_slot = slot_mgr.detect_active_slot()?;

                    // Arm Sentinel in probation
                    sentinel.start_probation(
                        &commit.sha,
                        previous_sha.as_deref().unwrap_or("unknown"),
                        active_slot.as_str(),
                        cand_slot.as_str(),
                    )?;

                    // Atomic swap to candidate slot
                    slot_mgr.atomic_swap(&cand_path, cand_slot)?;
                    info!(
                        "AutoUpdateService: Atomic swap completed into slot {:?}",
                        cand_slot
                    );
                }
            }
        }

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

    /// Executes an immediate startup check without waiting for the periodic cycle.
    ///
    /// 1. Inspects Sentinel state for any pending probation (e.g. node restarted into candidate binary).
    ///    Runs the 30-second Health Gate immediately. If healthy -> commits; if unhealthy -> rolls back autonomously.
    /// 2. Verifies Sentinel quarantine: if active, suppresses update checks during cooldown.
    /// 3. If clear, immediately queries main branch / releases for pending updates.
    pub async fn run_startup_check(&self) -> Result<SilentUpdateReport> {
        // 1. Check Sentinel quarantine
        if let Some(sentinel) = &self.sentinel {
            if sentinel.is_quarantined() {
                warn!("AutoUpdateService: Update check skipped due to active Sentinel quarantine cooldown");
                let last_sha = get_last_known_sha();
                return Ok(SilentUpdateReport {
                    status: "quarantined".to_string(),
                    applied: false,
                    previous_sha: last_sha.clone(),
                    current_sha: last_sha.unwrap_or_default(),
                    title: "Actualizaciones suspendidas por cuarentena de fallos".to_string(),
                    improvements: vec![],
                    commit_url: String::new(),
                    notification_sent: false,
                });
            }

            // 2. Check if a newly restarted binary is in probation (30s health gate)
            let state = sentinel.load_state();
            if state.status == SentinelStatus::InProbation {
                info!(
                    "AutoUpdateService: Startup check detected candidate {} in probation. Executing Health Gate...",
                    state.candidate_sha
                );
                if let Some(slot_mgr) = &self.slot_manager {
                    let port = std::env::var("PORT").unwrap_or_else(|_| "8006".to_string());
                    let health_url = format!("http://127.0.0.1:{}/health", port);
                    let client = self.client.clone();

                    let decision = sentinel
                        .supervise_health_gate(slot_mgr, || {
                            let url = health_url.clone();
                            let cl = client.clone();
                            async move {
                                match cl.get(&url).send().await {
                                    Ok(resp) => Ok(resp.status().is_success()),
                                    Err(e) => Err(anyhow!("Health probe failed: {}", e)),
                                }
                            }
                        })
                        .await?;

                    match decision {
                        HealthGateDecision::Committed { sha } => {
                            info!("AutoUpdateService: Candidate {} committed at startup!", sha);
                        }
                        HealthGateDecision::RolledBack {
                            sha,
                            target_slot,
                            reason,
                        } => {
                            warn!(
                                "AutoUpdateService: Candidate {} rolled back to {}! Reason: {}",
                                sha, target_slot, reason
                            );
                        }
                        _ => {}
                    }
                }
            }
        }

        // 3. Immediately run silent update cycle
        self.run_silent_update_cycle().await
    }

    /// Execute a full cycle: check main, apply update if present, and notify.
    pub async fn run_silent_update_cycle(&self) -> Result<SilentUpdateReport> {
        // If Sentinel is in active quarantine, abort cycle early
        if let Some(sentinel) = &self.sentinel {
            if sentinel.is_quarantined() {
                warn!("AutoUpdateService: Silent update cycle suppressed (quarantine active)");
                let last_sha = get_last_known_sha();
                return Ok(SilentUpdateReport {
                    status: "quarantined".to_string(),
                    applied: false,
                    previous_sha: last_sha.clone(),
                    current_sha: last_sha.unwrap_or_default(),
                    title: "Actualizaciones en pausa por fallas previas (quarantine activo)"
                        .to_string(),
                    improvements: vec![],
                    commit_url: format!("https://github.com/{}/tree/{}", self.repo, self.branch),
                    notification_sent: false,
                });
            }
        }

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
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

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

    #[test]
    fn test_slots_blue_green_swap_etxtbsy_protection() {
        let temp_dir = tempfile::tempdir().unwrap();
        let slot_mgr = SlotManager::new(temp_dir.path(), "xavier").unwrap();

        // 1. Initial state: populate slot_a with version 1
        let slot_a_bin = slot_mgr.slot_binary_path(BinarySlot::SlotA);
        fs::write(&slot_a_bin, b"#!/bin/sh\necho 'xavier v0.2.20'\n").unwrap();
        #[cfg(unix)]
        fs::set_permissions(&slot_a_bin, fs::Permissions::from_mode(0o755)).unwrap();

        // Symlink active pointer to slot_a
        #[cfg(unix)]
        std::os::unix::fs::symlink(&slot_a_bin, slot_mgr.active_binary_path()).unwrap();

        assert_eq!(slot_mgr.detect_active_slot().unwrap(), BinarySlot::SlotA);
        assert_eq!(slot_mgr.candidate_slot().unwrap(), BinarySlot::SlotB);

        // 2. Simulate running process holding open file descriptor to active binary (triggers ETXTBSY on write)
        let _running_file_handle = fs::File::open(slot_mgr.active_binary_path()).unwrap();

        // 3. Stage candidate binary for slot_b with sha256 checksum
        let cand_bytes = b"#!/bin/sh\necho 'xavier v0.2.21'\n";
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(cand_bytes);
        let expected_sha: String = hasher
            .finalize()
            .into_iter()
            .map(|b| format!("{:02x}", b))
            .collect();

        let staged = slot_mgr
            .stage_binary(cand_bytes, Some(&expected_sha))
            .unwrap();
        assert!(staged.exists());

        // 4. Pre-flight canary check
        let canary = slot_mgr
            .verify_binary_preflight(&staged, &[], Duration::from_secs(3))
            .unwrap();
        assert!(canary.success);
        assert!(canary.stdout.contains("xavier v0.2.21"));

        // 5. Execute atomic swap into slot_b
        // Must succeed without ETXTBSY despite running file handle being held open!
        let swapped_path = slot_mgr.atomic_swap(&staged, BinarySlot::SlotB).unwrap();
        assert_eq!(swapped_path, slot_mgr.slot_binary_path(BinarySlot::SlotB));

        // 6. Verify active pointer now targets slot_b
        assert_eq!(slot_mgr.detect_active_slot().unwrap(), BinarySlot::SlotB);
        assert_eq!(slot_mgr.candidate_slot().unwrap(), BinarySlot::SlotA);

        // Verify backup symlink was generated
        assert!(slot_mgr.backup_binary_path().exists());
    }

    #[test]
    fn test_canary_preflight_rejection_of_corrupt_binary() {
        let temp_dir = tempfile::tempdir().unwrap();
        let slot_mgr = SlotManager::new(temp_dir.path(), "xavier").unwrap();

        // Stage corrupt binary that crashes on boot (exit 42)
        let corrupt_bytes = b"#!/bin/sh\necho 'Fatal: segmentation fault' >&2\nexit 42\n";
        let staged = slot_mgr.stage_binary(corrupt_bytes, None).unwrap();

        // Canary pre-flight must reject it
        let result = slot_mgr.verify_binary_preflight(&staged, &[], Duration::from_secs(3));
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Canary pre-flight check failed"));
        assert!(err_msg.contains("code Some(42)"));
    }

    #[tokio::test]
    async fn test_sentinel_health_gate_success_commits_update() {
        let temp_dir = tempfile::tempdir().unwrap();
        let slot_mgr = SlotManager::new(temp_dir.path(), "xavier").unwrap();

        let sentinel_config = SentinelConfig {
            state_file: temp_dir.path().join("sentinel.json"),
            health_gate_duration: Duration::from_secs(5),
            poll_interval: Duration::from_millis(50),
            max_consecutive_failures: 3,
            quarantine_duration: Duration::from_secs(3600),
        };
        let sentinel = RollbackSentinel::with_config(sentinel_config);

        // Arm probation
        sentinel
            .start_probation("commit_sha_new", "commit_sha_old", "slot_a", "slot_b")
            .unwrap();

        let state = sentinel.load_state();
        assert_eq!(state.status, SentinelStatus::InProbation);

        // Simulate health probe that returns HTTP 200 OK
        let decision = sentinel
            .supervise_health_gate(&slot_mgr, || async { Ok(true) })
            .await
            .unwrap();

        assert_eq!(
            decision,
            HealthGateDecision::Committed {
                sha: "commit_sha_new".to_string()
            }
        );

        let final_state = sentinel.load_state();
        assert_eq!(final_state.status, SentinelStatus::Committed);
        assert_eq!(final_state.consecutive_failures, 0);
    }

    #[tokio::test]
    async fn test_sentinel_health_gate_autonomous_rollback_on_failure() {
        let temp_dir = tempfile::tempdir().unwrap();
        let slot_mgr = SlotManager::new(temp_dir.path(), "xavier").unwrap();

        // Setup slot_a as active and slot_b as candidate
        let slot_a_bin = slot_mgr.slot_binary_path(BinarySlot::SlotA);
        fs::write(&slot_a_bin, b"#!/bin/sh\necho 'xavier v0.2.20'\n").unwrap();
        #[cfg(unix)]
        fs::set_permissions(&slot_a_bin, fs::Permissions::from_mode(0o755)).unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&slot_a_bin, slot_mgr.active_binary_path()).unwrap();

        // Now swap into slot_b
        let slot_b_bytes = b"#!/bin/sh\necho 'broken candidate'\n";
        let staged = slot_mgr.stage_binary(slot_b_bytes, None).unwrap();
        slot_mgr.atomic_swap(&staged, BinarySlot::SlotB).unwrap();
        assert_eq!(slot_mgr.detect_active_slot().unwrap(), BinarySlot::SlotB);

        // Arm sentinel probation for slot_b candidate
        let sentinel_config = SentinelConfig {
            state_file: temp_dir.path().join("sentinel.json"),
            health_gate_duration: Duration::from_millis(200),
            poll_interval: Duration::from_millis(50),
            max_consecutive_failures: 3,
            quarantine_duration: Duration::from_secs(3600),
        };
        let sentinel = RollbackSentinel::with_config(sentinel_config);
        sentinel
            .start_probation("broken_sha", "old_good_sha", "slot_b", "slot_a")
            .unwrap();

        // Simulate health probe failing (HTTP 500 or timeout)
        let decision = sentinel
            .supervise_health_gate(&slot_mgr, || async { Ok(false) })
            .await
            .unwrap();

        match decision {
            HealthGateDecision::RolledBack {
                sha, target_slot, ..
            } => {
                assert_eq!(sha, "broken_sha");
                assert_eq!(target_slot, "slot_a");
            }
            other => panic!("Expected RolledBack decision, got {:?}", other),
        }

        // Verify autonomous rollback reverted active pointer back to slot_a!
        assert_eq!(slot_mgr.detect_active_slot().unwrap(), BinarySlot::SlotA);
        let final_state = sentinel.load_state();
        assert_eq!(final_state.status, SentinelStatus::RolledBack);
        assert_eq!(final_state.consecutive_failures, 1);
    }

    #[tokio::test]
    async fn test_sentinel_circuit_breaker_quarantine_tripping() {
        let temp_dir = tempfile::tempdir().unwrap();
        let slot_mgr = SlotManager::new(temp_dir.path(), "xavier").unwrap();

        let sentinel_config = SentinelConfig {
            state_file: temp_dir.path().join("sentinel.json"),
            health_gate_duration: Duration::from_millis(100),
            poll_interval: Duration::from_millis(30),
            max_consecutive_failures: 3,
            quarantine_duration: Duration::from_secs(3600),
        };
        let sentinel = RollbackSentinel::with_config(sentinel_config);

        // Setup dummy backup file for rollback
        fs::write(slot_mgr.backup_binary_path(), b"good binary").unwrap();

        // Simulate 3 consecutive rollbacks
        for i in 1..=3 {
            sentinel
                .start_probation(&format!("fail_sha_{}", i), "old_sha", "slot_b", "slot_a")
                .unwrap();

            let decision = sentinel
                .supervise_health_gate(&slot_mgr, || async { Ok(false) })
                .await
                .unwrap();

            if i < 3 {
                assert!(matches!(decision, HealthGateDecision::RolledBack { .. }));
            } else {
                // On 3rd failure, circuit breaker must trip to Quarantined!
                match decision {
                    HealthGateDecision::Quarantined {
                        consecutive_failures,
                        ..
                    } => {
                        assert_eq!(consecutive_failures, 3);
                    }
                    other => panic!("Expected Quarantined on 3rd failure, got {:?}", other),
                }
            }
        }

        assert!(sentinel.is_quarantined());

        // AutoUpdateService with quarantined sentinel returns quarantined report
        let service = AutoUpdateService::new().with_resilience(slot_mgr, sentinel);
        let report = service.run_startup_check().await.unwrap();
        assert_eq!(report.status, "quarantined");
        assert!(!report.applied);
    }
}
