//! Blue-Green Binary Slots & Atomic Symlink/Rename Swap
//!
//! Provides ETXTBSY-safe binary replacement, dual-slot layout,
//! and pre-flight canary sandbox validation.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Binary slots for Blue-Green execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinarySlot {
    SlotA,
    SlotB,
}

impl BinarySlot {
    pub fn other(&self) -> Self {
        match self {
            BinarySlot::SlotA => BinarySlot::SlotB,
            BinarySlot::SlotB => BinarySlot::SlotA,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            BinarySlot::SlotA => "slot_a",
            BinarySlot::SlotB => "slot_b",
        }
    }
}

/// Swapping strategy for the active binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwapMode {
    /// Active binary is a symlink pointing to Slot A or Slot B.
    /// Swap updates the symlink atomically via `symlink_tmp + rename`.
    Symlink,
    /// Active binary is replaced directly via atomic POSIX `rename(2)`.
    /// The running inode remains intact for active process, preventing ETXTBSY.
    AtomicRename,
}

/// Canary test execution report from pre-flight sandbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryReport {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

/// Slot layout and filesystem paths.
#[derive(Debug, Clone)]
pub struct SlotConfig {
    pub base_dir: PathBuf,
    pub active_bin: PathBuf,
    pub slot_a_dir: PathBuf,
    pub slot_b_dir: PathBuf,
    pub staging_dir: PathBuf,
    pub backup_file: PathBuf,
    pub binary_name: String,
    pub swap_mode: SwapMode,
}

/// Manages Blue-Green binary slots, staging downloads, and atomic swaps.
#[derive(Debug, Clone)]
pub struct SlotManager {
    config: SlotConfig,
}

impl SlotManager {
    /// Initialize SlotManager in a given root directory with binary name.
    pub fn new(base_dir: impl AsRef<Path>, binary_name: impl Into<String>) -> Result<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();
        let binary_name = binary_name.into();

        let active_bin = base_dir.join(&binary_name);
        let slot_a_dir = base_dir.join("slots").join("slot_a");
        let slot_b_dir = base_dir.join("slots").join("slot_b");
        let staging_dir = base_dir.join("staging");
        let backup_file = base_dir.join(format!("{}.bak", binary_name));

        let config = SlotConfig {
            base_dir,
            active_bin,
            slot_a_dir,
            slot_b_dir,
            staging_dir,
            backup_file,
            binary_name,
            swap_mode: SwapMode::Symlink,
        };

