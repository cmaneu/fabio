use std::fmt::Write;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
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
const SQL_SCOPE: &str = "https://database.windows.net/.default";
const LRO_POLL_INTERVAL: Duration = Duration::from_secs(2);
const LRO_MAX_WAIT: Duration = Duration::from_secs(120);

/// Minimum remaining lifetime before a token is considered expired and re-acquired.
const TOKEN_REFRESH_MARGIN: Duration = Duration::from_secs(300); // 5 minutes

/// Response from a paginated list endpoint.
pub struct PaginatedResponse {
    /// Collected items across all fetched pages.
    pub items: Vec<Value>,
    /// If present, there are more pages available. Pass this token to fetch the next page.
    pub continuation_token: Option<String>,
}

/// Which credential source successfully provided a token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialSource {
    /// Service principal via `AZURE_TENANT_ID` + `AZURE_CLIENT_ID` + `AZURE_CLIENT_SECRET` env vars.
    Environment,
    /// Azure Managed Identity (system-assigned or user-assigned).
    ManagedIdentity,
    /// Azure CLI (`az login`).
    AzureCli,
    /// Azure Developer CLI (`azd auth login`).
    AzureDeveloperCli,
}

impl std::fmt::Display for CredentialSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Environment => write!(f, "environment (service principal)"),
            Self::ManagedIdentity => write!(f, "managed identity"),
            Self::AzureCli => write!(f, "Azure CLI"),
            Self::AzureDeveloperCli => write!(f, "Azure Developer CLI"),
        }
    }
}

/// A cached token with its expiry time.
#[derive(Clone)]
struct CachedToken {
    token: String,
    expires_on: std::time::SystemTime,
}

impl CachedToken {
    fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now();
        self.expires_on
            .duration_since(now)
            .map_or(true, |remaining| remaining < TOKEN_REFRESH_MARGIN)
    }
}

/// Fabric API client with token management and LRO polling.
#[derive(Clone)]
pub struct FabricClient {
    http: Client,
    fabric_token: Arc<tokio::sync::RwLock<Option<CachedToken>>>,
    storage_token: Arc<tokio::sync::RwLock<Option<CachedToken>>>,
    sql_token: Arc<tokio::sync::RwLock<Option<CachedToken>>>,
    credential_source: Arc<tokio::sync::RwLock<Option<CredentialSource>>>,
}

impl FabricClient {
    pub fn new() -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            http,
            fabric_token: Arc::new(tokio::sync::RwLock::new(None)),
            storage_token: Arc::new(tokio::sync::RwLock::new(None)),
            sql_token: Arc::new(tokio::sync::RwLock::new(None)),
            credential_source: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    /// Ensure we have a valid Fabric API token (auto-refreshes if near expiry).
    pub async fn require_auth(&self) -> Result<String> {
        {
            let guard = self.fabric_token.read().await;
            if let Some(ref cached) = *guard {
                if !cached.is_expired() {
                    return Ok(cached.token.clone());
                }
            }
        }

        let (token, source) = acquire_token(FABRIC_SCOPE).await?;
        let mut guard = self.fabric_token.write().await;
        *guard = Some(token.clone());
        drop(guard);

        let mut src_guard = self.credential_source.write().await;
        *src_guard = Some(source);
        drop(src_guard);

        Ok(token.token)
    }

    /// Get a storage token for `OneLake` operations (auto-refreshes if near expiry).
    pub async fn require_storage_auth(&self) -> Result<String> {
        {
            let guard = self.storage_token.read().await;
            if let Some(ref cached) = *guard {
                if !cached.is_expired() {
                    return Ok(cached.token.clone());
                }
            }
        }

        let (token, _source) = acquire_token(STORAGE_SCOPE).await?;
        let mut guard = self.storage_token.write().await;
        *guard = Some(token.clone());
        drop(guard);
        Ok(token.token)
    }

