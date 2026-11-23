//! HTTP handlers for memory synchronisation endpoints.
//!
//! Exposes peer-to-peer memory sync over the REST API:
//!
//! - `POST /api/v1/memory/sync/push`                — push local changes to a remote peer
//! - `POST /api/v1/memory/sync/pull`                — pull changes from a remote peer
//! - `GET  /api/v1/memory/sync/status`              — current sync status / last session
//! - `POST /api/v1/memory/sync/resolve/{conflict_id}` — explicitly resolve a conflict
//!
//! These routes live on the same router as the rest of `routes.rs`, whose state
//! type is `Arc<dyn AgentLifecyclePort>`. Because that state does not carry a
//! `MemoryStore`, the handlers read the active `PeerMemorySync` from a
//! module-level singleton (mirroring the `TIME_STORE` / `HEALTH_PORT` pattern
//! already used in `routes.rs`). `init_memory_sync` wires the singleton at
//! startup; until then the handlers report `not_initialized` rather than panic.

use std::sync::{Arc, LazyLock, RwLock};
use std::time::{Duration, SystemTime};

use super::{error_json, error_response};
use axum::{extract::Path, http::StatusCode, Json};
use serde::Deserialize;
use serde_json::json;

use crate::memory::sync::{
    merge::deserialise_chunk, ChunkDiff, PeerMemorySync, SyncError, SyncSession,
};

/// Default workspace used when a request omits `workspace_id`.
const DEFAULT_WORKSPACE: &str = "default";

// ---------------------------------------------------------------------------
// Module-level singletons
// ---------------------------------------------------------------------------

/// The active sync service. Set once at startup via [`init_memory_sync`].
static MEMORY_SYNC: LazyLock<RwLock<Option<Arc<PeerMemorySync>>>> =
    LazyLock::new(|| RwLock::new(None));

/// The most recently completed sync session (set after each push/pull).
static LAST_SESSION: LazyLock<RwLock<Option<SyncSession>>> = LazyLock::new(|| RwLock::new(None));

/// Conflict IDs that have been explicitly resolved via the resolve endpoint.
static RESOLVED_CONFLICTS: LazyLock<RwLock<Vec<String>>> =
    LazyLock::new(|| RwLock::new(Vec::new()));

/// Wire the active `PeerMemorySync` into the module singleton.
///
/// Call once at startup. Safe to call again to replace the active service
/// (used by tests).
pub fn init_memory_sync(sync: Arc<PeerMemorySync>) {
    if let Ok(mut guard) = MEMORY_SYNC.write() {
        *guard = Some(sync);
    } else {
        tracing::error!("MEMORY_SYNC lock poisoned while initialising memory sync");
    }
}

/// Reset the module-level sync singletons to their pristine state.
///
/// Test-only helper: the singletons are process-global and never reset in
/// production, so an integration test that probes the *uninitialised* status
/// needs a way to clear state left behind by another test in the same binary.
/// `#[cfg(test)]` cannot be used here: integration tests link against the
/// library compiled **without** `cfg(test)`, so the item must be a real `pub`
/// (hidden from docs) to be reachable from `tests/`.
#[doc(hidden)]
pub fn reset_memory_sync_for_test() {
    if let Ok(mut guard) = MEMORY_SYNC.write() {
        *guard = None;
    }
    if let Ok(mut guard) = LAST_SESSION.write() {
        *guard = None;
    }
    if let Ok(mut guard) = RESOLVED_CONFLICTS.write() {
        guard.clear();
    }
}

/// Snapshot of the active sync service, if one has been initialised.
fn current_sync() -> Option<Arc<PeerMemorySync>> {
    MEMORY_SYNC.read().ok().and_then(|guard| guard.clone())
}

/// Record the outcome of a completed sync session.
fn set_last_session(session: SyncSession) {
    if let Ok(mut guard) = LAST_SESSION.write() {
        *guard = Some(session);
    }
}

// ---------------------------------------------------------------------------
// Request / response payloads
// ---------------------------------------------------------------------------

/// Body for push/pull requests.
#[derive(Debug, Deserialize)]
pub struct SyncPeerRequest {
    /// Base URL of the remote peer, e.g. `http://peer.local:8080`.
    pub peer_url: String,
    /// Workspace to sync. Defaults to [`DEFAULT_WORKSPACE`].
    #[serde(default)]
    pub workspace_id: Option<String>,
    /// Only sync records newer than this. Accepts epoch seconds or RFC 3339.
    /// Defaults to "all history" (UNIX_EPOCH) when omitted.
    #[serde(default)]
    pub since: Option<String>,
}

