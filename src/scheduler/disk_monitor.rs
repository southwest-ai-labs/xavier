use std::time::{Duration, Instant};
use sysinfo::System;
use tracing::{error, info, warn};

use crate::observability::notifier::{publish, Notification, NotificationLevel};
use crate::settings::XavierSettings;

const DISK_RATE_LIMIT: Duration = Duration::from_secs(3600); // 1 hour

pub struct DiskMonitor {
    pub last_notified: Option<Instant>,
}

impl DiskMonitor {
    pub fn new() -> Self {
        Self {
            last_notified: None,
        }
    }

    pub fn check_and_notify(&mut self) {
        // Rate limiting logic
        if let Some(last) = self.last_notified {
            if last.elapsed() < DISK_RATE_LIMIT {
                return;
            }
        }

        let settings = XavierSettings::current();
        let warn_pct = settings.advanced.disk_warn_pct as f64;
        let crit_pct = settings.advanced.disk_crit_pct as f64;

        let disks = sysinfo::Disks::new_with_refreshed_list();
        let total_bytes: u64 = disks.iter().map(|d| d.total_space()).sum();
        let available_bytes: u64 = disks.iter().map(|d| d.available_space()).sum();

        let disk_usage_percent = if total_bytes > 0 {
            ((total_bytes - available_bytes) as f64 / total_bytes as f64) * 100.0
        } else {
            0.0
        };

        let (level, title) = if disk_usage_percent >= crit_pct {
            (NotificationLevel::Critical, "CRITICAL: Disk Usage High")
        } else if disk_usage_percent >= warn_pct {
            (NotificationLevel::Warning, "WARNING: Disk Usage High")
        } else {
            return;
        };

        let message = format!(
            "Disk usage is at {:.2}%. Threshold is {}%. Suggested cleanup: run `cargo clean` or `xavier clean-cache`.",
            disk_usage_percent,
            if disk_usage_percent >= crit_pct { crit_pct } else { warn_pct }
        );

        let notification = Notification {
            title: title.to_string(),
            message,
            level: level.clone(),
            metadata: Some(serde_json::json!({ "island": "system", "tags": ["disk", "health"] })),
        };

        if publish(&notification) == 0 {
            if level == NotificationLevel::Critical {
                error!("{}", notification.message);
            } else {
                warn!("{}", notification.message);
            }
        }

        self.last_notified = Some(Instant::now());
    }
}
