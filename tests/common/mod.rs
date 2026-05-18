#![allow(dead_code)]
//! Shared test harness for fabio end-to-end integration tests.
//!
//! These tests exercise the compiled binary against a live Microsoft Fabric tenant.
//! They require valid Azure credentials (e.g., `az login`) and the following
//! environment variables to be set:
//!
//! - `FABIO_TEST_SOURCE_WORKSPACE` - Source workspace ID
//! - `FABIO_TEST_SOURCE_LAKEHOUSE` - Source lakehouse ID
//! - `FABIO_TEST_DEST_WORKSPACE` - Destination workspace ID
//! - `FABIO_TEST_DEST_LAKEHOUSE` - Destination lakehouse ID
//! - `FABIO_TEST_NOTEBOOK_ID` - Existing notebook ID in source workspace
//! - `FABIO_TEST_CAPACITY_ID` - Active capacity ID (optional)
//!
//! All integration tests are marked `#[ignore]` so they don't run during normal
//! `cargo test`. Run them explicitly with:
//!
//! ```sh
//! cargo test --test '*' -- --ignored
//! ```
//!
//! Or run a specific test group:
//!
//! ```sh
//! cargo test --test e2e_workspace -- --ignored
//! ```

use assert_cmd::Command;
use serde_json::Value;
use std::env;

/// Test configuration loaded from environment variables.
pub struct TestConfig {
    pub source_workspace: String,
    pub source_lakehouse: String,
    pub dest_workspace: String,
    pub dest_lakehouse: String,
    pub notebook_id: String,
    pub capacity_id: String,
}

impl TestConfig {
    /// Load configuration from environment variables.
    ///
    /// # Panics
    ///
    /// Panics if required environment variables are not set.
    pub fn from_env() -> Self {
        Self {
            source_workspace: env::var("FABIO_TEST_SOURCE_WORKSPACE")
                .unwrap_or_else(|_| "1619af1e-c97a-43f8-8f1e-c1990b0b3914".to_string()),
            source_lakehouse: env::var("FABIO_TEST_SOURCE_LAKEHOUSE")
                .unwrap_or_else(|_| "d4f7211c-cc03-4a86-9f16-0bb2f2af3c59".to_string()),
            dest_workspace: env::var("FABIO_TEST_DEST_WORKSPACE")
                .unwrap_or_else(|_| "c112b455-f02d-4c18-a0af-be75a82816d0".to_string()),
            dest_lakehouse: env::var("FABIO_TEST_DEST_LAKEHOUSE")
                .unwrap_or_else(|_| "36755b0f-b6af-4699-8945-df3aeb8717d6".to_string()),
            notebook_id: env::var("FABIO_TEST_NOTEBOOK_ID")
                .unwrap_or_else(|_| "38177352-dc1c-440b-a860-a83ec508e806".to_string()),
            capacity_id: env::var("FABIO_TEST_CAPACITY_ID")
                .unwrap_or_else(|_| "afdf5707-dde2-41ef-9d98-df65aa40eb7f".to_string()),
        }
    }
}

/// Build a `fabio` command ready to execute.
pub fn fabio() -> Command {
    Command::cargo_bin("fabio").expect("failed to find fabio binary")
}

/// Parse the stdout of a successful fabio command as JSON.
pub fn parse_json(output: &assert_cmd::assert::Assert) -> Value {
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    serde_json::from_str(&stdout).expect("failed to parse stdout as JSON")
}

/// Extract the `data` field from a fabio JSON envelope.
pub fn extract_data(json: &Value) -> &Value {
    json.get("data").expect("missing 'data' field in response")
}

/// Extract the `count` field from a fabio JSON list envelope.
pub fn extract_count(json: &Value) -> u64 {
    json.get("count")
        .and_then(Value::as_u64)
        .expect("missing 'count' field in response")
}

/// Generate a unique name for test artifacts to avoid collisions.
pub fn unique_name(prefix: &str) -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("{prefix}_{ts}")
}