/// Body for conflict-resolution requests.
#[derive(Debug, Deserialize)]
pub struct SyncResolveRequest {
    /// Which side wins: `"local"` (keep ours) or `"remote"` (take theirs).
    pub resolution: String,
    /// The remote chunk to force-apply when `resolution == "remote"`.
    #[serde(default)]
    pub chunk: Option<ChunkDiff>,
}

use axum::response::IntoResponse;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SyncStatusResponse {
    pub status: &'static str,
    pub initialized: bool,
    pub node_id: String,
    pub sync_interval_secs: u64,
    pub last_session: Option<SyncSession>,
    pub resolved_conflicts: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SyncSuccessResponse {
    pub status: &'static str,
    pub session: SyncSession,
}

#[derive(Debug, Serialize)]
pub struct SyncErrorResponse {
    pub status: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SyncResolveResponse {
    pub status: &'static str,
    pub conflict_id: String,
    pub resolution: String,
    pub applied: bool,
}

#[derive(Debug, Serialize)]
pub struct SyncResolveErrorResponse {
    pub status: &'static str,
    pub message: &'static str,
    pub conflict_id: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `GET /api/v1/memory/sync/status` — report sync status.
pub async fn sync_status_handler() -> impl IntoResponse {
    let sync = current_sync();

    let (node_id, sync_interval_secs) = sync
        .as_ref()
        .map(|s| (s.node_id.clone(), s.sync_interval.as_secs()))
        .unwrap_or_else(|| (String::new(), 0));

    let last_session = LAST_SESSION.read().ok().and_then(|guard| guard.clone());

    let resolved_conflicts = RESOLVED_CONFLICTS
        .read()
        .ok()
        .map(|guard| guard.clone())
        .unwrap_or_default();

    Json(SyncStatusResponse {
        status: "ok",
        initialized: sync.is_some(),
        node_id,
        sync_interval_secs,
        last_session,
        resolved_conflicts,
    })
}

fn sync_uninitialized_response() -> (StatusCode, Json<SyncErrorResponse>) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(SyncErrorResponse {
            status: "error",
            message: "memory sync not initialized".to_string(),
            peer_url: None,
        }),
    )
}

fn sync_error_response(e: SyncError, peer_url: String) -> (StatusCode, Json<SyncErrorResponse>) {
    let status_code = match &e {
        SyncError::Http { status, .. } => {
            StatusCode::from_u16(*status).unwrap_or(StatusCode::BAD_GATEWAY)
        }
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status_code,
        Json(SyncErrorResponse {
            status: "error",
            message: e.to_string(),
            peer_url: Some(peer_url),
        }),
    )
}

/// `POST /api/v1/memory/sync/push` — push local changes to a remote peer.
pub async fn sync_push_handler(Json(req): Json<SyncPeerRequest>) -> impl IntoResponse {
    if let Err(msg) = validate_peer_url(&req.peer_url) {
        return (
            StatusCode::BAD_REQUEST,
            Json(SyncErrorResponse {
                status: "error",
                message: msg.to_string(),
                peer_url: Some(req.peer_url),
            }),
        )
            .into_response();
    }

    let workspace_id = req
        .workspace_id
        .unwrap_or_else(|| DEFAULT_WORKSPACE.to_string());

    if let Err(msg) = validate_workspace_id(&workspace_id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(SyncErrorResponse {
                status: "error",
                message: msg.to_string(),
                peer_url: Some(req.peer_url),
            }),
        )
            .into_response();
    }

    let Some(sync) = current_sync() else {
        return sync_uninitialized_response().into_response();
    };

    let since = parse_since(&req.since);

    match sync.push_to(&req.peer_url, &workspace_id, since).await {
        Ok(session) => {
            set_last_session(session.clone());
            (
                StatusCode::OK,
                Json(SyncSuccessResponse {
                    status: "ok",
                    session,
                }),
            )
                .into_response()
        }
        Err(e) => sync_error_response(e, req.peer_url).into_response(),
    }
}

