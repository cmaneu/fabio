use std::fmt::Write;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::header::AUTHORIZATION;
use reqwest::{Client, Response, StatusCode};
use serde_json::Value;
use tokio::time::sleep;

use azure_core::credentials::TokenCredential;

use crate::errors::{ErrorCode, FabioError};

const FABRIC_BASE_URL: &str = "https://api.fabric.microsoft.com/v1";
const ONELAKE_DFS_URL: &str = "https://onelake.dfs.fabric.microsoft.com";
const ONELAKE_BLOB_URL: &str = "https://onelake.blob.fabric.microsoft.com";
const FABRIC_SCOPE: &str = "https://analysis.windows.net/powerbi/api/.default";
const STORAGE_SCOPE: &str = "https://storage.azure.com/.default";
const LRO_POLL_INTERVAL: Duration = Duration::from_secs(2);
const LRO_MAX_WAIT: Duration = Duration::from_secs(120);

/// Fabric API client with token management and LRO polling.
#[derive(Clone)]
pub struct FabricClient {
    http: Client,
    token: Arc<tokio::sync::RwLock<Option<String>>>,
    storage_token: Arc<tokio::sync::RwLock<Option<String>>>,
}

impl FabricClient {
    pub fn new() -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            http,
            token: Arc::new(tokio::sync::RwLock::new(None)),
            storage_token: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    /// Ensure we have a valid Fabric API token.
    pub async fn require_auth(&self) -> Result<String> {
        let guard = self.token.read().await;
        if let Some(ref t) = *guard {
            return Ok(t.clone());
        }
        drop(guard);

        let token = get_token(FABRIC_SCOPE).await?;
        let mut guard = self.token.write().await;
        *guard = Some(token.clone());
        drop(guard);
        Ok(token)
    }

    /// Get a storage token for `OneLake` operations.
    pub async fn require_storage_auth(&self) -> Result<String> {
        let guard = self.storage_token.read().await;
        if let Some(ref t) = *guard {
            return Ok(t.clone());
        }
        drop(guard);

        let token = get_token(STORAGE_SCOPE).await?;
        let mut guard = self.storage_token.write().await;
        *guard = Some(token.clone());
        drop(guard);
        Ok(token)
    }

    /// GET request to Fabric REST API.
    pub async fn get(&self, path: &str) -> Result<Value> {
        let token = self.require_auth().await?;
        let url = format!("{FABRIC_BASE_URL}{path}");

        let resp = self
            .http
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await
            .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

        handle_response(resp).await
    }

    /// POST request to Fabric REST API, optionally polling for LRO completion.
    pub async fn post(&self, path: &str, body: &Value, poll: bool) -> Result<Value> {
        let token = self.require_auth().await?;
        let url = format!("{FABRIC_BASE_URL}{path}");

        let resp = self
            .http
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .json(body)
            .send()
            .await
            .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

        if poll && resp.status() == StatusCode::ACCEPTED {
            return self.poll_lro(resp).await;
        }

        handle_response(resp).await
    }

