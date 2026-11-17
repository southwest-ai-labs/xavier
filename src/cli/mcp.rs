//! MCP transports — stdio and HTTP+SSE entry points.
//!
//! Both transports share a single [`build_mcp_state`] that initializes the
//! memory store, builds the shared application state, and authenticates the
//! MCP workspace. The actual JSON-RPC dispatch lives in
//! [`xavier::server::mcp::session::dispatch_mcp_value`], so both transports
//! expose identical protocol behavior with no duplicated logic.

use anyhow::Result;
use std::sync::Arc;
use xavier::app::security_service::SecurityService;
use xavier::server::mcp::transport::start_mcp_http_server;
use xavier::server::mcp_stdio::run_stdio_loop;
use xavier::settings::XavierSettings;
use xavier::workspace::{WorkspaceConfig, WorkspaceRegistry, WorkspaceState};
use xavier::AppState;

use crate::cli::codegraph_sync::find_git_root;
use crate::cli::config::resolve_http_bind_host;

/// Default port for the MCP HTTP+SSE server.
pub const DEFAULT_MCP_PORT: u16 = 8100;

/// Build the shared MCP application state and authenticated workspace.
///
/// Used by both the stdio and HTTP+SSE transports so they share identical
/// initialization and core wiring (memory store, security service, workspace).
pub async fn build_mcp_state() -> Result<(AppState, xavier::workspace::WorkspaceContext)> {
    let workspace_id = crate::cli::utils::resolve_active_workspace();

    // Build the shared application state so the unified MCP dispatcher has
    // everything it needs (security service, workspace registry, etc.).
    let security_service = Arc::new(SecurityService::new());
    let workspace_registry = Arc::new(WorkspaceRegistry::new());

    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    // Check canonical workspace locations in order:
    // 0) Explicit XAVIER_CODE_GRAPH_DB_PATH override (if set and exists)
    // a) Local workspace `.xavier/code_graph.db`
    // b) `XavierSettings::resolve_data_dir().join("code_graph.db")`
    // c) If in a git repository, `<git_root>/.xavier/code_graph.db`
    let env_code_db = std::env::var("XAVIER_CODE_GRAPH_DB_PATH")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(std::path::PathBuf::from);
    let local_code_db = cwd.join(".xavier").join("code_graph.db");
    let data_dir_code_db = XavierSettings::resolve_data_dir().join("code_graph.db");
    let git_root_code_db =
        find_git_root(&cwd).map(|git_root| git_root.join(".xavier").join("code_graph.db"));

    let found_code_db_path = if env_code_db.as_ref().is_some_and(|p| p.exists()) {
        env_code_db.clone()
    } else if local_code_db.exists() {
        Some(local_code_db.clone())
    } else if data_dir_code_db.exists() {
        Some(data_dir_code_db)
    } else if git_root_code_db.as_ref().is_some_and(|p| p.exists()) {
        git_root_code_db
    } else {
        None
    };

    let code_db = if let Some(ref code_db_path) = found_code_db_path {
        match code_graph::db::CodeGraphDB::new(code_db_path) {
            Ok(db) => {
                tracing::info!(
                    "MCP loaded persistent CodeGraphDB from {}",
                    code_db_path.display()
                );
                Arc::new(db)
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to open persistent CodeGraphDB at {}, falling back to in_memory: {}",
                    code_db_path.display(),
                    e
                );
                Arc::new(code_graph::db::CodeGraphDB::in_memory()?)
            }
        }
    } else {
        let warning_path = env_code_db.unwrap_or(local_code_db);
        tracing::warn!(
            path = %warning_path.display(),
            "No persistent CodeGraphDB found at {}; initializing empty in-memory graph. Run 'xavier code scan .' to index this workspace",
            warning_path.display()
        );
        Arc::new(code_graph::db::CodeGraphDB::in_memory()?)
    };
    let code_indexer = Arc::new(code_graph::indexer::Indexer::new(Arc::clone(&code_db)));
    let code_query = Arc::new(code_graph::query::QueryEngine::new(Arc::clone(&code_db)));

    let state = AppState {
        workspace_registry: Arc::clone(&workspace_registry),
        code_indexer: Arc::clone(&code_indexer),
        code_query,
        code_db,
        indexer: xavier::memory::file_indexer::FileIndexer::new(
            xavier::memory::file_indexer::FileIndexerConfig::default(),
            Some(code_indexer.clone()),
        ),
        agent_indexer: xavier::memory::agent_indexer::AgentIndexer::new(
            xavier::memory::file_indexer::FileIndexer::new(
                xavier::memory::file_indexer::FileIndexerConfig::default(),
                Some(code_indexer.clone()),
            ),
        ),
        security_service,
        code_graph_dump_path: Some(xavier::codebase::codegraph_paths::codegraph_dump_path_for(
            &cwd,
        )),
        telecom: None,
    };

    let workspace = WorkspaceState::new(
        WorkspaceConfig {
            id: workspace_id.clone(),
            token: "mcp-token".to_string(),
            plan: xavier::workspace::PlanTier::Personal,
            memory_backend: xavier::memory::store::MemoryBackend::Vec,
            storage_limit_bytes: None,
            request_limit: None,
            request_unit_limit: None,
            embedding_provider_mode: xavier::workspace::EmbeddingProviderMode::BringYourOwn,
            managed_google_embeddings: false,
            sync_policy: xavier::workspace::SyncPolicy::CloudMirror,
            protocol: Default::default(),
            dedup: xavier::settings::types::DedupSettings::default(),
        },
        xavier::agents::RuntimeConfig::default(),
        std::env::temp_dir().join("xavier-mcp"),
    )
    .await?;
    workspace_registry.insert(workspace).await?;
    let workspace = workspace_registry
        .authenticate("mcp-token")
        .await
        .ok_or_else(|| anyhow::anyhow!("failed to authenticate MCP workspace"))?;

    Ok((state, workspace))
}