/// `POST /api/v1/memory/sync/pull` — pull changes from a remote peer.
pub async fn sync_pull_handler(Json(req): Json<SyncPeerRequest>) -> impl IntoResponse {
    if let Err(msg) = validate_peer_url(&req.peer_url) {
        return (
            StatusCode::BAD_REQUEST,
            Json(SyncErrorResponse {
                status: "error",
                message: msg.to_string(),
                peer_url: Some(req.peer_url),
            }),
        )
            .into_response();
    }

    let workspace_id = req
        .workspace_id
        .unwrap_or_else(|| DEFAULT_WORKSPACE.to_string());

    if let Err(msg) = validate_workspace_id(&workspace_id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(SyncErrorResponse {
                status: "error",
                message: msg.to_string(),
                peer_url: Some(req.peer_url),
            }),
        )
            .into_response();
    }

    let Some(sync) = current_sync() else {
        return sync_uninitialized_response().into_response();
    };

    let since = parse_since(&req.since);

    match sync.pull_from(&req.peer_url, &workspace_id, since).await {
        Ok(session) => {
            set_last_session(session.clone());
            (
                StatusCode::OK,
                Json(SyncSuccessResponse {
                    status: "ok",
                    session,
                }),
            )
                .into_response()
        }
        Err(e) => sync_error_response(e, req.peer_url).into_response(),
    }
}

/// `POST /api/v1/memory/sync/resolve/{conflict_id}` — resolve a conflict.
///
/// - `resolution: "local"`  → keep the local record (no-op beyond recording).
/// - `resolution: "remote"` → force-apply the provided `chunk` to the store,
///   bypassing LWW so the remote version wins unconditionally.
///
/// The resolution is recorded (idempotently) and surfaced by the status
/// endpoint.
pub async fn sync_resolve_handler(
    Path(conflict_id): Path<String>,
    Json(req): Json<SyncResolveRequest>,
) -> impl IntoResponse {
    let resolution = req.resolution.trim().to_lowercase();
    if resolution != "local" && resolution != "remote" {
        return (
            StatusCode::BAD_REQUEST,
            Json(SyncResolveErrorResponse {
                status: "error",
                message: "resolution must be 'local' or 'remote'",
                conflict_id,
            }),
        )
            .into_response();
    }

    let mut applied = false;
    if resolution == "remote" {
        if let Some(sync) = current_sync() {
            if let Some(chunk) = req.chunk.as_ref() {
                if let Some(data) = chunk.data.as_ref() {
                    match deserialise_chunk(data) {
                        Ok(record) => match sync.store().put(record).await {
                            Ok(()) => applied = true,
                            Err(e) => {
                                tracing::warn!(
                                    "sync resolve {}: failed to apply remote chunk: {e}",
                                    conflict_id
                                );
                            }
                        },
                        Err(e) => {
                            tracing::warn!(
                                "sync resolve {}: cannot deserialise remote chunk: {e}",
                                conflict_id
                            );
                        }
                    }
                }
            }
        }
    }

    // Record the resolution idempotently.
    if let Ok(mut guard) = RESOLVED_CONFLICTS.write() {
        if !guard.iter().any(|id| id == &conflict_id) {
            guard.push(conflict_id.clone());
        }
    }

    Json(SyncResolveResponse {
        status: "resolved",
        conflict_id,
        resolution,
        applied,
    })
    .into_response()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Validate that a peer URL is well-formed http/https, has a valid host, and does not target
/// private or internal IP ranges (SSRF protection) unless loopback/localhost or explicitly
/// permitted via `XAVIER_ALLOWED_SYNC_PEERS`.
pub fn validate_peer_url(peer_url: &str) -> Result<(), &'static str> {
    let parsed = url::Url::parse(peer_url).map_err(|_| "Invalid peer_url: failed to parse URL")?;
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err("Invalid peer_url scheme: must be http or https");
    }
    let host_str = match parsed.host_str() {
        Some(h) if !h.is_empty() => h,
        _ => return Err("Invalid peer_url: missing host"),
    };

    if is_peer_allowed_by_env(peer_url, host_str) {
        return Ok(());
    }

    if host_str.eq_ignore_ascii_case("localhost") {
        return Ok(());
    }

    if let Some(host) = parsed.host() {
        match host {
            url::Host::Ipv4(ipv4) => {
                if is_private_or_internal_ip(std::net::IpAddr::V4(ipv4)) {
                    return Err("peer_url targets a private or internal network address");
                }
            }
            url::Host::Ipv6(ipv6) => {
                if is_private_or_internal_ip(std::net::IpAddr::V6(ipv6)) {
                    return Err("peer_url targets a private or internal network address");
                }
            }
            url::Host::Domain(_) => {}
        }
    }

    Ok(())
}

