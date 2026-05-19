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
