use std::fmt;

use thiserror::Error;

/// Machine-readable error codes for structured error output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    AuthRequired,
    Forbidden,
    NotFound,
    Conflict,
    RateLimited,
    CapacityInactive,
    InvalidInput,
    ApiError,
    Timeout,
    NetworkError,
    Unknown,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuthRequired => write!(f, "AUTH_REQUIRED"),
            Self::Forbidden => write!(f, "FORBIDDEN"),
            Self::NotFound => write!(f, "NOT_FOUND"),
            Self::Conflict => write!(f, "CONFLICT"),
            Self::RateLimited => write!(f, "RATE_LIMITED"),
            Self::CapacityInactive => write!(f, "CAPACITY_INACTIVE"),
            Self::InvalidInput => write!(f, "INVALID_INPUT"),
            Self::ApiError => write!(f, "API_ERROR"),
            Self::Timeout => write!(f, "TIMEOUT"),
            Self::NetworkError => write!(f, "NETWORK_ERROR"),
            Self::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Structured error type for the fabio CLI.
#[derive(Debug, Error)]
#[error("{code}: {message}")]
pub struct FabioError {
    pub code: ErrorCode,
    pub message: String,
    /// Optional hint with valid values or corrected command for agent self-correction.
    pub hint: Option<String>,
}

impl FabioError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            hint: None,
        }
    }

    /// Create an error with a hint for agent self-correction.
    pub fn with_hint(code: ErrorCode, message: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            hint: Some(hint.into()),
        }
    }

    pub fn auth_required(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::AuthRequired, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }

    pub fn api_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ApiError, message)
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidInput, message)
    }
}

/// Convert HTTP status codes to appropriate error codes.
impl FabioError {
    pub fn from_status(status: u16, message: impl Into<String>) -> Self {
        Self::from_status_with_body(status, message, "")
    }

    /// Create an error from HTTP status with the full response body for context-aware hints.
    pub fn from_status_with_body(status: u16, message: impl Into<String>, body: &str) -> Self {
        let msg = message.into();
        let code = match status {
            401 => ErrorCode::AuthRequired,
            403 => ErrorCode::Forbidden,
            404 => ErrorCode::NotFound,
            409 => ErrorCode::Conflict,
            429 => ErrorCode::RateLimited,
            _ if (500..600).contains(&status) => ErrorCode::ApiError,
            _ => ErrorCode::ApiError,
        };
        let hint = match code {
            ErrorCode::AuthRequired => Some("Run 'fabio auth login' to authenticate.".to_string()),
            ErrorCode::Forbidden => Some(forbidden_hint(&msg, body)),
            ErrorCode::RateLimited => {
                Some("Too many requests. Retry after a short backoff.".to_string())
            }
            _ => None,
        };
        Self {
            code,
            message: msg,
            hint,
        }
    }
}

/// Enrich a `FabioError` with an operation-specific permission hint.
///
/// If the error is `Forbidden`, replaces the generic hint with one tailored
/// to the operation (e.g., "item create requires Member role"). For non-Forbidden
/// errors, returns the original error unchanged.
pub fn enrich_forbidden(err: anyhow::Error, operation: &str, required_role: &str) -> anyhow::Error {
    let Some(fabio_err) = err.downcast_ref::<FabioError>() else {
        return err;
    };

    if fabio_err.code != ErrorCode::Forbidden {
        return err;
    }

    let hint = format!(
        "'{operation}' requires at least '{required_role}' role on the workspace. \
         Workspace roles: Admin > Member > Contributor > Viewer. \
         Check your access with: fabio workspace show --id <workspace-id>. \
         Ask a workspace Admin to grant you the required role."
    );

    FabioError::with_hint(ErrorCode::Forbidden, fabio_err.message.clone(), hint).into()
}