fn is_private_or_internal_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(ipv4) => {
            if ipv4.is_loopback() {
                return false;
            }
            if ipv4.is_private()
                || ipv4.is_link_local()
                || ipv4.is_broadcast()
                || ipv4.is_unspecified()
            {
                return true;
            }
            let octets = ipv4.octets();
            // Carrier-grade NAT (100.64.0.0/10)
            if octets[0] == 100 && (64..=127).contains(&octets[1]) {
                return true;
            }
            // Benchmarking (198.18.0.0/15)
            if octets[0] == 198 && (18..=19).contains(&octets[1]) {
                return true;
            }
            // Documentation (192.0.2.0/24, 198.51.100.0/24, 203.0.113.0/24)
            if (octets[0] == 192 && octets[1] == 0 && octets[2] == 2)
                || (octets[0] == 198 && octets[1] == 51 && octets[2] == 100)
                || (octets[0] == 203 && octets[1] == 0 && octets[2] == 113)
            {
                return true;
            }
            false
        }
        std::net::IpAddr::V6(ipv6) => {
            if ipv6.is_loopback() {
                return false;
            }
            if ipv6.is_unspecified() {
                return true;
            }
            if let Some(ipv4) = ipv6.to_ipv4_mapped() {
                return is_private_or_internal_ip(std::net::IpAddr::V4(ipv4));
            }
            // Unique local addresses (fc00::/7)
            if (ipv6.segments()[0] & 0xfe00) == 0xfc00 {
                return true;
            }
            // Link-local unicast (fe80::/10)
            if (ipv6.segments()[0] & 0xffc0) == 0xfe80 {
                return true;
            }
            false
        }
    }
}

fn is_peer_allowed_by_env(peer_url: &str, host_str: &str) -> bool {
    let Ok(allowed) = std::env::var("XAVIER_ALLOWED_SYNC_PEERS") else {
        return false;
    };
    for entry in allowed.split(',') {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.eq_ignore_ascii_case(peer_url) || trimmed.eq_ignore_ascii_case(host_str) {
            return true;
        }
        if let Ok(entry_url) = url::Url::parse(trimmed) {
            if let Some(entry_host) = entry_url.host_str() {
                if entry_host.eq_ignore_ascii_case(host_str) {
                    return true;
                }
            }
        }
    }
    false
}

/// Validate that a workspace ID does not contain path traversal sequences (`..`, `/`, `\`).
pub fn validate_workspace_id(workspace_id: &str) -> Result<(), &'static str> {
    if workspace_id.contains("..") || workspace_id.contains('/') || workspace_id.contains('\\') {
        return Err("Invalid workspace_id: path traversal sequences are forbidden");
    }
    Ok(())
}

/// Parse an optional `since` cursor into a `SystemTime`.
///
/// Accepts:
/// - epoch seconds (`"0"`, `"1700000000"`)
/// - RFC 3339 timestamps (`"2026-01-01T00:00:00Z"`)
///
/// `None` / empty / unparseable values fall back to `UNIX_EPOCH` (i.e. sync
/// all history), which matches the one-shot push/pull "everything" semantics.
fn parse_since(since: &Option<String>) -> SystemTime {
    let Some(raw) = since else {
        return SystemTime::UNIX_EPOCH;
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return SystemTime::UNIX_EPOCH;
    }
    if let Ok(secs) = trimmed.parse::<u64>() {
        return SystemTime::UNIX_EPOCH + Duration::from_secs(secs);
    }
    if let Ok(ts) = trimmed.parse::<jiff::Timestamp>() {
        if ts.as_nanosecond() >= 0 {
            return SystemTime::UNIX_EPOCH + Duration::from_nanos(ts.as_nanosecond() as u64);
        }
    }
    SystemTime::UNIX_EPOCH
}

// ---------------------------------------------------------------------------
// Legacy data-plane compatibility handlers
// ---------------------------------------------------------------------------

/// `GET /v1/memory/manifest` — return the full store manifest.
///
/// This is the legacy data-plane endpoint used by `PeerMemorySync` clients
/// to pull the manifest from a remote peer. Delegates to the same
/// underlying store logic as the control-plane.
fn legacy_uninitialized_response() -> (StatusCode, Json<serde_json::Value>) {
    error_response(
        StatusCode::SERVICE_UNAVAILABLE,
        "memory sync not initialized",
    )
}

