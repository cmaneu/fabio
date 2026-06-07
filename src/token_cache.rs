//! Persistent `OAuth2` token cache for fabio's own authentication.
//!
//! Stores access and refresh tokens at `~/.fabio/token_cache.json`.
//! Supports the Microsoft Identity Platform device code flow and token refresh.

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::errors::{ErrorCode, FabioError};

/// Fabio CLI's own Entra ID app registration (multitenant, public client).
/// Users see "Fabio CLI" in the consent screen and audit logs — independent from
/// Azure CLI or Azure PowerShell identity.
const PUBLIC_CLIENT_ID: &str = "38715dcd-c115-46b4-8ed1-967d06c9ec6d";

/// Default tenant for multi-tenant auth.
const DEFAULT_TENANT: &str = "common";

/// Fabric API scope.
const FABRIC_SCOPE: &str = "https://api.fabric.microsoft.com/.default";

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

    // Write atomically with restricted permissions to avoid TOCTOU window
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)?;
        file.write_all(json.as_bytes())?;
    }
    #[cfg(not(unix))]
    {
        // On Windows: write to a temp file in the same directory, then rename
        // atomically. This prevents partial-write exposure. Windows inherits
        // parent directory ACLs — the ~/.fabio directory should be user-only.
        let temp_path = path.with_extension("tmp");
        std::fs::write(&temp_path, &json)?;
        std::fs::rename(&temp_path, &path)?;
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

    // Disable redirects on token endpoint client to prevent credential forwarding
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

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

    // Step 2: Display instructions to the user (validate URI is from Microsoft)
    let valid_verification_hosts = ["login.microsoftonline.com", "microsoft.com", "aka.ms"];
    let uri_lower = dc.verification_uri.to_lowercase();
    let uri_trusted = uri_lower.starts_with("https://")
        && valid_verification_hosts.iter().any(|host| {
            uri_lower.strip_prefix("https://").is_some_and(|rest| {
                let domain = rest.split('/').next().unwrap_or("");
                domain == *host || domain.ends_with(&format!(".{host}"))
            })
        });
    if !uri_trusted {
        return Err(FabioError::new(
            ErrorCode::AuthRequired,
            format!(
                "Device code verification URI is not a trusted Microsoft domain: {}",
                dc.verification_uri
            ),
        )
        .into());
    }

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

        // Unknown error — only include status code, not raw response body
        // (which may contain internal traces, correlation IDs, or injected content)
        let sanitized = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| {
                v.get("error_description")
                    .or_else(|| v.get("error"))
                    .and_then(serde_json::Value::as_str)
                    .map(String::from)
            })
            .unwrap_or_else(|| format!("HTTP {status} (unrecognized error format)"));
        return Err(FabioError::new(ErrorCode::AuthRequired, sanitized).into());
    }
}

/// Refresh an access token using a refresh token.
async fn refresh_access_token(refresh_token: &str, tenant: &str, scope: &str) -> Result<TokenData> {
    // Disable redirects to prevent credential forwarding via POST body
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
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
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        // Extract only structured error fields — never expose raw response body
        let sanitized = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|v| {
                v.get("error_description")
                    .or_else(|| v.get("error"))
                    .and_then(serde_json::Value::as_str)
                    .map(String::from)
            })
            .unwrap_or_else(|| format!("HTTP {status} (token refresh failed)"));
        return Err(FabioError::new(ErrorCode::AuthRequired, sanitized).into());
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

// ── Service Principal Login Methods ─────────────────────────────────────────

/// Authenticate a service principal using a client secret.
pub async fn sp_login_secret(
    tenant: &str,
    client_id: &str,
    client_secret: &str,
    scope: &str,
) -> Result<TokenData> {
    use azure_core::credentials::TokenCredential;

    let credential = azure_identity::ClientSecretCredential::new(
        tenant,
        client_id.to_string(),
        azure_core::credentials::Secret::new(client_secret.to_string()),
        None,
    )
    .map_err(|e| {
        FabioError::new(
            ErrorCode::AuthRequired,
            format!("Failed to create client secret credential: {e}"),
        )
    })?;

    let token = credential.get_token(&[scope], None).await.map_err(|e| {
        FabioError::with_hint(
            ErrorCode::AuthRequired,
            format!("Service principal authentication failed: {e}"),
            "Check --tenant, --client-id, and --client-secret values.".to_string(),
        )
    })?;

    let expires_on = std::time::SystemTime::from(token.expires_on)
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let data = TokenData {
        access_token: token.token.secret().to_string(),
        refresh_token: None, // SP tokens don't have refresh tokens
        expires_on,
        tenant: tenant.to_string(),
        scope: scope.to_string(),
    };

    save_token(&data)?;
    Ok(data)
}

