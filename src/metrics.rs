//! Global API metrics tracking for performance diagnostics.
//!
//! Records aggregate timing for all Fabric API calls. Uses atomic counters
//! with zero overhead when not read. Metrics are included in every JSON
//! output envelope as `_timing` to help agents and users understand where
//! time is spent (Fabric API vs local processing).
//!
//! Design: Global statics (like `verbose.rs`) to avoid passing metrics
//! through every function signature. The client records into globals,
//! the output layer reads from them.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

// ── Global state ────────────────────────────────────────────────────────────

/// Total number of HTTP requests sent to Fabric/OneLake/ARM APIs.
static API_CALLS: AtomicU64 = AtomicU64::new(0);

/// Total milliseconds spent waiting for API responses (includes LRO polling).
static API_DURATION_MS: AtomicU64 = AtomicU64::new(0);

/// Command start time (set once at startup).
static CMD_START: OnceLock<Instant> = OnceLock::new();

// ── Public API ──────────────────────────────────────────────────────────────

/// Record the command start time. Called once at CLI startup.
pub fn init() {
    CMD_START.get_or_init(Instant::now);
}

/// Record a completed API call with its duration.
#[inline]
pub fn record_api_call(duration: Duration) {
    API_CALLS.fetch_add(1, Ordering::Relaxed);
    let ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
    API_DURATION_MS.fetch_add(ms, Ordering::Relaxed);
}

/// Get current metrics snapshot as a JSON value (used in tests).
#[cfg(test)]
pub fn timing_json() -> serde_json::Value {
    use serde_json::json;
    let total_ms = CMD_START.get().map_or(0, |s| {
        u64::try_from(s.elapsed().as_millis()).unwrap_or(u64::MAX)
    });
    let api_ms = API_DURATION_MS.load(Ordering::Relaxed);
    let api_calls = API_CALLS.load(Ordering::Relaxed);

    json!({
        "total_ms": total_ms,
        "api_ms": api_ms,
        "api_calls": api_calls
    })
}

/// Emit a one-line timing summary to stderr.
///
/// Format: `[timing] 5 calls, 1234ms API, 1500ms total`
/// Only emits if at least one API call was made (avoids noise for --help, --version, etc.)
pub fn emit_timing_summary() {
    let api_calls = API_CALLS.load(Ordering::Relaxed);
    if api_calls == 0 {
        return;
    }
    let api_ms = API_DURATION_MS.load(Ordering::Relaxed);
    let total_ms = CMD_START.get().map_or(0, |s| {
        u64::try_from(s.elapsed().as_millis()).unwrap_or(u64::MAX)
    });
    eprintln!("[timing] {api_calls} calls, {api_ms}ms API, {total_ms}ms total");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_read_metrics() {
        // Note: these tests share global state, so values accumulate.
        // We only test the recording mechanism, not absolute values.
        let before_calls = API_CALLS.load(Ordering::Relaxed);
        record_api_call(Duration::from_millis(100));
        record_api_call(Duration::from_millis(200));
        let after_calls = API_CALLS.load(Ordering::Relaxed);
        assert_eq!(after_calls - before_calls, 2);
    }

    #[test]
    fn timing_json_has_expected_fields() {
        init();
        let timing = timing_json();
        assert!(timing.get("total_ms").is_some());
        assert!(timing.get("api_ms").is_some());
        assert!(timing.get("api_calls").is_some());
    }
}
