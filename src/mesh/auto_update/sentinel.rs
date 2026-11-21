//! Rollback Sentinel & Failure Cycle Watchdog
//!
//! Enforces a 30-second Health Gate after any binary swap, executes autonomous
//! atomic rollback to .bak or the previous slot if /health fails, and trips
//! a quarantine circuit breaker to prevent endless crash loops.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};

use super::slots::{BinarySlot, SlotManager};

/// Current lifecycle status of the Sentinel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SentinelStatus {
    /// Normal operating state; no update under verification.
    Idle,
    /// Update applied, candidate is inside the 30-second Health Gate window.
    InProbation,
    /// Successfully validated 200 OK on /health; update finalized.
    Committed,
    /// Health check failed or timed out; autonomously reverted to backup.
    RolledBack,
    /// Consecutive failures exceeded threshold; updates frozen in cooldown.
    Quarantined,
}

/// Persistent sentinel state on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelState {
    pub status: SentinelStatus,
    pub candidate_sha: String,
    pub previous_sha: String,
    pub active_slot: String,
    pub backup_slot: String,
    pub started_at_secs: u64,
    pub deadline_secs: u64,
    pub consecutive_failures: u32,
    pub quarantine_until_secs: Option<u64>,
    pub last_error: Option<String>,
}

impl Default for SentinelState {
    fn default() -> Self {
        Self {
            status: SentinelStatus::Idle,
            candidate_sha: String::new(),
            previous_sha: String::new(),
            active_slot: "slot_a".to_string(),
            backup_slot: "slot_b".to_string(),
            started_at_secs: 0,
            deadline_secs: 0,
            consecutive_failures: 0,
            quarantine_until_secs: None,
            last_error: None,
        }
    }
}

/// Configuration options for Rollback Sentinel.
#[derive(Debug, Clone)]
pub struct SentinelConfig {
    pub state_file: PathBuf,
    pub health_gate_duration: Duration,
    pub poll_interval: Duration,
    pub max_consecutive_failures: u32,
    pub quarantine_duration: Duration,
}

impl Default for SentinelConfig {
    fn default() -> Self {
        let state_dir = std::env::var("XAVIER_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(".xavier"));

        Self {
            state_file: state_dir.join("update_sentinel.json"),
            health_gate_duration: Duration::from_secs(30),
            poll_interval: Duration::from_secs(1),
            max_consecutive_failures: 3,
            quarantine_duration: Duration::from_secs(7200), // 2 hours cooldown
        }
    }
}

/// Result of evaluating the health gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthGateDecision {
    /// Passed health check; update committed.
    Committed { sha: String },
    /// Failed health check or timeout; rolled back autonomously.
    RolledBack {
        sha: String,
        target_slot: String,
        reason: String,
    },
    /// Circuit breaker tripped due to consecutive failure loop.
    Quarantined {
        consecutive_failures: u32,
        until_secs: u64,
        reason: String,
    },
    /// No probation active.
    Skipped,
}

/// Autonomous Sentinel supervising health gating and rollback.
#[derive(Debug, Clone)]
pub struct RollbackSentinel {
    config: SentinelConfig,
}

impl RollbackSentinel {
    /// Create new sentinel with default or environment-configured settings.
    pub fn new() -> Self {
        Self::with_config(SentinelConfig::default())
    }

    /// Construct sentinel with custom configuration.
    pub fn with_config(config: SentinelConfig) -> Self {
        Self { config }
    }

    /// Helper to get current epoch seconds.
    fn current_epoch_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Load sentinel state from disk or return default.
    pub fn load_state(&self) -> SentinelState {
        if self.config.state_file.exists() {
            if let Ok(content) = fs::read_to_string(&self.config.state_file) {
                if let Ok(state) = serde_json::from_str::<SentinelState>(&content) {
                    return state;
                }
            }
        }
        SentinelState::default()
    }

    /// Persist state to disk safely.
    pub fn save_state(&self, state: &SentinelState) -> Result<()> {
        if let Some(parent) = self.config.state_file.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(state)?;
        let tmp = self.config.state_file.with_extension("tmp");
        fs::write(&tmp, json)?;
        fs::rename(&tmp, &self.config.state_file)?;
        Ok(())
    }

    /// Check if auto-update is currently in quarantine cooldown.
    pub fn is_quarantined(&self) -> bool {
        let mut state = self.load_state();
        if state.status == SentinelStatus::Quarantined {
            if let Some(until) = state.quarantine_until_secs {
                let now = Self::current_epoch_secs();
                if now < until {
                    return true;
                } else {
                    // Quarantine expired, reset status to Idle
                    state.status = SentinelStatus::Idle;
                    state.quarantine_until_secs = None;
                    let _ = self.save_state(&state);
                    return false;
                }
            }
        }
        false
    }

