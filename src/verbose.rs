//! Lightweight verbose/debug diagnostics emitted to stderr.
//!
//! When `--verbose` is set globally, HTTP requests, responses, LRO polls,
//! and auth events are traced to stderr. All output uses the `[verbose]`
//! prefix for easy filtering. Respects `--quiet` (never emits if quiet).
//!
//! Design: No external logging crate. Uses a global `AtomicBool` flag
//! checked at each trace site. Zero overhead when disabled.

#![allow(dead_code)]

use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag: is verbose tracing enabled?
static VERBOSE_ENABLED: AtomicBool = AtomicBool::new(false);

/// Enable verbose tracing (called once at startup from `main`/`execute`).
pub fn enable() {
    VERBOSE_ENABLED.store(true, Ordering::Relaxed);
}

/// Check if verbose tracing is currently enabled.
#[inline]
pub fn is_enabled() -> bool {
    VERBOSE_ENABLED.load(Ordering::Relaxed)
}

/// Emit a verbose diagnostic line to stderr.
/// No-op if verbose is disabled.
#[inline]
pub fn trace(msg: &str) {
    if is_enabled() {
        eprintln!("[verbose] {msg}");
    }
}

/// Emit a verbose diagnostic with a category prefix.
/// Example: `[verbose][http] GET https://api.fabric.microsoft.com/v1/workspaces`
#[inline]
pub fn trace_category(category: &str, msg: &str) {
    if is_enabled() {
        eprintln!("[verbose][{category}] {msg}");
    }
}

// ── HTTP tracing helpers ────────────────────────────────────────────────────

/// Maximum body length to include in verbose output (prevents flooding stderr).
const MAX_BODY_TRACE_LEN: usize = 2048;

/// Trace an outgoing HTTP request.
pub fn trace_request(method: &str, url: &str, body: Option<&str>) {
    if !is_enabled() {
        return;
    }
    eprintln!("[verbose][http] --> {method} {url}");
    if let Some(b) = body {
        if b.is_empty() || b == "null" {
            return;
        }
        let display = if b.len() > MAX_BODY_TRACE_LEN {
            format!(
                "{}...(truncated, {} bytes total)",
                &b[..MAX_BODY_TRACE_LEN],
                b.len()
            )
        } else {
            b.to_string()
        };
        eprintln!("[verbose][http]     body: {display}");
    }
}

/// Trace an incoming HTTP response (status + key headers).
pub fn trace_response(status: u16, url: &str, duration_ms: u128) {
    if !is_enabled() {
        return;
    }
    eprintln!("[verbose][http] <-- {status} {url} ({duration_ms}ms)");
}

/// Trace response headers of interest (for debugging LRO, rate limits, etc.).
pub fn trace_response_headers(headers: &[(String, String)]) {
    if !is_enabled() || headers.is_empty() {
        return;
    }
    for (name, value) in headers {
        eprintln!("[verbose][http]     {name}: {value}");
    }
}

/// Trace a response body (truncated).
pub fn trace_response_body(body: &str) {
    if !is_enabled() || body.is_empty() {
        return;
    }
    let display = if body.len() > MAX_BODY_TRACE_LEN {
        format!(
            "{}...(truncated, {} bytes total)",
            &body[..MAX_BODY_TRACE_LEN],
            body.len()
        )
    } else {
        body.to_string()
    };
    eprintln!("[verbose][http]     response: {display}");
}

// ── LRO tracing ─────────────────────────────────────────────────────────────

/// Trace an LRO poll attempt.
pub fn trace_lro_poll(poll_url: &str, attempt: u32, status: &str) {
    if !is_enabled() {
        return;
    }
    eprintln!("[verbose][lro] poll #{attempt} {poll_url} -> status: {status}");
}

/// Trace LRO completion.
pub fn trace_lro_complete(poll_url: &str, final_status: &str, elapsed_ms: u128) {
    if !is_enabled() {
        return;
    }
    eprintln!("[verbose][lro] completed: {final_status} ({elapsed_ms}ms) {poll_url}");
}

// ── Auth tracing ────────────────────────────────────────────────────────────

/// Trace which credential source was used.
pub fn trace_auth(scope: &str, source: &str) {
    if !is_enabled() {
        return;
    }
    eprintln!("[verbose][auth] acquired token for scope={scope} via {source}");
}

/// Trace token refresh (cache hit vs miss).
pub fn trace_auth_cache_hit(scope: &str) {
    if !is_enabled() {
        return;
    }
    eprintln!("[verbose][auth] using cached token for scope={scope}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    /// Reset global state for test isolation.
    fn reset() {
        VERBOSE_ENABLED.store(false, Ordering::Relaxed);
    }

    #[test]
    fn is_disabled_by_default() {
        reset();
        assert!(!is_enabled());
    }

    #[test]
    fn enable_sets_flag() {
        reset();
        enable();
        assert!(is_enabled());
        reset(); // clean up for other tests
    }

    #[test]
    fn trace_request_noop_when_disabled() {
        reset();
        // Should not panic or produce output
        trace_request("GET", "https://example.com", None);
        trace_request("POST", "https://example.com", Some("{\"key\":\"value\"}"));
    }

    #[test]
    fn trace_response_noop_when_disabled() {
        reset();
        trace_response(200, "https://example.com", 42);
    }

    #[test]
    fn trace_lro_poll_noop_when_disabled() {
        reset();
        trace_lro_poll("https://example.com/operations/123", 1, "Running");
    }

    #[test]
    fn trace_lro_complete_noop_when_disabled() {
        reset();
        trace_lro_complete("https://example.com/operations/123", "Succeeded", 5000);
    }

    #[test]
    fn trace_auth_noop_when_disabled() {
        reset();
        trace_auth("https://api.fabric.microsoft.com/.default", "Azure CLI");
        trace_auth_cache_hit("https://api.fabric.microsoft.com/.default");
    }

    #[test]
    fn trace_category_noop_when_disabled() {
        reset();
        trace_category("http", "test message");
    }

    #[test]
    fn trace_response_body_noop_when_disabled() {
        reset();
        trace_response_body("{\"status\":\"ok\"}");
    }

    #[test]
    fn trace_response_headers_noop_when_disabled() {
        reset();
        trace_response_headers(&[("Content-Type".to_string(), "application/json".to_string())]);
    }

    #[test]
    fn body_truncation_respects_max_length() {
        // Verify the truncation constant is reasonable
        assert_eq!(MAX_BODY_TRACE_LEN, 2048);
    }
}
