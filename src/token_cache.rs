//! Persistent `OAuth2` token cache for fabio's own authentication.
//!
//! Stores access and refresh tokens at `~/.fabio/token_cache.json`.
//! Supports the Microsoft Identity Platform device code flow and token refresh.

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::errors::{ErrorCode, FabioError};

/// Well-known public client ID (Azure PowerShell) — pre-consented in all Azure AD tenants
/// for Power BI / Fabric scopes. This allows fabio to authenticate without
/// requiring users to register their own app.
const PUBLIC_CLIENT_ID: &str = "1950a258-227b-4e31-a9cf-717495945fc2";

/// Default tenant for multi-tenant auth.
const DEFAULT_TENANT: &str = "common";

/// Fabric API scope.
const FABRIC_SCOPE: &str = "https://analysis.windows.net/powerbi/api/.default";

/// Margin before expiry to consider token stale and attempt refresh.
const REFRESH_MARGIN: Duration = Duration::from_secs(300); // 5 minutes

/// Cached token data persisted to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Unix timestamp (seconds) when the access token expires.
    pub expires_on: u64,
    /// The tenant used for authentication.
    pub tenant: String,
    /// The scope used for authentication.
    pub scope: String,
}

impl TokenData {
    /// Check if the access token is expired or about to expire.
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.expires_on.saturating_sub(REFRESH_MARGIN.as_secs()) <= now
    }
}

/// Device code response from Microsoft Identity Platform.
#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    #[serde(default)]
    message: String,
    /// Polling interval in seconds.
    #[serde(default = "default_interval")]
    interval: u64,
    /// Lifetime of the device code in seconds.
    #[serde(default = "default_expires_in")]
    expires_in: u64,
}

const fn default_interval() -> u64 {
    5
}
const fn default_expires_in() -> u64 {
    900
}

/// Token response from Microsoft Identity Platform.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    /// Seconds until the access token expires.
    expires_in: u64,
}

/// Error response from the token endpoint during polling.
#[derive(Debug, Deserialize)]
struct TokenErrorResponse {
    error: String,
    #[serde(default)]
    error_description: String,
}

/// Returns the path to the token cache file.
fn cache_path() -> Result<PathBuf> {
    let home = home::home_dir().ok_or_else(|| {
        FabioError::new(
            ErrorCode::Unknown,
            "Cannot determine home directory for token cache.",
        )
    })?;
    Ok(home.join(".fabio").join("token_cache.json"))
}

/// Returns the path to the logout marker file.
fn logout_marker_path() -> Result<PathBuf> {
    let home = home::home_dir().ok_or_else(|| {
        FabioError::new(
            ErrorCode::Unknown,
            "Cannot determine home directory for token cache.",
        )
    })?;
    Ok(home.join(".fabio").join(".logged_out"))
}

/// Check if the user has explicitly logged out.
/// When true, the credential chain should NOT fall back to Azure CLI or other sources.
pub fn is_explicitly_logged_out() -> bool {
    logout_marker_path().ok().is_some_and(|p| p.exists())
}

/// Load cached token from disk.
pub fn load_cached_token() -> Option<TokenData> {
    let path = cache_path().ok()?;
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Save token data to disk (creates ~/.fabio/ if needed).
/// Also removes the logout marker if present.
pub fn save_token(data: &TokenData) -> Result<()> {
    let path = cache_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
        // Restrict directory permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
        }
    }
    let json = serde_json::to_string_pretty(data)?;
    std::fs::write(&path, json)?;

    // Restrict permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }

    // Remove logout marker on successful login
    if let Ok(marker) = logout_marker_path() {
        if marker.exists() {
            std::fs::remove_file(&marker).ok();
        }
    }

    Ok(())
}

/// Delete the token cache file and write a logout marker.
pub fn clear_cache() -> Result<()> {
    let path = cache_path()?;
    if path.exists() {
        std::fs::remove_file(&path)?;
    }

    // Write logout marker so credential chain doesn't fall back to Azure CLI
    let marker = logout_marker_path()?;
    if let Some(parent) = marker.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&marker, "")?;

    Ok(())
}

/// Attempt to get a valid access token from cache, refreshing if needed.
/// Returns None if no cache exists or refresh fails.
pub async fn get_valid_token() -> Option<TokenData> {
    let cached = load_cached_token()?;

    if !cached.is_expired() {
        return Some(cached);
    }

    // Try refreshing with the refresh token
    let refresh_token = cached.refresh_token.as_ref()?;
    refresh_access_token(refresh_token, &cached.tenant, &cached.scope)
        .await
        .ok()
        .inspect(|new_data| {
            save_token(new_data).ok();
        })
}

/// Attempt to get a valid access token for a specific scope (e.g., storage, SQL).
/// Uses the refresh token from the cached session to acquire a token for the requested scope.
pub async fn get_token_for_scope(scope: &str) -> Option<TokenData> {
    let cached = load_cached_token()?;
    let refresh_token = cached.refresh_token.as_ref()?;

    // If the cached token already covers this scope and is valid, return it
    if cached.scope == scope && !cached.is_expired() {
        return Some(cached);
    }

    // Use the refresh token to get a token for the specific scope
    refresh_access_token(refresh_token, &cached.tenant, scope)
        .await
        .ok()
}

