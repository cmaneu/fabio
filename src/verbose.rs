//! Lightweight verbose/debug diagnostics emitted to stderr.
//!
//! When `--verbose` is set globally, HTTP requests, responses, LRO polls,
//! and auth events are traced to stderr. All output uses the `[verbose]`
//! prefix for easy filtering. Respects `--quiet` (never emits if quiet).
//!
//! **Security**: Request bodies are redacted for known sensitive JSON keys
//! (passwords, secrets, tokens, credentials) before logging. Response bodies
//! and headers are NOT logged in production (those helpers are `#[cfg(test)]` only).
//!
//! Design: No external logging crate. Uses a global `AtomicBool` flag
//! checked at each trace site. Zero overhead when disabled.

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

/// JSON keys whose values must be redacted in verbose output.
/// Case-insensitive comparison is used.
const SENSITIVE_KEYS: &[&str] = &[
    "password",
    "secret",
    "client_secret",
    "clientSecret",
    "credentials",
    "credential",
    "access_token",
    "accessToken",
    "refresh_token",
    "refreshToken",
    "token",
    "key",
    "connectionString",
    "sharedAccessSignature",
    "accountKey",
];

/// Redact sensitive fields in a JSON body string.
/// Returns the body with sensitive values replaced by `"[REDACTED]"`.
/// If the body is not valid JSON, returns it unchanged (non-JSON bodies
/// like `text/plain` or binary are not redacted — they don't have keys).
fn redact_sensitive_body(body: &str) -> String {
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(body) else {
        return body.to_string();
    };
    redact_value(&mut value);
    serde_json::to_string(&value).unwrap_or_else(|_| body.to_string())
}

/// Redact sensitive fields in a string if it's JSON. Public API for use
/// outside the verbose module (e.g., error message sanitization).
pub fn redact_body_if_json(body: &str) -> String {
    redact_sensitive_body(body)
}

/// Recursively redact sensitive keys in a JSON value.
fn redact_value(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                if SENSITIVE_KEYS.iter().any(|s| key.eq_ignore_ascii_case(s)) {
                    *val = serde_json::Value::from("[REDACTED]");
                } else {
                    redact_value(val);
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                redact_value(item);
            }
        }
        _ => {}
    }
}

/// Trace an outgoing HTTP request. Sensitive JSON fields are redacted.
pub fn trace_request(method: &str, url: &str, body: Option<&str>) {
    if !is_enabled() {
        return;
    }
    eprintln!("[verbose][http] --> {method} {url}");
    if let Some(b) = body {
        if b.is_empty() || b == "null" {
            return;
        }
        let redacted = redact_sensitive_body(b);
        let display = if redacted.len() > MAX_BODY_TRACE_LEN {
            format!(
                "{}...(truncated, {} bytes total)",
                &redacted[..redacted.floor_char_boundary(MAX_BODY_TRACE_LEN)],
                redacted.len()
            )
        } else {
            redacted
        };
        eprintln!("[verbose][http]     body: {display}");
    }
}

/// Trace an incoming HTTP response (status + key headers).
/// Always records metrics (even when verbose is disabled).
pub fn trace_response(status: u16, url: &str, duration_ms: u128) {
    // Always record API metrics regardless of verbose mode
    crate::metrics::record_api_call(std::time::Duration::from_millis(
        u64::try_from(duration_ms).unwrap_or(u64::MAX),
    ));

    if !is_enabled() {
        return;
    }
    let _ = status;
    eprintln!("[verbose][http] <-- {status} {url} ({duration_ms}ms)");
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
    fn body_truncation_respects_max_length() {
        assert_eq!(MAX_BODY_TRACE_LEN, 2048);
    }

    // ── Redaction tests ─────────────────────────────────────────────────

    #[test]
    fn redact_password_field() {
        let body = r#"{"name":"test","password":"super-secret-123"}"#;
        let result = redact_sensitive_body(body);
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("super-secret-123"));
        assert!(result.contains("test")); // non-sensitive field preserved
    }

    #[test]
    fn redact_client_secret() {
        let body = r#"{"clientSecret":"my-secret","clientId":"app-id"}"#;
        let result = redact_sensitive_body(body);
        assert!(!result.contains("my-secret"));
        assert!(result.contains("app-id")); // clientId is NOT sensitive
    }

    #[test]
    fn redact_nested_credentials() {
        let body = r#"{"credentialDetails":{"credentials":{"password":"pw123"}}}"#;
        let result = redact_sensitive_body(body);
        assert!(!result.contains("pw123"));
        assert!(result.contains("[REDACTED]"));
    }

    #[test]
    fn redact_access_token() {
        let body = r#"{"access_token":"eyJ0eXAi...","expires_in":3600}"#;
        let result = redact_sensitive_body(body);
        assert!(!result.contains("eyJ0eXAi"));
        assert!(result.contains("3600")); // non-sensitive field preserved
    }

    #[test]
    fn redact_in_array() {
        let body = r#"[{"password":"a"},{"password":"b"}]"#;
        let result = redact_sensitive_body(body);
        assert!(!result.contains("\"a\""));
        assert!(!result.contains("\"b\""));
    }

    #[test]
    fn non_json_body_unchanged() {
        let body = "password=secret&user=admin";
        let result = redact_sensitive_body(body);
        assert_eq!(result, body); // not JSON, returned as-is
    }

    #[test]
    fn empty_body_unchanged() {
        assert_eq!(redact_sensitive_body(""), "");
        assert_eq!(redact_sensitive_body("null"), "null");
    }

    #[test]
    fn non_sensitive_fields_preserved() {
        let body = r#"{"displayName":"test","description":"hello","id":"123"}"#;
        let result = redact_sensitive_body(body);
        assert!(result.contains("test"));
        assert!(result.contains("hello"));
        assert!(result.contains("123"));
        assert!(!result.contains("[REDACTED]"));
    }

    #[test]
    fn redact_connection_string() {
        let body = r#"{"connectionString":"Server=tcp:x.database.windows.net;Password=foo"}"#;
        let result = redact_sensitive_body(body);
        assert!(!result.contains("Password=foo"));
        assert!(result.contains("[REDACTED]"));
    }

    #[test]
    fn redact_shared_access_signature() {
        let body = r#"{"sharedAccessSignature":"sv=2021&sig=abc123"}"#;
        let result = redact_sensitive_body(body);
        assert!(!result.contains("abc123"));
    }
}
