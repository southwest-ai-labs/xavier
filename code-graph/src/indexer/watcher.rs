//! Reactive Filesystem Watcher for CodeGraph.
//!
//! Watches repository directories for file additions, modifications, and deletions,
//! debounces filesystem burst events, and incrementally applies them to [`Indexer`].

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use crate::error::{GraphError, Result};
use crate::indexer::{Indexer, PathChange, PathChangeKind};

/// Default debounce window for filesystem bursts (e.g. git checkout, format on save).
pub const DEFAULT_DEBOUNCE_MS: u64 = 500;

/// Ignored directory names that are never indexed.
const IGNORED_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    ".xavier",
    "dist",
    "build",
    ".cargo",
    ".idea",
    ".vscode",
];

/// Recognized source code extensions that trigger index updates.
const CODE_EXTENSIONS: &[&str] = &[
    "rs", "ts", "js", "tsx", "jsx", "py", "go", "java", "c", "cpp", "cc", "cxx", "h", "hpp",
];

/// Checks if a file path should be ignored by the watcher.
pub fn is_ignored_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    for part in path_str.split(['/', '\\']) {
        if IGNORED_DIRS.contains(&part) {
            return true;
        }
    }
    for comp in path.components() {
        if let Component::Normal(os_str) = comp {
            if let Some(s) = os_str.to_str() {
                if IGNORED_DIRS.contains(&s) {
                    return true;
                }
            }
        }
    }
    false
}

/// Checks if a path has a supported source code file extension.
pub fn is_supported_code_file(path: &Path) -> bool {
    if is_ignored_path(path) {
        return false;
    }
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => CODE_EXTENSIONS.contains(&ext),
        None => false,
    }
}

/// Reactive debounced filesystem watcher for incremental CodeGraph indexing.
pub struct CodeGraphWatcher {
    root: PathBuf,
    watcher: Option<RecommendedWatcher>,
    task_handle: Option<JoinHandle<()>>,
}