    /// PATCH request to Fabric REST API.
    pub async fn patch(&self, path: &str, body: &Value) -> Result<Value> {
        let token = self.require_auth().await?;
        let url = format!("{FABRIC_BASE_URL}{path}");

        let resp = self
            .http
            .patch(&url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .json(body)
            .send()
            .await
            .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

        handle_response(resp).await
    }

    /// DELETE request to Fabric REST API.
    pub async fn delete(&self, path: &str) -> Result<Value> {
        let token = self.require_auth().await?;
        let url = format!("{FABRIC_BASE_URL}{path}");

        let resp = self
            .http
            .delete(&url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await
            .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

        handle_response(resp).await
    }

    /// Upload a file to `OneLake` via DFS (create + append + flush).
    pub async fn upload_onelake_file(
        &self,
        workspace: &str,
        item: &str,
        path: &str,
        data: &[u8],
    ) -> Result<Value> {
        let token = self.require_storage_auth().await?;
        let base = format!("{ONELAKE_DFS_URL}/{workspace}/{item}/{path}");

        // Step 1: Create
        self.http
            .put(format!("{base}?resource=file"))
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header("Content-Length", "0")
            .send()
            .await
            .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

        // Step 2: Append
        self.http
            .patch(format!("{base}?action=append&position=0"))
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header("Content-Length", data.len().to_string())
            .body(data.to_vec())
            .send()
            .await
            .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

        // Step 3: Flush
        self.http
            .patch(format!("{base}?action=flush&position={}", data.len()))
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header("Content-Length", "0")
            .send()
            .await
            .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

        Ok(serde_json::json!({
            "path": path,
            "size": data.len(),
            "status": "uploaded"
        }))
    }

    /// Download a file from `OneLake` via DFS.
    pub async fn download_onelake_file(
        &self,
        workspace: &str,
        item: &str,
        path: &str,
    ) -> Result<Vec<u8>> {
        let token = self.require_storage_auth().await?;
        let url = format!("{ONELAKE_DFS_URL}/{workspace}/{item}/{path}");

        let resp = self
            .http
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await
            .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(FabioError::from_status(status, text).into());
        }

        Ok(resp.bytes().await?.to_vec())
    }

    /// List files in `OneLake` via DFS.
    pub async fn list_onelake_files(
        &self,
        workspace: &str,
        item: &str,
        directory: Option<&str>,
    ) -> Result<Vec<Value>> {
        let token = self.require_storage_auth().await?;
        let mut url =
            format!("{ONELAKE_DFS_URL}/{workspace}/{item}?resource=filesystem&recursive=true");
        if let Some(dir) = directory {
            let _ = write!(url, "&directory={dir}");
        }

        let resp = self
            .http
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await
            .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

        let body = handle_response(resp).await?;
        let paths = body
            .get("paths")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        Ok(paths)
    }

    /// Server-side file copy via `OneLake` Blob API.
    pub async fn copy_onelake_file(
        &self,
        src_workspace: &str,
        src_item: &str,
        src_path: &str,
        dst_workspace: &str,
        dst_item: &str,
        dst_path: &str,
    ) -> Result<Value> {
        let token = self.require_storage_auth().await?;
        let source_url = format!("{ONELAKE_BLOB_URL}/{src_workspace}/{src_item}/{src_path}");
        let dest_url = format!("{ONELAKE_BLOB_URL}/{dst_workspace}/{dst_item}/{dst_path}");

        let resp = self
            .http
            .put(&dest_url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header("x-ms-copy-source", &source_url)
            .header("Content-Length", "0")
            .send()
            .await
            .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

        if !resp.status().is_success() && resp.status() != StatusCode::ACCEPTED {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(FabioError::from_status(status, text).into());
        }

        Ok(serde_json::json!({
            "source": src_path,
            "destination": dst_path,
            "status": "copied"
        }))
    }

    /// Delete a file from `OneLake` via DFS.
    pub async fn delete_onelake_file(
        &self,
        workspace: &str,
        item: &str,
        path: &str,
    ) -> Result<Value> {
        let token = self.require_storage_auth().await?;
        let url = format!("{ONELAKE_DFS_URL}/{workspace}/{item}/{path}");

        let resp = self
            .http
            .delete(&url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await
            .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(FabioError::from_status(status, text).into());
        }

        Ok(serde_json::json!({
            "path": path,
            "status": "deleted"
        }))
    }

    /// Delete a directory recursively from `OneLake` via DFS.
    pub async fn delete_onelake_directory(
        &self,
        workspace: &str,
        item: &str,
        path: &str,
    ) -> Result<Value> {
        let token = self.require_storage_auth().await?;
        let url = format!("{ONELAKE_DFS_URL}/{workspace}/{item}/{path}?recursive=true");

        let resp = self
            .http
            .delete(&url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await
            .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(FabioError::from_status(status, text).into());
        }

        Ok(serde_json::json!({
            "path": path,
            "status": "deleted"
        }))
    }

    /// Run a notebook and return the job instance ID.
    pub async fn run_notebook(&self, workspace: &str, item_id: &str) -> Result<String> {
        let token = self.require_auth().await?;
        let url = format!(
            "{FABRIC_BASE_URL}/workspaces/{workspace}/items/{item_id}/jobs/instances?jobType=RunNotebook"
        );

        let resp = self
            .http
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

        if resp.status() != StatusCode::ACCEPTED && resp.status() != StatusCode::OK {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(FabioError::from_status(status, text).into());
        }

        // Extract job instance ID from Location header
        let location = resp
            .headers()
            .get("location")
            .or_else(|| resp.headers().get("Location"))
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        let job_id = location.rsplit('/').next().unwrap_or("").to_string();
        if job_id.is_empty() {
            return Err(
                FabioError::api_error("No job instance ID returned from run request").into(),
            );
        }

        Ok(job_id)
    }

    /// Poll a long-running operation until completion.
    async fn poll_lro(&self, initial_response: Response) -> Result<Value> {
        let location = initial_response
            .headers()
            .get("location")
            .or_else(|| initial_response.headers().get("Location"))
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        let operation_id = initial_response
            .headers()
            .get("x-ms-operation-id")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        let Some(poll_url) = location
            .or_else(|| operation_id.map(|op_id| format!("{FABRIC_BASE_URL}/operations/{op_id}")))
        else {
            // No LRO info - try to parse response body
            return handle_response(initial_response).await;
        };

        let token = self.require_auth().await?;
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > LRO_MAX_WAIT {
                return Err(FabioError::new(ErrorCode::Timeout, "LRO polling timed out").into());
            }

            sleep(LRO_POLL_INTERVAL).await;

            let resp = self
                .http
                .get(&poll_url)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .send()
                .await
                .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

            let status = resp.status();
            if status == StatusCode::OK
                || status == StatusCode::CREATED
                || status == StatusCode::ACCEPTED
            {
                // Capture resource location before consuming body
                let resource_location = resp
                    .headers()
                    .get("location")
                    .or_else(|| resp.headers().get("Location"))
                    .and_then(|v| v.to_str().ok())
                    .map(String::from);

                // Parse body and check operation status
                let body: Value = resp.json().await.unwrap_or(Value::Null);
                let op_status = body.get("status").and_then(Value::as_str).unwrap_or("");

                match op_status {
                    "Succeeded" | "succeeded" => {
                        // Check if there's a resource location for the final result
                        if let Some(ref loc) = resource_location {
                            let final_resp = self
                                .http
                                .get(loc)
                                .header(AUTHORIZATION, format!("Bearer {token}"))
                                .send()
                                .await
                                .map_err(|e| {
                                    FabioError::new(ErrorCode::NetworkError, e.to_string())
                                })?;
                            if final_resp.status().is_success() {
                                let final_body: Value =
                                    final_resp.json().await.unwrap_or(Value::Null);
                                if !final_body.is_null() {
                                    return Ok(final_body);
                                }
                            }
                        }
                        return Ok(body);
                    }
                    "Failed" | "failed" => {
                        let msg = body
                            .get("error")
                            .and_then(|e| e.get("message"))
                            .and_then(Value::as_str)
                            .unwrap_or("LRO failed");
                        return Err(FabioError::api_error(msg).into());
                    }
                    // Running, NotStarted, or other in-progress states - keep polling
                    _ => {
                        // If 200/201 with no status field, operation is complete
                        if op_status.is_empty()
                            && (status == StatusCode::OK || status == StatusCode::CREATED)
                        {
                            return Ok(body);
                        }
                        continue;
                    }
                }
            }
            // Unexpected status
            let text = resp.text().await.unwrap_or_default();
            return Err(FabioError::from_status(status.as_u16(), text).into());
        }
    }
}

/// Get an access token using the default Azure credential chain.
async fn get_token(scope: &str) -> Result<String> {
    use azure_identity::DeveloperToolsCredential;

    let credential = DeveloperToolsCredential::new(None)
        .context("Failed to create Azure credential. Run 'az login' first.")?;

    let token = credential.get_token(&[scope], None).await.map_err(|e| {
        FabioError::auth_required(format!("Authentication failed: {e}. Run 'az login' first."))
    })?;

    Ok(token.token.secret().to_string())
}

/// Handle an HTTP response, converting errors to `FabioError`.
async fn handle_response(resp: Response) -> Result<Value> {
    let status = resp.status();

    if status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        if text.is_empty() {
            return Ok(Value::Null);
        }
        let value: Value = serde_json::from_str(&text)
            .map_err(|e| FabioError::api_error(format!("Invalid JSON response: {e}")))?;
        return Ok(value);
    }

    let status_code = status.as_u16();
    let text = resp.text().await.unwrap_or_default();

    // Check for CapacityNotActive
    if text.contains("CapacityNotActive") {
        return Err(FabioError::new(
            ErrorCode::CapacityInactive,
            "Capacity is inactive. Resume it in the Azure portal.",
        )
        .into());
    }

    // Try to extract error message from JSON body
    let message = serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|v| {
            v.get("error")
                .and_then(|e| e.get("message"))
                .and_then(Value::as_str)
                .map(String::from)
                .or_else(|| v.get("message").and_then(Value::as_str).map(String::from))
        })
        .unwrap_or_else(|| format!("HTTP {status_code}: {text}"));

    Err(FabioError::from_status(status_code, message).into())
}
