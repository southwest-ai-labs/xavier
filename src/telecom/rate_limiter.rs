//! High-performance in-memory sliding-window rate limiter per node/wallet (#2397 / WAVE-27.19)

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Rate limit decision outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitDecision {
    pub allowed: bool,
    pub remaining_requests: usize,
    pub reset_after_ms: u64,
}

/// Sliding window rate limiter tracking message bursts per peer/wallet identity.
#[derive(Clone)]
pub struct TelecomRateLimiter {
    max_requests_per_window: usize,
    window_duration: Duration,
    max_capacity: usize,
    records: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
}

impl TelecomRateLimiter {
    /// Default maximum capacity of tracked node/wallet IDs before pruning kicks in.
    pub const DEFAULT_MAX_CAPACITY: usize = 10_000;

    /// Creates a new rate limiter with specified quota per time window and default capacity.
    pub fn new(max_requests_per_window: usize, window_duration: Duration) -> Self {
        Self::with_capacity(
            max_requests_per_window,
            window_duration,
            Self::DEFAULT_MAX_CAPACITY,
        )
    }

    /// Creates a new rate limiter with specified quota per time window and maximum key capacity.
    pub fn with_capacity(
        max_requests_per_window: usize,
        window_duration: Duration,
        max_capacity: usize,
    ) -> Self {
        Self {
            max_requests_per_window,
            window_duration,
            max_capacity: max_capacity.max(1),
            records: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Checks if a request from `node_or_wallet_id` is within quota and records it if allowed.
    pub async fn check_rate_limit(&self, node_or_wallet_id: &str) -> RateLimitDecision {
        let mut map = self.records.lock().await;
        let now = Instant::now();
        let cutoff = now.checked_sub(self.window_duration).unwrap_or(now);

        // Enforce maximum capacity bounds when inserting a new key or when capacity exceeded.
        if map.len() >= self.max_capacity {
            // 1. Evict expired entries and empty vectors across all keys.
            map.retain(|k, timestamps| {
                if k == node_or_wallet_id {
                    return true;
                }
                timestamps.retain(|&t| t > cutoff);
                !timestamps.is_empty()
            });

            // 2. If still at or over capacity and key is new, evict the oldest/most idle entries.
            while map.len() >= self.max_capacity && !map.contains_key(node_or_wallet_id) {
                if let Some(oldest_key) = map
                    .iter()
                    .filter(|(k, _)| k.as_str() != node_or_wallet_id)
                    .min_by_key(|(_, ts)| ts.last().copied().unwrap_or(now))
                    .map(|(k, _)| k.clone())
                {
                    map.remove(&oldest_key);
                } else {
                    break;
                }
            }
        }

        let (allowed, remaining_requests, reset_after_ms) =
            if let Some(timestamps) = map.get_mut(node_or_wallet_id) {
                timestamps.retain(|&t| t > cutoff);
                if timestamps.len() < self.max_requests_per_window {
                    timestamps.push(now);
                    (
                        true,
                        self.max_requests_per_window - timestamps.len(),
                        self.window_duration.as_millis() as u64,
                    )
                } else {
                    let earliest = timestamps.first().copied().unwrap_or(now);
                    let elapsed = now.duration_since(earliest);
                    let remaining = self.window_duration.saturating_sub(elapsed);
                    (false, 0, remaining.as_millis() as u64)
                }
            } else if self.max_requests_per_window > 0 {
                map.insert(node_or_wallet_id.to_string(), vec![now]);
                (
                    true,
                    self.max_requests_per_window - 1,
                    self.window_duration.as_millis() as u64,
                )
            } else {
                (false, 0, self.window_duration.as_millis() as u64)
            };

        // Clean up empty key entry if timestamps emptied and no new timestamp was pushed.
        if !allowed {
            if let Some(ts) = map.get(node_or_wallet_id) {
                if ts.is_empty() {
                    map.remove(node_or_wallet_id);
                }
            }
        }

        RateLimitDecision {
            allowed,
            remaining_requests,
            reset_after_ms,
        }
    }

    /// Evicts idle keys whose timestamps are all expired past the sliding window.
    pub async fn evict_idle(&self) {
        let mut map = self.records.lock().await;
        let now = Instant::now();
        let cutoff = now.checked_sub(self.window_duration).unwrap_or(now);
        map.retain(|_, timestamps| {
            timestamps.retain(|&t| t > cutoff);
            !timestamps.is_empty()
        });
    }

    /// Returns the number of tracked keys in memory.
    pub async fn len(&self) -> usize {
        self.records.lock().await.len()
    }

    /// Returns `true` if no keys are currently tracked.
    pub async fn is_empty(&self) -> bool {
        self.records.lock().await.is_empty()
    }

    /// Checks if a specific key is currently tracked.
    pub async fn contains_key(&self, key: &str) -> bool {
        self.records.lock().await.contains_key(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_allows_and_throttles() {
        let limiter = TelecomRateLimiter::new(3, Duration::from_millis(100));

        let res1 = limiter.check_rate_limit("wallet_1").await;
        assert!(res1.allowed);
        assert_eq!(res1.remaining_requests, 2);

        let res2 = limiter.check_rate_limit("wallet_1").await;
        assert!(res2.allowed);

        let res3 = limiter.check_rate_limit("wallet_1").await;
        assert!(res3.allowed);

        // 4th request must be rejected
        let res4 = limiter.check_rate_limit("wallet_1").await;
        assert!(!res4.allowed);
        assert_eq!(res4.remaining_requests, 0);

        // Different wallet is independent
        let res_other = limiter.check_rate_limit("wallet_2").await;
        assert!(res_other.allowed);

        // Wait for sliding window to expire
        tokio::time::sleep(Duration::from_millis(120)).await;
        let res_after = limiter.check_rate_limit("wallet_1").await;
        assert!(res_after.allowed);
    }

    #[tokio::test]
    async fn test_rate_limiter_evicts_idle_keys() {
        let limiter = TelecomRateLimiter::new(3, Duration::from_millis(100));

        limiter.check_rate_limit("wallet_idle_1").await;
        limiter.check_rate_limit("wallet_idle_2").await;
        assert_eq!(limiter.len().await, 2);
        assert!(limiter.contains_key("wallet_idle_1").await);
        assert!(limiter.contains_key("wallet_idle_2").await);

        // Wait for sliding window to expire
        tokio::time::sleep(Duration::from_millis(120)).await;

        // Evict idle entries
        limiter.evict_idle().await;

        // Map must be completely empty now
        assert_eq!(limiter.len().await, 0);
        assert!(limiter.is_empty().await);
        assert!(!limiter.contains_key("wallet_idle_1").await);
        assert!(!limiter.contains_key("wallet_idle_2").await);
    }

    #[tokio::test]
    async fn test_rate_limiter_capacity_bounded_prunes_idle_and_oldest() {
        // Max capacity of 2 keys
        let limiter = TelecomRateLimiter::with_capacity(3, Duration::from_millis(100), 2);

        limiter.check_rate_limit("wallet_a").await;
        tokio::time::sleep(Duration::from_millis(10)).await;
        limiter.check_rate_limit("wallet_b").await;
        assert_eq!(limiter.len().await, 2);

        // Adding wallet_c while wallet_a and wallet_b are still active should evict oldest (wallet_a)
        limiter.check_rate_limit("wallet_c").await;
        assert_eq!(limiter.len().await, 2);
        assert!(!limiter.contains_key("wallet_a").await);
        assert!(limiter.contains_key("wallet_b").await);
        assert!(limiter.contains_key("wallet_c").await);

        // Now let wallet_b and wallet_c expire
        tokio::time::sleep(Duration::from_millis(120)).await;

        // Adding wallet_d should automatically clean expired idle keys
        limiter.check_rate_limit("wallet_d").await;
        assert_eq!(limiter.len().await, 1);
        assert!(limiter.contains_key("wallet_d").await);
        assert!(!limiter.contains_key("wallet_b").await);
        assert!(!limiter.contains_key("wallet_c").await);
    }

    #[tokio::test]
    async fn test_rate_limiter_zero_quota_does_not_leak_empty_keys() {
        let limiter = TelecomRateLimiter::new(0, Duration::from_millis(100));

        let res = limiter.check_rate_limit("wallet_blocked").await;
        assert!(!res.allowed);
        assert_eq!(limiter.len().await, 0);
        assert!(!limiter.contains_key("wallet_blocked").await);
    }
}