pub async fn legacy_manifest_handler() -> impl IntoResponse {
    let Some(sync) = current_sync() else {
        return legacy_uninitialized_response().into_response();
    };

    match crate::memory::sync::manifest::build_manifest(sync.store().as_ref()).await {
        Ok(manifest) => (StatusCode::OK, Json(manifest)).into_response(),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to build manifest: {}", e),
        )
        .into_response(),
    }
}

/// `POST /v1/memory/push` — receive a batch of ChunkDiffs from a peer.
///
/// Accepts the old data-plane format: raw `Vec<ChunkDiff>` as JSON body.
/// Applies changes via the same LWW merge logic.
pub async fn legacy_push_handler(
    Json(diffs): Json<Vec<crate::memory::sync::ChunkDiff>>,
) -> impl IntoResponse {
    let Some(sync) = current_sync() else {
        return legacy_uninitialized_response().into_response();
    };

    let mut conflicts = 0u64;
    match crate::memory::sync::merge::apply_changes_received(
        sync.store().as_ref(),
        &diffs,
        &mut conflicts,
    )
    .await
    {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({
                "status": "ok",
                "conflicts": conflicts,
                "received": diffs.len()
            })),
        )
            .into_response(),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to apply changes: {}", e),
        )
        .into_response(),
    }
}

/// `POST /v1/memory/pull` — return diffs for the requested manifest entries.
///
/// Accepts `Vec<ManifestEntry>` (old data-plane format) and returns
/// the corresponding `Vec<ChunkDiff>`.
pub async fn legacy_pull_handler(
    Json(want): Json<Vec<crate::memory::sync::ManifestEntry>>,
) -> impl IntoResponse {
    let Some(sync) = current_sync() else {
        return legacy_uninitialized_response().into_response();
    };

    match crate::memory::sync::push_pull::entries_as_push_diffs(sync.store().as_ref(), &want).await
    {
        Ok(diffs) => (StatusCode::OK, Json(diffs)).into_response(),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to build push diffs: {}", e),
        )
        .into_response(),
    }
}