/// `xavier mcp` — start the MCP stdio transport (for local/OpenClaw integration).
pub async fn start_mcp_stdio() -> Result<()> {
    let (state, workspace) = build_mcp_state().await?;
    run_stdio_loop(state, workspace).await
}

/// Start the MCP HTTP+SSE (Streamable HTTP) transport on `port`.
///
/// Binds to the resolved HTTP host (default `127.0.0.1`). Intended to be
/// spawned alongside the main HTTP server or run standalone.
pub async fn start_mcp_http(port: u16) -> Result<()> {
    let (state, workspace) = build_mcp_state().await?;
    let bind_host = resolve_http_bind_host();
    let bind_addr = format!("{bind_host}:{port}");
    tracing::info!("Starting Xavier MCP HTTP+SSE server on {}", bind_addr);
    start_mcp_http_server(state, workspace, bind_addr).await
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestEnvGuard {
        prev_cwd: Option<std::path::PathBuf>,
        vars: Vec<(&'static str, Option<String>)>,
    }

    impl TestEnvGuard {
        fn new(keys: &[&'static str]) -> Self {
            let vars = keys.iter().map(|&k| (k, std::env::var(k).ok())).collect();
            Self {
                prev_cwd: std::env::current_dir().ok(),
                vars,
            }
        }
    }

    impl Drop for TestEnvGuard {
        fn drop(&mut self) {
            if let Some(ref cwd) = self.prev_cwd {
                let _ = std::env::set_current_dir(cwd);
            }
            for (k, v) in &self.vars {
                if let Some(v) = v {
                    std::env::set_var(k, v);
                } else {
                    std::env::remove_var(k);
                }
            }
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_build_mcp_state_missing_db_fallback_warning() {
        let temp = tempfile::tempdir().expect("tempdir");
        let _guard = TestEnvGuard::new(&[
            "XAVIER_CODE_GRAPH_DB_PATH",
            "XAVIER_EMBEDDER",
            "XAVIER_TOKEN",
        ]);
        std::env::remove_var("XAVIER_CODE_GRAPH_DB_PATH");
        std::env::set_var("XAVIER_EMBEDDER", "disabled");
        std::env::set_var("XAVIER_TOKEN", "mcp-token");
        std::env::set_current_dir(temp.path()).expect("set_current_dir");

        let res = build_mcp_state().await;
        assert!(
            res.is_ok(),
            "build_mcp_state should succeed when DB is missing: {:?}",
            res.err()
        );
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_build_mcp_state_loads_local_db() {
        let temp = tempfile::tempdir().expect("tempdir");
        let xavier_dir = temp.path().join(".xavier");
        std::fs::create_dir_all(&xavier_dir).expect("create_dir_all");
        let db_path = xavier_dir.join("code_graph.db");
        {
            let _db = code_graph::db::CodeGraphDB::new(&db_path).expect("create db");
        }

        let _guard = TestEnvGuard::new(&[
            "XAVIER_CODE_GRAPH_DB_PATH",
            "XAVIER_EMBEDDER",
            "XAVIER_TOKEN",
        ]);
        std::env::remove_var("XAVIER_CODE_GRAPH_DB_PATH");
        std::env::set_var("XAVIER_EMBEDDER", "disabled");
        std::env::set_var("XAVIER_TOKEN", "mcp-token");
        std::env::set_current_dir(temp.path()).expect("set_current_dir");

        let res = build_mcp_state().await;
        assert!(
            res.is_ok(),
            "build_mcp_state should load local DB: {:?}",
            res.err()
        );
    }
}
