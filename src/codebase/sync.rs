//! Git HEAD Auto-Sync Monitor for CodeGraph.
//!
//! Checks for changes in the workspace's Git HEAD commit hash and triggers
//! incremental CodeGraph delta synchronization automatically.

use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

use crate::cli::codegraph_sync::{check_and_sync_if_head_changed, GitSyncResult};

/// Default polling interval to check for git HEAD updates.
pub const DEFAULT_GIT_HEAD_POLL_INTERVAL: Duration = Duration::from_secs(5);

/// Background monitor that checks for Git HEAD changes and triggers CodeGraph sync.
pub struct GitHeadWatcher {
    workspace: PathBuf,
    task_handle: Option<JoinHandle<()>>,
}

impl GitHeadWatcher {
    /// Starts background monitoring for Git HEAD changes in `workspace`.
    pub fn start(workspace: PathBuf, poll_interval: Option<Duration>) -> Self {
        let interval = poll_interval.unwrap_or(DEFAULT_GIT_HEAD_POLL_INTERVAL);
        let ws = workspace.clone();

        let task_handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                ticker.tick().await;
                match check_and_sync_if_head_changed(&ws).await {
                    Ok(Some(sync_result)) => {
                        info!(
                            head = %sync_result.head,
                            changes = sync_result.changed_paths,
                            "CodeGraph auto-synced on Git HEAD transition"
                        );
                    }
                    Ok(None) => {
                        // HEAD unchanged or not a git repository
                    }
                    Err(e) => {
                        debug!(
                            workspace = %ws.display(),
                            "Git HEAD auto-sync check error: {e}"
                        );
                    }
                }
            }
        });

        info!(workspace = %workspace.display(), "GitHeadWatcher started");

        Self {
            workspace,
            task_handle: Some(task_handle),
        }
    }

    /// Workspace path monitored by this watcher.
    pub fn workspace(&self) -> &Path {
        &self.workspace
    }

    /// Triggers an immediate one-shot HEAD check and sync without waiting for the timer.
    pub async fn check_now(&self) -> anyhow::Result<Option<GitSyncResult>> {
        check_and_sync_if_head_changed(&self.workspace).await
    }

    /// Stops the background polling task.
    pub async fn stop(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
            let _ = handle.await;
        }
        info!(workspace = %self.workspace.display(), "GitHeadWatcher stopped");
    }
}

impl Drop for GitHeadWatcher {
    fn drop(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_git_head_watcher_lifecycle() {
        let temp = TempDir::new().expect("tempdir");
        let mut watcher =
            GitHeadWatcher::start(temp.path().to_path_buf(), Some(Duration::from_millis(50)));

        assert_eq!(watcher.workspace(), temp.path());

        // Calling check_now on a non-git directory fails open (returns Ok(None))
        let res = watcher.check_now().await;
        assert!(res.is_ok());
        assert!(res.unwrap().is_none());

        watcher.stop().await;
    }
}