    /// Start 30-second probation gate for a swapped candidate binary.
    pub fn start_probation(
        &self,
        candidate_sha: &str,
        previous_sha: &str,
        active_slot: &str,
        backup_slot: &str,
    ) -> Result<()> {
        let now = Self::current_epoch_secs();
        let deadline = now + self.config.health_gate_duration.as_secs();

        let mut state = self.load_state();
        state.status = SentinelStatus::InProbation;
        state.candidate_sha = candidate_sha.to_string();
        state.previous_sha = previous_sha.to_string();
        state.active_slot = active_slot.to_string();
        state.backup_slot = backup_slot.to_string();
        state.started_at_secs = now;
        state.deadline_secs = deadline;
        state.last_error = None;

        self.save_state(&state)?;

        info!(
            "Sentinel: Armed 30s probation Health Gate for candidate {} (deadline in {}s)",
            candidate_sha,
            self.config.health_gate_duration.as_secs()
        );
        Ok(())
    }

    /// Commit the current update after health verification passes.
    pub fn commit_update(&self) -> Result<()> {
        let mut state = self.load_state();
        state.status = SentinelStatus::Committed;
        state.consecutive_failures = 0;
        state.last_error = None;
        self.save_state(&state)?;

        info!(
            "Sentinel: Health Gate PASSED. Update {} successfully committed.",
            state.candidate_sha
        );
        Ok(())
    }

    /// Execute autonomous rollback when candidate fails health check.
    pub fn trigger_rollback(
        &self,
        slot_mgr: &SlotManager,
        reason: &str,
    ) -> Result<HealthGateDecision> {
        let mut state = self.load_state();
        warn!(
            "Sentinel: Triggering autonomous rollback for candidate {}! Reason: {}",
            state.candidate_sha, reason
        );

        // Determine backup slot enum if known
        let target_slot = if state.backup_slot.contains("slot_a") {
            Some(BinarySlot::SlotA)
        } else if state.backup_slot.contains("slot_b") {
            Some(BinarySlot::SlotB)
        } else {
            None
        };

        // Execute atomic rollback via slot manager
        slot_mgr
            .atomic_rollback(target_slot)
            .context("SlotManager failed to execute atomic rollback")?;

        state.consecutive_failures += 1;
        state.last_error = Some(reason.to_string());

        let decision = if state.consecutive_failures >= self.config.max_consecutive_failures {
            let now = Self::current_epoch_secs();
            let quarantine_until = now + self.config.quarantine_duration.as_secs();
            state.status = SentinelStatus::Quarantined;
            state.quarantine_until_secs = Some(quarantine_until);

            error!(
                "Sentinel: Consecutive failures ({}) reached threshold. Auto-update QUARANTINED until epoch {}.",
                state.consecutive_failures, quarantine_until
            );

            HealthGateDecision::Quarantined {
                consecutive_failures: state.consecutive_failures,
                until_secs: quarantine_until,
                reason: reason.to_string(),
            }
        } else {
            state.status = SentinelStatus::RolledBack;
            HealthGateDecision::RolledBack {
                sha: state.candidate_sha.clone(),
                target_slot: state.backup_slot.clone(),
                reason: reason.to_string(),
            }
        };

        self.save_state(&state)?;
        Ok(decision)
    }

    /// Supervise the 30-second Health Gate:
    /// Asynchronously polls `health_probe` until HTTP 200 OK is received or deadline expires.
    /// If probe fails or times out, executes autonomous rollback immediately without human intervention.
    pub async fn supervise_health_gate<F, Fut>(
        &self,
        slot_mgr: &SlotManager,
        health_probe: F,
    ) -> Result<HealthGateDecision>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<bool>>,
    {
        let state = self.load_state();
        if state.status != SentinelStatus::InProbation {
            return Ok(HealthGateDecision::Skipped);
        }

        let deadline = tokio::time::Instant::now() + self.config.health_gate_duration;
        let poll_interval = self.config.poll_interval;

        info!(
            "Sentinel: Monitoring Health Gate (timeout {:?}) for candidate {}...",
            self.config.health_gate_duration, state.candidate_sha
        );

        loop {
            // Check if deadline expired
            if tokio::time::Instant::now() >= deadline {
                let reason = format!(
                    "Health Gate timed out after {:?}",
                    self.config.health_gate_duration
                );
                return self.trigger_rollback(slot_mgr, &reason);
            }

            // Probe /health
            match health_probe().await {
                Ok(true) => {
                    self.commit_update()?;
                    return Ok(HealthGateDecision::Committed {
                        sha: state.candidate_sha,
                    });
                }
                Ok(false) => {
                    warn!("Sentinel: /health responded non-200, retrying...");
                }
                Err(e) => {
                    warn!(
                        "Sentinel: Health probe connection error: {}, retrying...",
                        e
                    );
                }
            }

            tokio::time::sleep(poll_interval).await;
        }
    }
}