    /// Get a SQL token for TDS connections (scope: `database.windows.net`).
    pub async fn require_sql_auth(&self) -> Result<String> {
        {
            let guard = self.sql_token.read().await;
            if let Some(ref cached) = *guard {
                if !cached.is_expired() {
                    return Ok(cached.token.clone());
                }
            }
        }

        let (token, _source) = acquire_token(SQL_SCOPE).await?;
        let mut guard = self.sql_token.write().await;
        *guard = Some(token.clone());
        drop(guard);
        Ok(token.token)
    }

    /// Get a token for an arbitrary scope (used for Kusto queries with dynamic query URIs).
    /// Not cached — each call acquires a fresh token from the credential chain.
    pub async fn require_token_for_scope(&self, scope: &str) -> Result<String> {
        let (token, _source) = acquire_token(scope).await?;
        Ok(token.token)
    }

    /// Returns the inner HTTP client for direct requests (e.g., Kusto REST API).
    pub const fn http(&self) -> &Client {
        &self.http
    }

    /// Returns which credential source was used for the last successful authentication.
    pub async fn credential_source(&self) -> Option<CredentialSource> {
        let guard = self.credential_source.read().await;
        *guard
    }

    /// Invalidate the cached Fabric API token (forces re-acquisition on next request).
    async fn invalidate_fabric_token(&self) {
        let mut guard = self.fabric_token.write().await;
        *guard = None;
    }

    /// Invalidate the cached Storage token (forces re-acquisition on next request).
    #[allow(dead_code)]
    async fn invalidate_storage_token(&self) {
        let mut guard = self.storage_token.write().await;
        *guard = None;
    }

    /// GET request to Fabric REST API (retries once on 401 with fresh token).
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

        if resp.status() == StatusCode::UNAUTHORIZED {
            // Token may have expired server-side; refresh and retry once
            self.invalidate_fabric_token().await;
            let token = self.require_auth().await?;
            let resp = self
                .http
                .get(&url)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .send()
                .await
                .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;
            return handle_response(resp).await;
        }