/// Generate a context-aware hint for 403 Forbidden errors based on the error message and body.
fn forbidden_hint(message: &str, body: &str) -> String {
    let msg_lower = message.to_lowercase();
    let body_lower = body.to_lowercase();
    let combined = format!("{msg_lower} {body_lower}");

    // Detect git-specific permission issues (check before generic patterns)
    if combined.contains("git") || combined.contains("source control") {
        return "Insufficient permissions for git operations. Git connect/commit/pull requires \
                Admin or Member workspace role. Verify your role with: \
                fabio workspace show --id <workspace-id>."
            .to_string();
    }

    // Detect OneLake/storage permission issues (check before generic patterns)
    if combined.contains("storage") || combined.contains("onelake") || combined.contains("blob") {
        return "Insufficient OneLake storage permissions. Ensure you have at least \
                Contributor role on the workspace, or that OneLake data access is enabled. \
                Verify workspace role with: fabio workspace show --id <workspace-id>."
            .to_string();
    }

    // Detect generic insufficient workspace role
    if combined.contains("insufficient privileges")
        || combined.contains("does not have permission")
        || combined.contains("unauthorized")
        || combined.contains("access denied")
        || combined.contains("forbidden")
    {
        return "Insufficient workspace permissions. Fabric workspace roles required: \
                Admin (full control), Member (create/edit items), Contributor (edit items), \
                Viewer (read-only). Check your role with: fabio workspace show --id <workspace-id> \
                or ask a workspace Admin to grant you the required role."
            .to_string();
    }

    // Generic 403 hint
    "Insufficient permissions for this operation. Possible causes: \
     (1) Your workspace role (Viewer/Contributor/Member/Admin) is too low for this action. \
     (2) The API scope in your token lacks the required permission. \
     (3) A tenant admin policy restricts this operation. \
     Check your role with: fabio workspace show --id <workspace-id>. \
     Re-authenticate with: fabio auth login."
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_code_display() {
        assert_eq!(ErrorCode::AuthRequired.to_string(), "AUTH_REQUIRED");
        assert_eq!(ErrorCode::Forbidden.to_string(), "FORBIDDEN");
        assert_eq!(ErrorCode::NotFound.to_string(), "NOT_FOUND");
        assert_eq!(ErrorCode::RateLimited.to_string(), "RATE_LIMITED");
    }

    #[test]
    fn fabio_error_new_has_no_hint() {
        let err = FabioError::new(ErrorCode::NotFound, "item not found");
        assert_eq!(err.code, ErrorCode::NotFound);
        assert_eq!(err.message, "item not found");
        assert!(err.hint.is_none());
    }

    #[test]
    fn fabio_error_with_hint_carries_hint() {
        let err = FabioError::with_hint(
            ErrorCode::InvalidInput,
            "invalid mode",
            "Valid values: Overwrite, Append",
        );
        assert_eq!(err.code, ErrorCode::InvalidInput);
        assert_eq!(err.hint.unwrap(), "Valid values: Overwrite, Append");
    }

    #[test]
    fn from_status_401_maps_to_auth_required_with_hint() {
        let err = FabioError::from_status(401, "unauthorized");
        assert_eq!(err.code, ErrorCode::AuthRequired);
        assert!(err.hint.is_some());
        assert!(err.hint.unwrap().contains("fabio auth login"));
    }

    #[test]
    fn from_status_429_maps_to_rate_limited_with_hint() {
        let err = FabioError::from_status(429, "slow down");
        assert_eq!(err.code, ErrorCode::RateLimited);
        assert!(err.hint.unwrap().contains("backoff"));
    }

    #[test]
    fn from_status_404_has_no_hint() {
        let err = FabioError::from_status(404, "not found");
        assert_eq!(err.code, ErrorCode::NotFound);
        assert!(err.hint.is_none());
    }

    #[test]
    fn from_status_500_maps_to_api_error() {
        let err = FabioError::from_status(500, "server error");
        assert_eq!(err.code, ErrorCode::ApiError);
    }

    #[test]
    fn from_status_403_maps_to_forbidden_with_hint() {
        let err = FabioError::from_status(403, "insufficient privileges for this action");
        assert_eq!(err.code, ErrorCode::Forbidden);
        let hint = err.hint.unwrap();
        assert!(
            hint.contains("workspace role"),
            "Hint should mention workspace roles: {hint}"
        );
    }

    #[test]
    fn from_status_403_generic_message_gives_generic_hint() {
        let err = FabioError::from_status(403, "some error");
        assert_eq!(err.code, ErrorCode::Forbidden);
        let hint = err.hint.unwrap();
        assert!(
            hint.contains("Insufficient permissions"),
            "Hint should be generic: {hint}"
        );
    }

    #[test]
    fn from_status_403_with_body_context_detects_storage() {
        let err = FabioError::from_status_with_body(
            403,
            "AuthorizationFailure",
            r#"{"error":{"code":"AuthorizationFailure","message":"OneLake storage denied"}}"#,
        );
        assert_eq!(err.code, ErrorCode::Forbidden);
        let hint = err.hint.unwrap();
        assert!(
            hint.contains("OneLake"),
            "Hint should mention OneLake: {hint}"
        );
    }

    #[test]
    fn from_status_403_with_body_context_detects_git() {
        let err = FabioError::from_status_with_body(
            403,
            "permission denied",
            r#"{"error":{"code":"Forbidden","message":"Git source control access denied"}}"#,
        );
        assert_eq!(err.code, ErrorCode::Forbidden);
        let hint = err.hint.unwrap();
        assert!(
            hint.contains("git operations"),
            "Hint should mention git: {hint}"
        );
    }
}
