use std::fmt;

use thiserror::Error;

/// Machine-readable error codes for structured error output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    AuthRequired,
    AuthExpired,
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
            Self::AuthExpired => write!(f, "AUTH_EXPIRED"),
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
        let msg = message.into();
        let code = match status {
            401 => ErrorCode::AuthRequired,
            403 => ErrorCode::AuthExpired,
            404 => ErrorCode::NotFound,
            409 => ErrorCode::Conflict,
            429 => ErrorCode::RateLimited,
            _ if (500..600).contains(&status) => ErrorCode::ApiError,
            _ => ErrorCode::ApiError,
        };
        let hint = match code {
            ErrorCode::AuthRequired => {
                Some("Run 'fabio auth login' to authenticate.".to_string())
            }
            ErrorCode::AuthExpired => Some(
                "Credentials expired or insufficient permissions. Run 'fabio auth login' to re-authenticate.".to_string(),
            ),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_code_display() {
        assert_eq!(ErrorCode::AuthRequired.to_string(), "AUTH_REQUIRED");
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
}