impl CodeGraphWatcher {
    /// Start watching the specified root directory for changes.
    pub fn start(indexer: Arc<Indexer>, root: PathBuf, debounce: Option<Duration>) -> Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default(),
        )
        .map_err(|e| GraphError::Io(std::io::Error::other(e.to_string())))?;

        watcher
            .watch(&root, RecursiveMode::Recursive)
            .map_err(|e| GraphError::Io(std::io::Error::other(e.to_string())))?;

        let debounce_dur = debounce.unwrap_or_else(|| Duration::from_millis(DEFAULT_DEBOUNCE_MS));
        let root_clone = root.clone();

        let task_handle = tokio::spawn(async move {
            Self::event_loop(indexer, root_clone, rx, debounce_dur).await;
        });

        info!(root = %root.display(), "CodeGraph reactive watcher started");

        Ok(Self {
            root,
            watcher: Some(watcher),
            task_handle: Some(task_handle),
        })
    }

    /// Root directory monitored by this watcher.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Internal event processing loop with debouncing.
    async fn event_loop(
        indexer: Arc<Indexer>,
        root: PathBuf,
        mut rx: mpsc::UnboundedReceiver<notify::Result<Event>>,
        debounce_dur: Duration,
    ) {
        let mut pending_changes: HashMap<PathBuf, PathChangeKind> = HashMap::new();
        let mut debounce_deadline: Option<tokio::time::Instant> = None;

        loop {
            tokio::select! {
                maybe_event = rx.recv() => {
                    match maybe_event {
                        Some(Ok(event)) => {
                            for path in event.paths {
                                if !is_supported_code_file(&path) {
                                    continue;
                                }

                                match event.kind {
                                    EventKind::Create(_) => {
                                        pending_changes.insert(path, PathChangeKind::Added);
                                    }
                                    EventKind::Modify(_) => {
                                        pending_changes
                                            .entry(path)
                                            .and_modify(|k| {
                                                if *k != PathChangeKind::Added {
                                                    *k = PathChangeKind::Modified;
                                                }
                                            })
                                            .or_insert(PathChangeKind::Modified);
                                    }
                                    EventKind::Remove(_) => {
                                        pending_changes.insert(path, PathChangeKind::Deleted);
                                    }
                                    _ => {}
                                }
                            }

                            if !pending_changes.is_empty() {
                                debounce_deadline = Some(tokio::time::Instant::now() + debounce_dur);
                            }
                        }
                        Some(Err(e)) => {
                            warn!("CodeGraph watcher error: {e}");
                        }
                        None => {
                            // Channel closed
                            break;
                        }
                    }
                }
                _ = async {
                    if let Some(deadline) = debounce_deadline {
                        tokio::time::sleep_until(deadline).await;
                    } else {
                        // Pending indefinitely if no deadline is set
                        std::future::pending::<()>().await;
                    }
                }, if debounce_deadline.is_some() => {
                    debounce_deadline = None;
                    if pending_changes.is_empty() {
                        continue;
                    }

                    let batch: Vec<(PathBuf, PathChangeKind)> = pending_changes.drain().collect();
                    let mut path_changes = Vec::with_capacity(batch.len());

                    for (abs_path, kind) in batch {
                        let rel_path = match abs_path.strip_prefix(&root) {
                            Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
                            Err(_) => abs_path.to_string_lossy().replace('\\', "/"),
                        };

                        let change = match kind {
                            PathChangeKind::Added => PathChange::added(rel_path),
                            PathChangeKind::Modified => PathChange::modified(rel_path),
                            PathChangeKind::Deleted => PathChange::deleted(rel_path),
                            PathChangeKind::Renamed { from } => PathChange::renamed(from, rel_path),
                        };
                        path_changes.push(change);
                    }

                    debug!(
                        count = path_changes.len(),
                        root = %root.display(),
                        "Applying reactive debounced changes to CodeGraph"
                    );

                    if let Err(e) = indexer.apply_paths(&root, &path_changes).await {
                        error!("Failed to apply reactive changes to CodeGraph: {e}");
                    }
                }
            }
        }
    }

    /// Stops the watcher and cancels the background event loop.
    pub async fn stop(&mut self) {
        if let Some(mut watcher) = self.watcher.take() {
            let _ = watcher.unwatch(&self.root);
        }
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
            let _ = handle.await;
        }
        info!(root = %self.root.display(), "CodeGraph reactive watcher stopped");
    }
}

impl Drop for CodeGraphWatcher {
    fn drop(&mut self) {
        if let Some(mut watcher) = self.watcher.take() {
            let _ = watcher.unwatch(&self.root);
        }
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::CodeGraphDB;
    use tempfile::TempDir;

    #[test]
    fn test_is_ignored_path() {
        assert!(is_ignored_path(Path::new("/project/.git/HEAD")));
        assert!(is_ignored_path(Path::new(
            "C:\\project\\target\\debug\\app.exe"
        )));
        assert!(is_ignored_path(Path::new(
            "project/node_modules/pkg/index.js"
        )));
        assert!(is_ignored_path(Path::new("/project/.xavier/code_graph.db")));
        assert!(!is_ignored_path(Path::new("/project/src/lib.rs")));
    }

    #[test]
    fn test_is_supported_code_file() {
        assert!(is_supported_code_file(Path::new("src/main.rs")));
        assert!(is_supported_code_file(Path::new("frontend/app.tsx")));
        assert!(is_supported_code_file(Path::new("backend/server.py")));
        assert!(!is_supported_code_file(Path::new("README.md")));
        assert!(!is_supported_code_file(Path::new(".git/config")));
        assert!(!is_supported_code_file(Path::new("target/debug/build.rs")));
    }

    #[tokio::test]
    async fn test_watcher_creation_and_shutdown() {
        let temp = TempDir::new().expect("tempdir");
        let db = Arc::new(CodeGraphDB::in_memory().expect("db"));
        let indexer = Arc::new(Indexer::new(db));

        let mut watcher = CodeGraphWatcher::start(
            indexer,
            temp.path().to_path_buf(),
            Some(Duration::from_millis(50)),
        )
        .expect("start watcher");

        assert_eq!(watcher.root(), temp.path());
        watcher.stop().await;
    }
}