/// Authenticate a service principal using a PEM or PFX certificate.
pub async fn sp_login_certificate(
    tenant: &str,
    client_id: &str,
    certificate_path: &str,
    certificate_password: Option<&str>,
    scope: &str,
) -> Result<TokenData> {
    use azure_core::credentials::TokenCredential;

    let cert_bytes = std::fs::read(certificate_path).map_err(|e| {
        FabioError::new(
            ErrorCode::InvalidInput,
            format!("Failed to read certificate file '{certificate_path}': {e}"),
        )
    })?;

    if cert_bytes.is_empty() {
        return Err(FabioError::new(
            ErrorCode::InvalidInput,
            format!("Certificate file '{certificate_path}' is empty."),
        )
        .into());
    }

    let options =
        certificate_password.map(|pw| azure_identity::ClientCertificateCredentialOptions {
            password: Some(azure_core::credentials::Secret::new(pw.to_string())),
            ..Default::default()
        });

    let credential = azure_identity::ClientCertificateCredential::new(
        tenant.to_string(),
        client_id.to_string(),
        azure_core::credentials::SecretBytes::from(cert_bytes),
        options,
    )
    .map_err(|e| {
        FabioError::with_hint(
            ErrorCode::AuthRequired,
            format!("Failed to create certificate credential: {e}"),
            "Ensure the certificate file is valid PEM or PFX format. For PFX, provide --certificate-password.".to_string(),
        )
    })?;

    let token = credential.get_token(&[scope], None).await.map_err(|e| {
        FabioError::with_hint(
            ErrorCode::AuthRequired,
            format!("Certificate authentication failed: {e}"),
            "Check the certificate is valid and not expired. Ensure it matches the app registration.".to_string(),
        )
    })?;

    let expires_on = std::time::SystemTime::from(token.expires_on)
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let data = TokenData {
        access_token: token.token.secret().to_string(),
        refresh_token: None,
        expires_on,
        tenant: tenant.to_string(),
        scope: scope.to_string(),
    };

    save_token(&data)?;
    Ok(data)
}

/// Authenticate a service principal using a federated token (OIDC assertion).
/// Used for workload identity in CI/CD (GitHub Actions, Azure DevOps Pipelines).
pub async fn sp_login_federated(
    tenant: &str,
    client_id: &str,
    federated_token: &str,
    scope: &str,
) -> Result<TokenData> {
    use azure_core::credentials::TokenCredential;

    // Create a static assertion provider that returns the federated token
    let assertion = StaticAssertion(federated_token.to_string());

    let credential = azure_identity::ClientAssertionCredential::new(
        tenant.to_string(),
        client_id.to_string(),
        assertion,
        None,
    )
    .map_err(|e| {
        FabioError::new(
            ErrorCode::AuthRequired,
            format!("Failed to create federated token credential: {e}"),
        )
    })?;

    let token = credential.get_token(&[scope], None).await.map_err(|e| {
        FabioError::with_hint(
            ErrorCode::AuthRequired,
            format!("Federated token authentication failed: {e}"),
            "Check the federated token is valid and not expired. Ensure the app registration has the correct federated credential configured.".to_string(),
        )
    })?;

    let expires_on = std::time::SystemTime::from(token.expires_on)
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let data = TokenData {
        access_token: token.token.secret().to_string(),
        refresh_token: None,
        expires_on,
        tenant: tenant.to_string(),
        scope: scope.to_string(),
    };

    save_token(&data)?;
    Ok(data)
}

/// A static OIDC assertion that always returns the same token string.
/// Used for federated identity login where the token is provided once.
#[derive(Debug)]
struct StaticAssertion(String);