/// Run the `OAuth2` device code flow interactively.
#[allow(clippy::too_many_lines)]
pub async fn device_code_login(tenant: Option<&str>, scope: Option<&str>) -> Result<TokenData> {
    let tenant = tenant.unwrap_or(DEFAULT_TENANT);
    let scope = scope.unwrap_or(FABRIC_SCOPE);

    let http = reqwest::Client::new();

    // Step 1: Request device code
    let device_code_url =
        format!("https://login.microsoftonline.com/{tenant}/oauth2/v2.0/devicecode");

    let resp = http
        .post(&device_code_url)
        .form(&[
            ("client_id", PUBLIC_CLIENT_ID),
            ("scope", &format!("{scope} offline_access")),
        ])
        .send()
        .await
        .map_err(|e| {
            FabioError::new(
                ErrorCode::NetworkError,
                format!("Device code request failed: {e}"),
            )
        })?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(FabioError::with_hint(
            ErrorCode::AuthRequired,
            format!("Device code request failed: {text}"),
            "Check your network connection and tenant ID.".to_string(),
        )
        .into());
    }

    let dc: DeviceCodeResponse = resp.json().await.map_err(|e| {
        FabioError::new(
            ErrorCode::AuthRequired,
            format!("Invalid device code response: {e}"),
        )
    })?;

    // Step 2: Display instructions to the user
    if dc.message.is_empty() {
        eprintln!(
            "To sign in, open: {}\nEnter the code: {}",
            dc.verification_uri, dc.user_code
        );
    } else {
        eprintln!("{}", dc.message);
    }

    // Step 3: Poll for token
    let token_url = format!("https://login.microsoftonline.com/{tenant}/oauth2/v2.0/token");
    let interval = Duration::from_secs(dc.interval.max(1));
    let deadline = SystemTime::now() + Duration::from_secs(dc.expires_in);

    loop {
        tokio::time::sleep(interval).await;

        if SystemTime::now() > deadline {
            return Err(FabioError::new(
                ErrorCode::Timeout,
                "Device code flow timed out. Please try again.",
            )
            .into());
        }

        let resp = http
            .post(&token_url)
            .form(&[
                ("client_id", PUBLIC_CLIENT_ID),
                ("device_code", dc.device_code.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await
            .map_err(|e| {
                FabioError::new(ErrorCode::NetworkError, format!("Token poll failed: {e}"))
            })?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();

        if status.is_success() {
            let token_resp: TokenResponse = serde_json::from_str(&body).map_err(|e| {
                FabioError::new(
                    ErrorCode::AuthRequired,
                    format!("Invalid token response: {e}"),
                )
            })?;

            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let data = TokenData {
                access_token: token_resp.access_token,
                refresh_token: token_resp.refresh_token,
                expires_on: now + token_resp.expires_in,
                tenant: tenant.to_string(),
                scope: scope.to_string(),
            };

            save_token(&data)?;
            return Ok(data);
        }

        // Check error type
        if let Ok(err_resp) = serde_json::from_str::<TokenErrorResponse>(&body) {
            match err_resp.error.as_str() {
                "authorization_pending" => continue, // User hasn't authenticated yet
                "slow_down" => {
                    // Back off
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
                "authorization_declined" => {
                    return Err(FabioError::new(
                        ErrorCode::AuthRequired,
                        "Authentication was declined by the user.",
                    )
                    .into());
                }
                "expired_token" => {
                    return Err(FabioError::new(
                        ErrorCode::Timeout,
                        "Device code expired. Please try 'fabio auth login' again.",
                    )
                    .into());
                }
                _ => {
                    return Err(FabioError::with_hint(
                        ErrorCode::AuthRequired,
                        format!("Authentication failed: {}", err_resp.error_description),
                        format!("Error code: {}", err_resp.error),
                    )
                    .into());
                }
            }
        }

        // Unknown error
        return Err(FabioError::new(
            ErrorCode::AuthRequired,
            format!("Unexpected token response (HTTP {status}): {body}"),
        )
        .into());
    }
}

/// Refresh an access token using a refresh token.
async fn refresh_access_token(refresh_token: &str, tenant: &str, scope: &str) -> Result<TokenData> {
    let http = reqwest::Client::new();
    let token_url = format!("https://login.microsoftonline.com/{tenant}/oauth2/v2.0/token");

    let resp = http
        .post(&token_url)
        .form(&[
            ("client_id", PUBLIC_CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("scope", &format!("{scope} offline_access")),
        ])
        .send()
        .await
        .map_err(|e| {
            FabioError::new(
                ErrorCode::NetworkError,
                format!("Token refresh failed: {e}"),
            )
        })?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(FabioError::new(
            ErrorCode::AuthRequired,
            format!("Token refresh failed: {text}"),
        )
        .into());
    }

    let token_resp: TokenResponse = resp.json().await.map_err(|e| {
        FabioError::new(
            ErrorCode::AuthRequired,
            format!("Invalid refresh response: {e}"),
        )
    })?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    Ok(TokenData {
        access_token: token_resp.access_token,
        refresh_token: token_resp
            .refresh_token
            .or_else(|| Some(refresh_token.to_string())),
        expires_on: now + token_resp.expires_in,
        tenant: tenant.to_string(),
        scope: scope.to_string(),
    })
}
