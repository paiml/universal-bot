//! Error types for the Bedrock client

use thiserror::Error;

/// Errors that can occur when using the Bedrock client
#[derive(Error, Debug)]
pub enum BedrockError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Invalid input parameters
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Invalid response from Bedrock
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// AWS service error
    #[error("AWS service error: {0}")]
    ServiceError(String),

    /// Request failed
    #[error("Request failed: {0}")]
    RequestFailed(String),

    /// Connection pool exhausted
    #[error("Connection pool exhausted: {0}")]
    PoolExhausted(String),

    /// Timeout error
    #[error("Request timed out: {0}")]
    Timeout(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimited(String),

    /// Model not available
    #[error("Model not available: {0}")]
    ModelUnavailable(String),

    /// Content filtering error
    #[error("Content filtered: {0}")]
    ContentFiltered(String),

    /// Token limit exceeded
    #[error("Token limit exceeded: {0}")]
    TokenLimitExceeded(String),

    /// Authentication error
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Authorization error
    #[error("Authorization failed: {0}")]
    Authorization(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl BedrockError {
    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::ServiceError(_) => true,
            Self::RequestFailed(_) => true,
            Self::Timeout(_) => true,
            Self::RateLimited(_) => true,
            Self::ModelUnavailable(_) => true,
            Self::Internal(_) => true,
            _ => false,
        }
    }

    /// Get the error category
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::Configuration(_) => ErrorCategory::Configuration,
            Self::InvalidInput(_) => ErrorCategory::Client,
            Self::InvalidResponse(_) => ErrorCategory::Server,
            Self::ServiceError(_) => ErrorCategory::Server,
            Self::RequestFailed(_) => ErrorCategory::Network,
            Self::PoolExhausted(_) => ErrorCategory::Resource,
            Self::Timeout(_) => ErrorCategory::Network,
            Self::RateLimited(_) => ErrorCategory::RateLimit,
            Self::ModelUnavailable(_) => ErrorCategory::Server,
            Self::ContentFiltered(_) => ErrorCategory::Content,
            Self::TokenLimitExceeded(_) => ErrorCategory::Resource,
            Self::Authentication(_) => ErrorCategory::Authentication,
            Self::Authorization(_) => ErrorCategory::Authorization,
            Self::Internal(_) => ErrorCategory::Internal,
        }
    }

    /// Get the HTTP status code that would be appropriate for this error
    pub fn status_code(&self) -> u16 {
        match self {
            Self::Configuration(_) => 500,
            Self::InvalidInput(_) => 400,
            Self::InvalidResponse(_) => 502,
            Self::ServiceError(_) => 502,
            Self::RequestFailed(_) => 503,
            Self::PoolExhausted(_) => 503,
            Self::Timeout(_) => 504,
            Self::RateLimited(_) => 429,
            Self::ModelUnavailable(_) => 503,
            Self::ContentFiltered(_) => 400,
            Self::TokenLimitExceeded(_) => 400,
            Self::Authentication(_) => 401,
            Self::Authorization(_) => 403,
            Self::Internal(_) => 500,
        }
    }
}

/// Error categories for classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    /// Configuration or setup error
    Configuration,
    /// Client-side error (bad request)
    Client,
    /// Server-side error
    Server,
    /// Network connectivity error
    Network,
    /// Resource exhaustion
    Resource,
    /// Rate limiting
    RateLimit,
    /// Content policy violation
    Content,
    /// Authentication failure
    Authentication,
    /// Authorization failure
    Authorization,
    /// Internal system error
    Internal,
}

impl ErrorCategory {
    /// Check if errors in this category are typically retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Configuration => false,
            Self::Client => false,
            Self::Server => true,
            Self::Network => true,
            Self::Resource => true,
            Self::RateLimit => true,
            Self::Content => false,
            Self::Authentication => false,
            Self::Authorization => false,
            Self::Internal => true,
        }
    }
}

/// Result type alias for Bedrock operations
pub type Result<T> = std::result::Result<T, BedrockError>;

/// Convert AWS SDK errors to Bedrock errors
impl From<aws_sdk_bedrockruntime::Error> for BedrockError {
    fn from(error: aws_sdk_bedrockruntime::Error) -> Self {
        // For now, just convert all AWS errors to a generic service error
        // TODO: Match specific error types when they're available
        Self::RequestFailed(error.to_string())
    }
}

/// Convert standard errors to Bedrock errors
impl From<anyhow::Error> for BedrockError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error.to_string())
    }
}

impl From<std::time::SystemTimeError> for BedrockError {
    fn from(error: std::time::SystemTimeError) -> Self {
        Self::Internal(format!("System time error: {}", error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_retryability() {
        assert!(BedrockError::ServiceError("test".to_string()).is_retryable());
        assert!(BedrockError::RateLimited("test".to_string()).is_retryable());
        assert!(!BedrockError::InvalidInput("test".to_string()).is_retryable());
        assert!(!BedrockError::Authentication("test".to_string()).is_retryable());
    }

    #[test]
    fn test_error_categories() {
        assert_eq!(
            BedrockError::InvalidInput("test".to_string()).category(),
            ErrorCategory::Client
        );
        assert_eq!(
            BedrockError::ServiceError("test".to_string()).category(),
            ErrorCategory::Server
        );
        assert_eq!(
            BedrockError::RateLimited("test".to_string()).category(),
            ErrorCategory::RateLimit
        );
    }

    #[test]
    fn test_status_codes() {
        assert_eq!(
            BedrockError::InvalidInput("test".to_string()).status_code(),
            400
        );
        assert_eq!(
            BedrockError::Authentication("test".to_string()).status_code(),
            401
        );
        assert_eq!(
            BedrockError::Authorization("test".to_string()).status_code(),
            403
        );
        assert_eq!(
            BedrockError::RateLimited("test".to_string()).status_code(),
            429
        );
        assert_eq!(
            BedrockError::ServiceError("test".to_string()).status_code(),
            502
        );
    }

    #[test]
    fn test_category_retryability() {
        assert!(ErrorCategory::Server.is_retryable());
        assert!(ErrorCategory::Network.is_retryable());
        assert!(!ErrorCategory::Client.is_retryable());
        assert!(!ErrorCategory::Authentication.is_retryable());
    }
}
