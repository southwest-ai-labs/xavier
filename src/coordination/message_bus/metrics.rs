use crate::coordination::message_bus::MessageBus;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Helper trait providing queue depth (pending message count) for bounded channels.
///
/// Tokio's `mpsc::Sender::capacity()` reports remaining available buffer capacity.
/// The actual queue length (number of pending messages) is computed as:
/// `max_capacity() - capacity()`.
pub trait QueueDepth {
    /// Returns the number of pending messages waiting in the queue.
    fn len(&self) -> usize;

    /// Returns true if there are no pending messages in the queue.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T> QueueDepth for tokio::sync::mpsc::Sender<T> {
    fn len(&self) -> usize {
        self.max_capacity().saturating_sub(self.capacity())
    }
}

/// Message Bus metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BusMetrics {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub messages_failed: u64,
    pub dlq_size: usize,
    pub registered_agents: usize,
    pub topic_subscriptions: HashMap<String, usize>,
    pub queue_sizes: HashMap<String, usize>,
    pub stale_agents: usize,
    pub heartbeats_received: u64,
}

impl MessageBus {
    /// Get bus metrics
    pub async fn get_metrics(&self) -> BusMetrics {
        let mut metrics = self.metrics.write().await;

        // Update queue sizes to reflect actual pending message queue depth
        let queues = self.queues.read().await;
        metrics.queue_sizes = queues.iter().map(|(k, v)| (k.clone(), v.len())).collect();

        metrics.clone()
    }
}