/// `GET /v1/memory/pull-since/{workspace_id}/{since}` — incremental pull.
///
/// Returns all chunks modified since the given epoch timestamp.
pub async fn legacy_pull_since_handler(
    Path((workspace_id, since_secs)): Path<(String, u64)>,
) -> impl IntoResponse {
    if let Err(msg) = validate_workspace_id(&workspace_id) {
        return error_response(StatusCode::BAD_REQUEST, msg).into_response();
    }

    let Some(sync) = current_sync() else {
        return legacy_uninitialized_response().into_response();
    };

    let since = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(since_secs);
    match crate::memory::sync::push_pull::collect_changes_since(
        sync.store().as_ref(),
        &workspace_id,
        since,
    )
    .await
    {
        Ok(diffs) => (StatusCode::OK, Json(diffs)).into_response(),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to collect changes: {}", e),
        )
        .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_since_none_is_epoch() {
        assert_eq!(parse_since(&None), SystemTime::UNIX_EPOCH);
    }

    #[test]
    fn parse_since_empty_is_epoch() {
        assert_eq!(parse_since(&Some(String::new())), SystemTime::UNIX_EPOCH);
        assert_eq!(
            parse_since(&Some("   ".to_string())),
            SystemTime::UNIX_EPOCH
        );
    }

    #[test]
    fn parse_since_epoch_seconds() {
        let st = parse_since(&Some("0".to_string()));
        assert_eq!(st, SystemTime::UNIX_EPOCH);

        let st = parse_since(&Some("100".to_string()));
        assert_eq!(st, SystemTime::UNIX_EPOCH + Duration::from_secs(100));
    }

    #[test]
    fn parse_since_rfc3339() {
        let st = parse_since(&Some("1970-01-01T00:00:30Z".to_string()));
        assert_eq!(st, SystemTime::UNIX_EPOCH + Duration::from_secs(30));
    }

    #[test]
    fn parse_since_garbage_is_epoch() {
        assert_eq!(
            parse_since(&Some("not-a-date".to_string())),
            SystemTime::UNIX_EPOCH
        );
    }

    // -------------------------------------------------------------------
    // HTTP status code propagation tests
    // -------------------------------------------------------------------

    use axum::{body::Body, http::StatusCode};
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn push_handler_returns_503_when_not_initialized() {
        use axum::response::IntoResponse;
        // Ensure the singleton is empty (default state).
        // The handler should return 503 Service Unavailable.
        let resp = super::sync_push_handler(Json(
            serde_json::from_value(serde_json::json!({
                "peer_url": "http://peer:8080"
            }))
            .unwrap(),
        ))
        .await;
        let response = resp.into_response();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn pull_handler_returns_503_when_not_initialized() {
        use axum::response::IntoResponse;
        let resp = super::sync_pull_handler(Json(
            serde_json::from_value(serde_json::json!({
                "peer_url": "http://peer:8080"
            }))
            .unwrap(),
        ))
        .await;
        let response = resp.into_response();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn resolve_handler_returns_400_for_invalid_resolution() {
        use axum::response::IntoResponse;
        let resp = super::sync_resolve_handler(
            Path("conflict-123".to_string()),
            Json(SyncResolveRequest {
                resolution: "bogus".to_string(),
                chunk: None,
            }),
        )
        .await;
        let response = resp.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Verify the body contains the error message.
        let body = response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        let parsed: serde_json::Value = serde_json::from_slice(&body).expect("parse JSON");
        assert_eq!(parsed["status"], "error");
        assert!(parsed["message"]
            .as_str()
            .unwrap()
            .contains("resolution must be"));
    }

    #[test]
    fn test_validate_peer_url_schemes_and_hosts() {
        // Valid localhost & domains
        assert!(validate_peer_url("http://localhost:8080").is_ok());
        assert!(validate_peer_url("https://localhost").is_ok());
        assert!(validate_peer_url("http://127.0.0.1:8080").is_ok());
        assert!(validate_peer_url("https://127.0.0.1").is_ok());
        assert!(validate_peer_url("http://[::1]:8080").is_ok());
        assert!(validate_peer_url("https://peer.example.com").is_ok());

        // Invalid schemes
        assert!(validate_peer_url("ftp://localhost:8080").is_err());
        assert!(validate_peer_url("file:///etc/passwd").is_err());
        assert!(validate_peer_url("javascript:alert(1)").is_err());

        // Private / internal IP ranges rejected
        assert!(validate_peer_url("http://10.0.0.1:8080").is_err());
        assert!(validate_peer_url("http://192.168.1.10:8080").is_err());
        assert!(validate_peer_url("http://172.16.0.1").is_err());
        assert!(validate_peer_url("http://169.254.169.254/latest/meta-data").is_err());
        assert!(validate_peer_url("http://0.0.0.0").is_err());
    }

    #[test]
    fn test_validate_peer_url_allowlist() {
        std::env::set_var(
            "XAVIER_ALLOWED_SYNC_PEERS",
            "10.0.0.5,http://192.168.1.50:8080",
        );
        assert!(validate_peer_url("http://10.0.0.5:9000").is_ok());
        assert!(validate_peer_url("http://192.168.1.50:8080").is_ok());
        // Still rejects unlisted private IPs
        assert!(validate_peer_url("http://10.0.0.6:9000").is_err());
        std::env::remove_var("XAVIER_ALLOWED_SYNC_PEERS");
    }

    #[test]
    fn test_validate_workspace_id_traversal() {
        assert!(validate_workspace_id("default").is_ok());
        assert!(validate_workspace_id("workspace-123").is_ok());

        assert!(validate_workspace_id("../etc").is_err());
        assert!(validate_workspace_id("foo/bar").is_err());
        assert!(validate_workspace_id("foo\\bar").is_err());
        assert!(validate_workspace_id("..").is_err());
    }

    #[tokio::test]
    async fn push_handler_rejects_ssrf_and_traversal() {
        use axum::response::IntoResponse;

        // SSRF attempt
        let resp = super::sync_push_handler(Json(
            serde_json::from_value(serde_json::json!({
                "peer_url": "http://169.254.169.254/latest/meta-data"
            }))
            .unwrap(),
        ))
        .await
        .into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        // Path traversal attempt
        let resp = super::sync_push_handler(Json(
            serde_json::from_value(serde_json::json!({
                "peer_url": "http://localhost:8080",
                "workspace_id": "../evil"
            }))
            .unwrap(),
        ))
        .await
        .into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
