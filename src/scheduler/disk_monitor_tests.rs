use super::disk_monitor::DiskMonitor;
use crate::settings::AdvancedSettings;
use crate::settings::XavierSettings;
use std::time::{Duration, Instant};

#[test]
fn test_disk_monitor_creation() {
    let monitor = DiskMonitor::new();
    assert!(monitor.last_notified.is_none());
}

#[test]
fn test_disk_monitor_rate_limit() {
    let mut monitor = DiskMonitor::new();
    monitor.last_notified = Some(Instant::now());

    // Within rate limit, should not update last_notified
    let before = monitor.last_notified;
    monitor.check_and_notify();
    assert_eq!(before, monitor.last_notified);
}
