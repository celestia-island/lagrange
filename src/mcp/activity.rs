//! Agent activity ring buffer — records MCP tool calls for the dev
//! console's agent-activity view.
//!
//! `lagrange dev --mcp --port N` exposes the buffer at
//! `GET /mcp-activity.json`; the webui console renders it. The buffer is
//! capped so a long-running session cannot grow without bound.

use jiff::Timestamp;
use serde::Serialize;
use std::sync::Mutex;

/// One recorded MCP tool call.
#[derive(Debug, Clone, Serialize)]
pub struct ActivityEntry {
    /// RFC 3339 UTC timestamp of the call.
    pub ts: String,
    /// Tool name (`docs.search`, `docs.write`, …).
    pub tool: String,
    /// Whether the call succeeded (no `isError` in the result).
    pub ok: bool,
    /// One-line summary of the arguments (query, document id, …).
    pub detail: String,
}

/// Capped, thread-safe activity log shared between the MCP stdio session
/// and the dev HTTP server.
pub struct ActivityLog {
    entries: Mutex<Vec<ActivityEntry>>,
    cap: usize,
}

impl ActivityLog {
    /// New log keeping the most recent `cap` entries.
    pub fn new(cap: usize) -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
            cap: cap.max(1),
        }
    }

    /// Record one tool call; evicts the oldest entry when over capacity.
    pub fn record(&self, tool: &str, ok: bool, detail: impl Into<String>) {
        let entry = ActivityEntry {
            ts: Timestamp::now().strftime("%Y-%m-%dT%H:%M:%SZ").to_string(),
            tool: tool.to_string(),
            ok,
            detail: detail.into(),
        };
        let mut guard = match self.entries.lock() {
            Ok(g) => g,
            // A poisoned lock means a recorder panicked mid-write; the log
            // is best-effort telemetry, so recover and continue.
            Err(p) => p.into_inner(),
        };
        if guard.len() >= self.cap {
            guard.remove(0);
        }
        guard.push(entry);
    }

    /// Newest-first snapshot for serialization.
    pub fn snapshot(&self) -> Vec<ActivityEntry> {
        let guard = match self.entries.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let mut v = guard.clone();
        v.reverse();
        v
    }
}

impl Default for ActivityLog {
    fn default() -> Self {
        Self::new(200)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_evicts_oldest() {
        let log = ActivityLog::new(2);
        log.record("docs.search", true, "q=quick start");
        log.record("docs.fetch", true, "doc=architecture");
        log.record("docs.write", false, "denied");
        let snap = log.snapshot();
        assert_eq!(snap.len(), 2);
        // newest first
        assert_eq!(snap[0].tool, "docs.write");
        assert!(!snap[0].ok);
        assert_eq!(snap[1].tool, "docs.fetch");
    }

    #[test]
    fn snapshot_is_newest_first() {
        let log = ActivityLog::new(10);
        log.record("a", true, "");
        log.record("b", true, "");
        let snap = log.snapshot();
        assert_eq!(snap[0].tool, "b");
        assert_eq!(snap[1].tool, "a");
    }
}
