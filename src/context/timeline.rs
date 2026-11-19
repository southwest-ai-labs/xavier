//! Timeline Engine (Time Travel)
//!
//! Provides the ability for Xavier to navigate and query contexts specifically
//! by dates, time ranges, and timeline events. This acts as the "Cognitive Calendar",
//! allowing the LLM to understand the lifecycle of tasks and rebuild the state
//! of the context at a given point in time.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::memory::qmd_memory::QmdMemory;
use crate::memory::schema::MemoryQueryFilters;
use crate::memory::virtual_memory::{MemoryReference, VirtualMemory};

/// A request to navigate the cognitive calendar
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineQuery {
    pub query: String,
    pub start_date: Option<jiff::Timestamp>,
    pub end_date: Option<jiff::Timestamp>,
    pub agent_id: Option<String>,
    pub limit: usize,
}

/// A slice of context from a specific point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSlice {
    pub period_start: jiff::Timestamp,
    pub period_end: jiff::Timestamp,
    pub memories: Vec<MemoryReference>,
    pub timeline_events: Vec<TimelineEventSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEventSummary {
    pub timestamp: jiff::Timestamp,
    pub operation: String,
    pub summary: String,
    pub agent_id: String,
}

pub struct TimelineEngine {
    memory: Arc<QmdMemory>,
}

impl TimelineEngine {
    /// New.
    pub fn new(memory: Arc<QmdMemory>) -> Self {
        Self { memory }
    }

    /// Retrieve a slice of context restricted to a specific time range.
    /// This simulates "time travel" by only looking at memories created
    /// or modified within that window, alongside the raw audit events.
    pub async fn get_time_slice(&self, query: &TimelineQuery) -> Result<TimeSlice> {
        let start = query.start_date.unwrap_or_else(|| {
            jiff::Timestamp::now()
                .checked_sub(jiff::Span::new().hours(30 * 24))
                .unwrap()
        });
        let end = query.end_date.unwrap_or_else(jiff::Timestamp::now);

        // 1. Fetch memory references
        let filters = MemoryQueryFilters {
            recorded_after: Some(start.to_string()),
            recorded_before: Some(end.to_string()),
            ..Default::default()
        };

        let vm = VirtualMemory::new(Arc::clone(&self.memory), None);
        let entries = vm
            .page_in_filtered(&query.query, query.limit, Some(&filters))
            .await?;
        let memories: Vec<MemoryReference> =
            entries.into_iter().map(|e| e.to_reference()).collect();

        // 2. Fetch timeline audit events (if supported by backend)
        let since_iso = start.to_string();
        let mut timeline_events = Vec::new();
        let ws = self.memory.workspace_id();

        if let Some(store) = self.memory.store().await {
            if let Ok(events) = store.list_timeline_events(ws, &since_iso).await {
                for ev in events {
                    if let Ok(ts) = ev.timestamp.parse::<jiff::Timestamp>() {
                        if ts <= end {
                            timeline_events.push(TimelineEventSummary {
                                timestamp: ts,
                                operation: ev.event_type.clone(),
                                summary: ev
                                    .payload
                                    .get("summary")
                                    .and_then(|s| s.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                agent_id: ev.agent_id.clone(),
                            });
                        }
                    }
                }
            }
        }

        // Sort events chronologically
        timeline_events.sort_by_key(|a| a.timestamp);
        timeline_events.truncate(query.limit);

        Ok(TimeSlice {
            period_start: start,
            period_end: end,
            memories,
            timeline_events,
        })
    }
}