        handle_response(resp).await
    }

    /// GET request returning raw text response as a JSON string value.
    /// Used for endpoints that return non-JSON content (e.g., file downloads).
    pub async fn get_text(&self, path: &str) -> Result<String> {
        let token = self.require_auth().await?;
        let url = format!("{FABRIC_BASE_URL}{path}");

        let resp = self
            .http
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await
            .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

        if resp.status() == StatusCode::UNAUTHORIZED {
            self.invalidate_fabric_token().await;
            let token = self.require_auth().await?;
            let resp = self
                .http
                .get(&url)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .send()
                .await
                .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

            let status = resp.status();
            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(FabioError::new(ErrorCode::ApiError, body).into());
            }
            return Ok(resp.text().await.unwrap_or_default());
        }

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(FabioError::new(ErrorCode::ApiError, body).into());
        }
        Ok(resp.text().await.unwrap_or_default())
    }

    /// GET a paginated list from Fabric REST API.
    ///
    /// When `paginate` is true, follows `continuationToken` until all pages are fetched.
    /// When `start_token` is provided, begins pagination from that token.
    /// Returns a `PaginatedResponse` with all collected items and optional continuation token.
    pub async fn get_list(
        &self,
        path: &str,
        array_field: &str,
        paginate: bool,
        start_token: Option<&str>,
    ) -> Result<PaginatedResponse> {
        let token = self.require_auth().await?;
        let mut all_items: Vec<Value> = Vec::new();
        let mut continuation_token: Option<String> = start_token.map(String::from);

        loop {
            let url = continuation_token.as_ref().map_or_else(
                || format!("{FABRIC_BASE_URL}{path}"),
                |ct| {
                    let separator = if path.contains('?') { '&' } else { '?' };
                    format!("{FABRIC_BASE_URL}{path}{separator}continuationToken={ct}")
                },
            );

            let resp = self
                .http
                .get(&url)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .send()
                .await
                .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

            let body = handle_response(resp).await?;

            // Extract items from the specified array field (try primary, then fallback to "value")
            let items = body
                .get(array_field)
                .and_then(Value::as_array)
                .or_else(|| {
                    if array_field == "value" {
                        None
                    } else {
                        body.get("value").and_then(Value::as_array)
                    }
                })
                .cloned()
                .unwrap_or_default();

            all_items.extend(items);

            // Check for continuation token
            let next_token = body
                .get("continuationToken")
                .and_then(Value::as_str)
                .map(String::from);

            if !paginate || next_token.is_none() {
                // Return with the token if we're not paginating (so caller can expose it)
                return Ok(PaginatedResponse {
                    items: all_items,
                    continuation_token: next_token,
                });
            }

            continuation_token = next_token;
        }
    }

    /// POST request to Fabric REST API, optionally polling for LRO completion.
    /// Retries once on 401 with a fresh token.
    /// Retries up to 3 times on 429/430 (rate limited) with exponential backoff.
    pub async fn post(&self, path: &str, body: &Value, poll: bool) -> Result<Value> {
        const MAX_RATE_LIMIT_RETRIES: u32 = 3;
        let url = format!("{FABRIC_BASE_URL}{path}");
        let mut attempt: u32 = 0;

        loop {
            let token = self.require_auth().await?;

            let resp = self
                .http
                .post(&url)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .json(body)
                .send()
                .await
                .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

            if resp.status() == StatusCode::UNAUTHORIZED {
                self.invalidate_fabric_token().await;
                let token = self.require_auth().await?;
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
                return handle_response(resp).await;
            }

            // Retry on rate-limit (429) or Spark capacity throttle (430)
            let status_code = resp.status().as_u16();
            if (status_code == 429 || status_code == 430) && attempt < MAX_RATE_LIMIT_RETRIES {
                attempt += 1;
                let backoff_secs = 10u64 * u64::from(attempt); // 10s, 20s, 30s
                eprintln!(
                    "Rate limited (HTTP {status_code}). Retrying in {backoff_secs}s (attempt {attempt}/{MAX_RATE_LIMIT_RETRIES})..."
                );
                sleep(Duration::from_secs(backoff_secs)).await;
                continue;
            }

            if poll && resp.status() == StatusCode::ACCEPTED {
                return self.poll_lro(resp).await;
            }

            return handle_response(resp).await;
        }
    }

    /// POST request with raw text body (text/plain content type).
    pub async fn post_raw(&self, path: &str, content: &str) -> Result<Value> {
        let token = self.require_auth().await?;
        let url = format!("{FABRIC_BASE_URL}{path}");

        let resp = self
            .http
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header("Content-Type", "text/plain")
            .body(content.to_owned())
            .send()
            .await
            .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

        if resp.status() == StatusCode::UNAUTHORIZED {
            self.invalidate_fabric_token().await;
            let token = self.require_auth().await?;
            let resp = self
                .http
                .post(&url)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .header("Content-Type", "text/plain")
                .body(content.to_owned())
                .send()
                .await
                .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;
            return handle_response(resp).await;
        }

        handle_response(resp).await
    }

    /// POST request with configurable LRO wait and timeout (retries once on 401).
    pub async fn post_with_timeout(
        &self,
        path: &str,
        body: &Value,
        wait: bool,
        timeout_secs: u64,
    ) -> Result<Value> {
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

        if resp.status() == StatusCode::UNAUTHORIZED {
            self.invalidate_fabric_token().await;
            let token = self.require_auth().await?;
            let resp = self
                .http
                .post(&url)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .json(body)
                .send()
                .await
                .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;
            return self
                .handle_post_with_timeout_response(resp, wait, timeout_secs)
                .await;
        }

        self.handle_post_with_timeout_response(resp, wait, timeout_secs)
            .await
    }

    /// Handle the response from `post_with_timeout` (factored out for retry).
    async fn handle_post_with_timeout_response(
        &self,
        resp: Response,
        wait: bool,
        timeout_secs: u64,
    ) -> Result<Value> {
        if resp.status() == StatusCode::ACCEPTED {
            if wait {
                return self
                    .poll_lro_with_timeout(resp, Duration::from_secs(timeout_secs))
                    .await;
            }
            let location = resp
                .headers()
                .get("location")
                .or_else(|| resp.headers().get("Location"))
                .and_then(|v| v.to_str().ok())
                .map(String::from);
            let operation_id = resp
                .headers()
                .get("x-ms-operation-id")
                .and_then(|v| v.to_str().ok())
                .map(String::from);

            return Ok(serde_json::json!({
                "status": "accepted",
                "operationId": operation_id,
                "location": location,
            }));
        }

        handle_response(resp).await
    }

    /// GET request that handles LRO (202 Accepted) responses (retries once on 401).
    pub async fn get_with_lro(&self, path: &str) -> Result<Value> {
        let token = self.require_auth().await?;
        let url = format!("{FABRIC_BASE_URL}{path}");

        let resp = self
            .http
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await
            .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

        if resp.status() == StatusCode::UNAUTHORIZED {
            self.invalidate_fabric_token().await;
            let token = self.require_auth().await?;
            let resp = self
                .http
                .get(&url)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .send()
                .await
                .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;
            if resp.status() == StatusCode::ACCEPTED {
                return self.poll_lro(resp).await;
            }
            return handle_response(resp).await;
        }

        if resp.status() == StatusCode::ACCEPTED {
            return self.poll_lro(resp).await;
        }

        handle_response(resp).await
    }

    /// PATCH request to Fabric REST API (retries once on 401).
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

        if resp.status() == StatusCode::UNAUTHORIZED {
            self.invalidate_fabric_token().await;
            let token = self.require_auth().await?;
            let resp = self
                .http
                .patch(&url)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .json(body)
                .send()
                .await
                .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;
            return handle_response(resp).await;
        }

        handle_response(resp).await
    }

    /// PUT request to Fabric REST API (retries once on 401).
    pub async fn put(&self, path: &str, body: &Value) -> Result<Value> {
        let token = self.require_auth().await?;
        let url = format!("{FABRIC_BASE_URL}{path}");

        let resp = self
            .http
            .put(&url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .json(body)
            .send()
            .await
            .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

        if resp.status() == StatusCode::UNAUTHORIZED {
            self.invalidate_fabric_token().await;
            let token = self.require_auth().await?;
            let resp = self
                .http
                .put(&url)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .json(body)
                .send()
                .await
                .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;
            return handle_response(resp).await;
        }

        handle_response(resp).await
    }

    /// PUT request with raw text body (e.g., for file uploads requiring text/plain).
    pub async fn put_raw(&self, path: &str, content: &str) -> Result<Value> {
        let token = self.require_auth().await?;
        let url = format!("{FABRIC_BASE_URL}{path}");

        let resp = self
            .http
            .put(&url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header("Content-Type", "text/plain")
            .body(content.to_owned())
            .send()
            .await
            .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

        if resp.status() == StatusCode::UNAUTHORIZED {
            self.invalidate_fabric_token().await;
            let token = self.require_auth().await?;
            let resp = self
                .http
                .put(&url)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .header("Content-Type", "text/plain")
                .body(content.to_owned())
                .send()
                .await
                .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;
            return handle_response(resp).await;
        }

        handle_response(resp).await
    }

    /// DELETE request to Fabric REST API (retries once on 401).
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

        if resp.status() == StatusCode::UNAUTHORIZED {
            self.invalidate_fabric_token().await;
            let token = self.require_auth().await?;
            let resp = self
                .http
                .delete(&url)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .send()
                .await
                .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;
            return handle_response(resp).await;
        }

        handle_response(resp).await
    }

    /// POST request to Power BI REST API (different base URL, same Fabric auth scope).
    /// Used for Power BI-specific operations like Publish to Web.
    pub async fn post_powerbi(&self, path: &str, body: &Value) -> Result<Value> {
        let token = self.require_auth().await?;
        let url = format!("https://api.powerbi.com/v1.0/myorg{path}");

        let resp = self
            .http
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .json(body)
            .send()
            .await
            .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

        if resp.status() == StatusCode::UNAUTHORIZED {
            self.invalidate_fabric_token().await;
            let token = self.require_auth().await?;
            let resp = self
                .http
                .post(&url)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .json(body)
                .send()
                .await
                .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;
            return handle_response(resp).await;
        }

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

    /// Get file properties from `OneLake` via DFS HEAD request.
    /// Returns headers including `Content-MD5` and `ETag`.
    pub async fn get_file_properties(
        &self,
        workspace: &str,
        item: &str,
        path: &str,
    ) -> Result<Value> {
        let token = self.require_storage_auth().await?;
        let url = format!("{ONELAKE_DFS_URL}/{workspace}/{item}/{path}");

        let resp = self
            .http
            .head(&url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await
            .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(FabioError::from_status(status, text).into());
        }

        let headers = resp.headers();
        let content_md5 = headers
            .get("Content-MD5")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let etag = headers
            .get("ETag")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let content_length = headers
            .get("Content-Length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        Ok(serde_json::json!({
            "path": path,
            "contentMD5": content_md5,
            "eTag": etag,
            "contentLength": content_length
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
                return Err(FabioError::with_hint(
                    ErrorCode::Timeout,
                    format!(
                        "LRO polling timed out after {}s (poll URL: {poll_url})",
                        LRO_MAX_WAIT.as_secs()
                    ),
                    "The operation may still be running server-side. \
                     Check status with: fabio jobs list --workspace <WS>, or retry with a longer \
                     --timeout if supported. Some operations (notebook run, graph refresh) \
                     can take several minutes on small capacities.",
                )
                .into());
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

    /// Poll a long-running operation with a custom timeout.
    async fn poll_lro_with_timeout(
        &self,
        initial_response: Response,
        max_wait: Duration,
    ) -> Result<Value> {
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
            return handle_response(initial_response).await;
        };

        let token = self.require_auth().await?;
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > max_wait {
                return Err(FabioError::with_hint(
                    ErrorCode::Timeout,
                    format!(
                        "LRO polling timed out after {}s (poll URL: {poll_url})",
                        max_wait.as_secs()
                    ),
                    "The operation may still be running server-side. \
                     Check status with: fabio jobs list --workspace <WS>, or retry with a longer \
                     --timeout if supported.",
                )
                .into());
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
                let resource_location = resp
                    .headers()
                    .get("location")
                    .or_else(|| resp.headers().get("Location"))
                    .and_then(|v| v.to_str().ok())
                    .map(String::from);

                let body: Value = resp.json().await.unwrap_or(Value::Null);
                let op_status = body.get("status").and_then(Value::as_str).unwrap_or("");

                match op_status {
                    "Succeeded" | "succeeded" => {
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
                    _ => {
                        if op_status.is_empty()
                            && (status == StatusCode::OK || status == StatusCode::CREATED)
                        {
                            return Ok(body);
                        }
                        continue;
                    }
                }
            }
            let text = resp.text().await.unwrap_or_default();
            return Err(FabioError::from_status(status.as_u16(), text).into());
        }
    }
}

/// Acquire an access token using the credential chain:
/// 1. Environment (service principal via `AZURE_TENANT_ID` + `AZURE_CLIENT_ID` + `AZURE_CLIENT_SECRET`)
/// 2. Managed Identity (for Azure-hosted workloads)
/// 3. Developer Tools (Azure CLI, then Azure Developer CLI)
///
/// Returns the cached token and which credential source provided it.
async fn acquire_token(scope: &str) -> Result<(CachedToken, CredentialSource)> {
    // 1. Try environment credentials (service principal)
    if let Some(result) = try_environment_credential(scope).await {
        return result;
    }

    // 2. Try managed identity (only when running in Azure)
    if let Some(result) = try_managed_identity_credential(scope).await {
        return result;
    }

    // 3. Fall back to developer tools (az cli, azd)
    try_developer_tools_credential(scope).await
}

/// Try service principal authentication via environment variables.
/// Returns None if env vars are not set (skip to next), Some(Ok/Err) if attempted.
async fn try_environment_credential(
    scope: &str,
) -> Option<Result<(CachedToken, CredentialSource)>> {
    let tenant_id = std::env::var("AZURE_TENANT_ID").ok()?;
    let client_id = std::env::var("AZURE_CLIENT_ID").ok()?;
    let client_secret = std::env::var("AZURE_CLIENT_SECRET").ok();

    // Need at least tenant + client_id + secret for client credentials flow
    let secret = client_secret?;

    let credential = match azure_identity::ClientSecretCredential::new(
        &tenant_id,
        client_id,
        azure_core::credentials::Secret::new(secret),
        None,
    ) {
        Ok(c) => c,
        Err(e) => {
            return Some(Err(FabioError::auth_required(format!(
                "Failed to create service principal credential: {e}"
            ))
            .into()));
        }
    };

    match credential.get_token(&[scope], None).await {
        Ok(token) => {
            let expires_on = std::time::SystemTime::from(token.expires_on);
            Some(Ok((
                CachedToken {
                    token: token.token.secret().to_string(),
                    expires_on,
                },
                CredentialSource::Environment,
            )))
        }
        Err(e) => Some(Err(FabioError::with_hint(
            ErrorCode::AuthRequired,
            format!("Service principal authentication failed: {e}"),
            "Check AZURE_TENANT_ID, AZURE_CLIENT_ID, and AZURE_CLIENT_SECRET environment variables.".to_string(),
        )
        .into())),
    }
}

/// Try managed identity authentication.
/// Returns None if not running in a managed identity environment (skip to next).
async fn try_managed_identity_credential(
    scope: &str,
) -> Option<Result<(CachedToken, CredentialSource)>> {
    // Only attempt managed identity if we detect an Azure hosting environment.
    // Check for common managed identity indicators:
    // - IDENTITY_ENDPOINT (App Service, Container Apps)
    // - MSI_ENDPOINT (legacy App Service)
    // - IMDS is always available on Azure VMs but we can't cheaply detect that without a network call
    let has_identity_env = std::env::var("IDENTITY_ENDPOINT").is_ok()
        || std::env::var("MSI_ENDPOINT").is_ok()
        || std::env::var("AZURE_FEDERATED_TOKEN_FILE").is_ok();

    if !has_identity_env {
        return None;
    }

    let Ok(credential) = azure_identity::ManagedIdentityCredential::new(None) else {
        return None;
    };

    match credential.get_token(&[scope], None).await {
        Ok(token) => {
            let expires_on = std::time::SystemTime::from(token.expires_on);
            Some(Ok((
                CachedToken {
                    token: token.token.secret().to_string(),
                    expires_on,
                },
                CredentialSource::ManagedIdentity,
            )))
        }
        Err(_) => None, // Managed identity not available, fall through
    }
}

/// Try developer tools credentials (Azure CLI, then Azure Developer CLI).
async fn try_developer_tools_credential(scope: &str) -> Result<(CachedToken, CredentialSource)> {
    // Try Azure CLI first
    if let Ok(credential) = azure_identity::AzureCliCredential::new(None) {
        if let Ok(token) = credential.get_token(&[scope], None).await {
            let expires_on = std::time::SystemTime::from(token.expires_on);
            return Ok((
                CachedToken {
                    token: token.token.secret().to_string(),
                    expires_on,
                },
                CredentialSource::AzureCli,
            ));
        }
    }

    // Try Azure Developer CLI
    if let Ok(credential) = azure_identity::AzureDeveloperCliCredential::new(None) {
        if let Ok(token) = credential.get_token(&[scope], None).await {
            let expires_on = std::time::SystemTime::from(token.expires_on);
            return Ok((
                CachedToken {
                    token: token.token.secret().to_string(),
                    expires_on,
                },
                CredentialSource::AzureDeveloperCli,
            ));
        }
    }

    Err(FabioError::with_hint(
        ErrorCode::AuthRequired,
        "No valid credentials found. Tried: environment variables, managed identity, Azure CLI, Azure Developer CLI.",
        "Run 'az login' or set AZURE_TENANT_ID + AZURE_CLIENT_ID + AZURE_CLIENT_SECRET environment variables.".to_string(),
    )
    .into())
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

    Err(FabioError::from_status_with_body(status_code, message, &text).into())
}