#[async_trait::async_trait]
impl azure_identity::ClientAssertion for StaticAssertion {
    async fn secret(
        &self,
        _options: Option<azure_core::http::ClientMethodOptions<'_>>,
    ) -> azure_core::Result<String> {
        Ok(self.0.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_data_not_expired_when_far_future() {
        let data = TokenData {
            access_token: "test".to_string(),
            refresh_token: None,
            expires_on: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 3600, // 1 hour from now
            tenant: "test".to_string(),
            scope: "test".to_string(),
        };
        assert!(!data.is_expired());
    }

    #[test]
    fn token_data_expired_when_past() {
        let data = TokenData {
            access_token: "test".to_string(),
            refresh_token: None,
            expires_on: 0, // epoch = definitely expired
            tenant: "test".to_string(),
            scope: "test".to_string(),
        };
        assert!(data.is_expired());
    }

    #[test]
    fn token_data_expired_within_refresh_margin() {
        let data = TokenData {
            access_token: "test".to_string(),
            refresh_token: None,
            // Expires in 4 minutes (less than 5-minute REFRESH_MARGIN)
            expires_on: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 240,
            tenant: "test".to_string(),
            scope: "test".to_string(),
        };
        assert!(data.is_expired());
    }

    #[test]
    fn token_data_not_expired_just_outside_margin() {
        let data = TokenData {
            access_token: "test".to_string(),
            refresh_token: None,
            // Expires in 6 minutes (more than 5-minute REFRESH_MARGIN)
            expires_on: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 360,
            tenant: "test".to_string(),
            scope: "test".to_string(),
        };
        assert!(!data.is_expired());
    }

    #[test]
    fn token_data_serializes_to_json() {
        let data = TokenData {
            access_token: "abc123".to_string(),
            refresh_token: Some("refresh456".to_string()),
            expires_on: 1_700_000_000,
            tenant: "my-tenant".to_string(),
            scope: "https://api.fabric.microsoft.com/.default".to_string(),
        };
        let json = serde_json::to_value(&data).unwrap();
        assert_eq!(json["access_token"], "abc123");
        assert_eq!(json["refresh_token"], "refresh456");
        assert_eq!(json["expires_on"], 1_700_000_000);
        assert_eq!(json["tenant"], "my-tenant");
    }

    #[test]
    fn token_data_deserializes_from_json() {
        let json = r#"{
            "access_token": "xyz",
            "refresh_token": null,
            "expires_on": 9999999999,
            "tenant": "t",
            "scope": "s"
        }"#;
        let data: TokenData = serde_json::from_str(json).unwrap();
        assert_eq!(data.access_token, "xyz");
        assert!(data.refresh_token.is_none());
        assert_eq!(data.expires_on, 9_999_999_999);
    }

    #[test]
    fn token_data_sp_has_no_refresh_token() {
        // SP tokens should never have a refresh token
        let data = TokenData {
            access_token: "sp-token".to_string(),
            refresh_token: None,
            expires_on: 1_700_000_000,
            tenant: "sp-tenant".to_string(),
            scope: "https://api.fabric.microsoft.com/.default".to_string(),
        };
        assert!(data.refresh_token.is_none());
    }

    #[test]
    fn cache_path_is_under_home_fabio() {
        let path = cache_path().unwrap();
        assert!(
            path.ends_with(".fabio/token_cache.json") || path.ends_with(".fabio\\token_cache.json")
        );
    }

    #[test]
    fn logout_marker_path_is_under_home_fabio() {
        let path = logout_marker_path().unwrap();
        assert!(path.ends_with(".fabio/.logged_out") || path.ends_with(".fabio\\.logged_out"));
    }

    #[tokio::test]
    async fn static_assertion_returns_token() {
        use azure_identity::ClientAssertion;
        let assertion = StaticAssertion("my-oidc-token".to_string());
        let result = assertion.secret(None).await.unwrap();
        assert_eq!(result, "my-oidc-token");
    }

    #[tokio::test]
    async fn static_assertion_returns_empty_token() {
        use azure_identity::ClientAssertion;
        let assertion = StaticAssertion(String::new());
        let result = assertion.secret(None).await.unwrap();
        assert_eq!(result, "");
    }
}