        let mgr = Self { config };
        mgr.ensure_dirs()?;
        Ok(mgr)
    }

    /// Set swap mode (Symlink or AtomicRename).
    pub fn with_swap_mode(mut self, mode: SwapMode) -> Self {
        self.config.swap_mode = mode;
        self
    }

    /// Access underlying configuration.
    pub fn config(&self) -> &SlotConfig {
        &self.config
    }

    /// Ensure required directory structure exists.
    pub fn ensure_dirs(&self) -> Result<()> {
        fs::create_dir_all(&self.config.slot_a_dir)?;
        fs::create_dir_all(&self.config.slot_b_dir)?;
        fs::create_dir_all(&self.config.staging_dir)?;
        Ok(())
    }

    /// Path to a specific slot's binary.
    pub fn slot_binary_path(&self, slot: BinarySlot) -> PathBuf {
        match slot {
            BinarySlot::SlotA => self.config.slot_a_dir.join(&self.config.binary_name),
            BinarySlot::SlotB => self.config.slot_b_dir.join(&self.config.binary_name),
        }
    }

    /// Path to the active binary or symlink.
    pub fn active_binary_path(&self) -> &Path {
        &self.config.active_bin
    }

    /// Path to backup binary.
    pub fn backup_binary_path(&self) -> &Path {
        &self.config.backup_file
    }

    /// Detect which slot is currently active.
    /// If symlink points to Slot B, returns Slot B; otherwise defaults to Slot A.
    pub fn detect_active_slot(&self) -> Result<BinarySlot> {
        if !self.config.active_bin.exists() {
            return Ok(BinarySlot::SlotA);
        }

        if let Ok(target) = fs::read_link(&self.config.active_bin) {
            let target_str = target.to_string_lossy();
            if target_str.contains("slot_b") {
                return Ok(BinarySlot::SlotB);
            }
        }

        Ok(BinarySlot::SlotA)
    }

    /// The inactive slot to receive the candidate binary.
    pub fn candidate_slot(&self) -> Result<BinarySlot> {
        Ok(self.detect_active_slot()?.other())
    }

    /// Stage a new binary into the staging directory.
    /// Verifies SHA-256 hash if provided and applies executable permissions (0755).
    pub fn stage_binary(&self, bytes: &[u8], expected_sha256: Option<&str>) -> Result<PathBuf> {
        if bytes.is_empty() {
            return Err(anyhow!("Cannot stage empty binary payload"));
        }

        // Verify SHA-256 checksum
        if let Some(expected) = expected_sha256 {
            let mut hasher = Sha256::new();
            hasher.update(bytes);
            let calculated: String = hasher
                .finalize()
                .into_iter()
                .map(|b| format!("{:02x}", b))
                .collect();
            if !calculated.eq_ignore_ascii_case(expected.trim()) {
                return Err(anyhow!(
                    "Binary SHA-256 mismatch: calculated {}, expected {}",
                    calculated,
                    expected
                ));
            }
        }

        let stage_filename = format!(
            "{}.staging.{}",
            self.config.binary_name,
            jiff::Timestamp::now().as_nanosecond() as i64
        );
        let stage_path = self.config.staging_dir.join(stage_filename);

        fs::write(&stage_path, bytes)
            .with_context(|| format!("Failed to write staged binary to {:?}", stage_path))?;

        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&stage_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&stage_path, perms)?;
        }

        debug!("Staged candidate binary at {:?}", stage_path);
        Ok(stage_path)
    }

    /// Pre-flight canary sandbox check:
    /// Executes the staged binary in a subprocess with canary arguments (e.g. `--version`).
    /// Ensures that:
    /// 1. File exists and is executable.
    /// 2. Subprocess boots without crashing, segfaulting, or missing libraries.
    /// 3. Returns exit code 0 within timeout.
    pub fn verify_binary_preflight(
        &self,
        binary_path: &Path,
        test_args: &[&str],
        timeout: Duration,
    ) -> Result<CanaryReport> {
        if !binary_path.exists() {
            return Err(anyhow!(
                "Candidate binary does not exist at {:?}",
                binary_path
            ));
        }

        #[cfg(unix)]
        {
            let meta = fs::metadata(binary_path)?;
            let mode = meta.permissions().mode();
            if mode & 0o111 == 0 {
                return Err(anyhow!(
                    "Candidate binary at {:?} lacks executable permissions (mode {:o})",
                    binary_path,
                    mode
                ));
            }
        }

        let start = Instant::now();

        // Spawn canary child process
        let mut cmd = std::process::Command::new(binary_path);
        cmd.args(test_args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().with_context(|| {
            format!(
                "Failed to spawn canary process for candidate {:?}",
                binary_path
            )
        })?;

        // Poll child with timeout to prevent hanging canaries
        let poll_interval = Duration::from_millis(50);
        let mut finished = false;

        while start.elapsed() < timeout {
            if child.try_wait()?.is_some() {
                finished = true;
                break;
            }
            std::thread::sleep(poll_interval);
        }

        if !finished {
            let _ = child.kill();
            return Err(anyhow!(
                "Canary execution timed out after {:?} for {:?}",
                timeout,
                binary_path
            ));
        }

        let output = child.wait_with_output()?;
        let duration_ms = start.elapsed().as_millis() as u64;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let code = output.status.code();

        let report = CanaryReport {
            success: output.status.success(),
            exit_code: code,
            stdout,
            stderr,
            duration_ms,
        };

        if !report.success {
            return Err(anyhow!(
                "Canary pre-flight check failed (code {:?}): stdout: '{}', stderr: '{}'",
                code,
                report.stdout.trim(),
                report.stderr.trim()
            ));
        }

        info!(
            "Canary pre-flight verification PASSED in {}ms for {:?}",
            duration_ms, binary_path
        );

        Ok(report)
    }

    /// Atomic swap to candidate slot.
    ///
    /// Protection against Linux `ETXTBSY`:
    /// Under UNIX/Linux, opening an actively running binary for writing (`O_WRONLY`)
    /// fails with error 26 (ETXTBSY: Text file busy).
    /// This method replaces the binary inode entry via atomic directory operations:
    /// - Symlink mode: `symlink(target, tmp_link)` + `rename(tmp_link, active_link)`
    /// - AtomicRename mode: `rename(staged, active_bin)`
    ///
    /// In both cases, POSIX semantics ensure the running process retains its open file
    /// descriptor/inode undisturbed, while any new executions invoke the new binary.
    pub fn atomic_swap(&self, staged_binary: &Path, candidate_slot: BinarySlot) -> Result<PathBuf> {
        let dest_slot_path = self.slot_binary_path(candidate_slot);

        // 1. Move/copy staged binary to candidate slot
        if dest_slot_path.exists() {
            let _ = fs::remove_file(&dest_slot_path);
        }
        fs::copy(staged_binary, &dest_slot_path).with_context(|| {
            format!(
                "Failed to copy staged binary to candidate slot {:?}",
                dest_slot_path
            )
        })?;

        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&dest_slot_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dest_slot_path, perms)?;
        }

        // Clean up staged binary
        let _ = fs::remove_file(staged_binary);

        match self.config.swap_mode {
            SwapMode::Symlink => {
                // Prepare .bak pointing to previous active target if exists
                if self.config.active_bin.exists() {
                    if let Ok(prev_target) = fs::read_link(&self.config.active_bin) {
                        let _ = fs::remove_file(&self.config.backup_file);
                        #[cfg(unix)]
                        let _ = std::os::unix::fs::symlink(&prev_target, &self.config.backup_file);
                    }
                }

                // Create temporary symlink pointing to new slot
                let tmp_link = self.config.base_dir.join(format!(
                    "{}.tmp.{}",
                    self.config.binary_name,
                    jiff::Timestamp::now().as_nanosecond() as i64
                ));

                #[cfg(unix)]
                {
                    std::os::unix::fs::symlink(&dest_slot_path, &tmp_link)
                        .with_context(|| format!("Failed to create symlink at {:?}", tmp_link))?;
                    // Atomic rename of symlink over active pointer
                    fs::rename(&tmp_link, &self.config.active_bin).with_context(|| {
                        format!(
                            "Failed to atomically swap symlink from {:?} to {:?}",
                            tmp_link, self.config.active_bin
                        )
                    })?;
                }

                #[cfg(not(unix))]
                {
                    // Fallback on non-unix: copy
                    let _ = fs::copy(&dest_slot_path, &self.config.active_bin);
                }
            }
            SwapMode::AtomicRename => {
                // If active binary exists, backup to .bak (read-only copy, no ETXTBSY)
                if self.config.active_bin.exists() {
                    let _ = fs::remove_file(&self.config.backup_file);
                    fs::copy(&self.config.active_bin, &self.config.backup_file)?;
                }

                // Atomically rename candidate binary over active binary
                // POSIX rename replaces dentry atomically without writing to active inode
                fs::rename(&dest_slot_path, &self.config.active_bin).with_context(|| {
                    format!(
                        "Failed to atomically rename candidate {:?} to {:?}",
                        dest_slot_path, self.config.active_bin
                    )
                })?;
            }
        }

        info!(
            "Atomic swap completed: active pointer now targets {:?} ({:?})",
            dest_slot_path, candidate_slot
        );

        Ok(dest_slot_path)
    }

    /// Revert active binary to backup slot or .bak file atomically.
    pub fn atomic_rollback(&self, target_slot: Option<BinarySlot>) -> Result<()> {
        match self.config.swap_mode {
            SwapMode::Symlink => {
                let target_path = if let Some(slot) = target_slot {
                    self.slot_binary_path(slot)
                } else if self.config.backup_file.exists() {
                    if let Ok(target) = fs::read_link(&self.config.backup_file) {
                        target
                    } else {
                        self.config.backup_file.clone()
                    }
                } else {
                    return Err(anyhow!("No valid rollback target or backup file available"));
                };

                let tmp_link = self.config.base_dir.join(format!(
                    "{}.rollback.tmp.{}",
                    self.config.binary_name,
                    jiff::Timestamp::now().as_nanosecond() as i64
                ));

                #[cfg(unix)]
                {
                    std::os::unix::fs::symlink(&target_path, &tmp_link)?;
                    fs::rename(&tmp_link, &self.config.active_bin)?;
                }

                #[cfg(not(unix))]
                {
                    let _ = fs::copy(&target_path, &self.config.active_bin);
                }
            }
            SwapMode::AtomicRename => {
                if !self.config.backup_file.exists() {
                    return Err(anyhow!(
                        "Backup file {:?} does not exist for rollback",
                        self.config.backup_file
                    ));
                }
                // Atomically rename backup over active binary
                fs::rename(&self.config.backup_file, &self.config.active_bin)?;
            }
        }

        warn!("Atomic rollback executed: reverted to previous binary/slot.");
        Ok(())
    }
}
