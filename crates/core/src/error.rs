//! Error types for Universal Bot
//!
//! This module defines the error types used throughout the Universal Bot framework,
//! following best practices for error handling in Rust.

use std::fmt;

use thiserror::Error;

/// Main error type for Universal Bot
#[derive(Error, Debug)]
pub enum Error {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Pipeline processing error
    #[error("Pipeline error: {0}")]
    Pipeline(String),

    /// Context management error
    #[error("Context error: {0}")]
    Context(String),

    /// Plugin error
    #[error("Plugin error: {0}")]
    Plugin(String),

    /// AI provider error
    #[error("AI provider error: {0}")]
    Provider(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// Timeout error
    #[error("Operation timed out after {0:?}")]
    Timeout(std::time::Duration),

    /// Rate limit error
    #[error("Rate limit exceeded")]
    RateLimit,

    /// Authentication error
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Authorization error
    #[error("Authorization failed: {0}")]
    Authorization(String),

    /// Resource not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Database error
    #[error("Database error: {0}")]
    Database(String),

    /// Cache error
    #[error("Cache error: {0}")]
    Cache(String),

    /// Initialization error
    #[error("Initialization failed: {0}")]
    Initialization(String),

    /// Internal error (should not happen)
    #[error("Internal error: {0}")]
    Internal(String),

    /// Other error with context
    #[error("{message}")]
    Other {
        /// Error message
        message: String,
        /// Optional error source
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl Error {
    /// Create a new error with a message
    pub fn new(message: impl Into<String>) -> Self {
        Self::Other {
            message: message.into(),
            source: None,
        }
    }

    /// Create a new error with a message and source
    pub fn with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Other {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Check if this error is retryable
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Network(_) | Self::Timeout(_) | Self::RateLimit | Self::Provider(_)
        )
    }

    /// Check if this error is a client error
    #[must_use]
    pub fn is_client_error(&self) -> bool {
        matches!(
            self,
            Self::InvalidInput(_)
                | Self::Validation(_)
                | Self::Authentication(_)
                | Self::Authorization(_)
                | Self::NotFound(_)
        )
    }

    /// Check if this error is a server error
    #[must_use]
    pub fn is_server_error(&self) -> bool {
        matches!(
            self,
            Self::Internal(_) | Self::Database(_) | Self::Cache(_) | Self::Initialization(_)
        )
    }

    /// Get the error code for API responses
    #[must_use]
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::Configuration(_) => "E001",
            Self::Validation(_) => "E002",
            Self::Pipeline(_) => "E003",
            Self::Context(_) => "E004",
            Self::Plugin(_) => "E005",
            Self::Provider(_) => "E006",
            Self::Network(_) => "E007",
            Self::Timeout(_) => "E008",
            Self::RateLimit => "E009",
            Self::Authentication(_) => "E010",
            Self::Authorization(_) => "E011",
            Self::NotFound(_) => "E012",
            Self::InvalidInput(_) => "E013",
            Self::Serialization(_) => "E014",
            Self::Database(_) => "E015",
            Self::Cache(_) => "E016",
            Self::Initialization(_) => "E017",
            Self::Internal(_) => "E018",
            Self::Other { .. } => "E999",
        }
    }

    /// Get the HTTP status code for this error
    #[must_use]
    pub fn http_status_code(&self) -> u16 {
        match self {
            Self::InvalidInput(_) | Self::Validation(_) => 400,
            Self::Authentication(_) => 401,
            Self::Authorization(_) => 403,
            Self::NotFound(_) => 404,
            Self::Timeout(_) => 408,
            Self::RateLimit => 429,
            Self::Internal(_) | Self::Database(_) | Self::Cache(_) => 500,
            Self::Network(_) | Self::Provider(_) => 502,
            Self::Initialization(_) => 503,
            _ => 500,
        }
    }
}

/// Result type alias using our Error type
pub type Result<T> = std::result::Result<T, Error>;

/// Extension trait for converting errors with context
pub trait ErrorContext<T> {
    /// Add context to an error
    fn context(self, msg: impl fmt::Display) -> Result<T>;

    /// Add context with a closure
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String;
}

impl<T, E> ErrorContext<T> for std::result::Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn context(self, msg: impl fmt::Display) -> Result<T> {
        self.map_err(|e| Error::with_source(msg.to_string(), e))
    }

    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| Error::with_source(f(), e))
    }
}

/// Error response for API
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ErrorResponse {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Additional details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// Request ID for tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl From<Error> for ErrorResponse {
    fn from(error: Error) -> Self {
        Self {
            code: error.error_code().to_string(),
            message: error.to_string(),
            details: None,
            request_id: None,
        }
    }
}

impl ErrorResponse {
    /// Create a new error response
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            request_id: None,
        }
    }

    /// Add details to the error response
    #[must_use]
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Add request ID for tracing
    #[must_use]
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as StdError;

    #[test]
    fn test_error_creation() {
        let error = Error::new("test error");
        assert_eq!(error.to_string(), "test error");
        assert_eq!(error.error_code(), "E999");
    }

    #[test]
    fn test_error_with_source() {
        let source = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let error = Error::with_source("wrapper error", source);
        assert_eq!(error.to_string(), "wrapper error");
        assert!(StdError::source(&error).is_some());
    }

    #[test]
    fn test_retryable_errors() {
        assert!(Error::Network("network error".into()).is_retryable());
        assert!(Error::Timeout(std::time::Duration::from_secs(30)).is_retryable());
        assert!(Error::RateLimit.is_retryable());
        assert!(Error::Provider("provider error".into()).is_retryable());

        assert!(!Error::InvalidInput("bad input".into()).is_retryable());
        assert!(!Error::Authentication("auth failed".into()).is_retryable());
    }

    #[test]
    fn test_client_errors() {
        assert!(Error::InvalidInput("bad input".into()).is_client_error());
        assert!(Error::Validation("validation failed".into()).is_client_error());
        assert!(Error::Authentication("auth failed".into()).is_client_error());
        assert!(Error::Authorization("not authorized".into()).is_client_error());
        assert!(Error::NotFound("not found".into()).is_client_error());

        assert!(!Error::Internal("internal error".into()).is_client_error());
        assert!(!Error::Database("db error".into()).is_client_error());
    }

    #[test]
    fn test_server_errors() {
        assert!(Error::Internal("internal error".into()).is_server_error());
        assert!(Error::Database("db error".into()).is_server_error());
        assert!(Error::Cache("cache error".into()).is_server_error());
        assert!(Error::Initialization("init failed".into()).is_server_error());

        assert!(!Error::InvalidInput("bad input".into()).is_server_error());
        assert!(!Error::Authentication("auth failed".into()).is_server_error());
    }

    #[test]
    fn test_http_status_codes() {
        assert_eq!(Error::InvalidInput("bad".into()).http_status_code(), 400);
        assert_eq!(Error::Validation("bad".into()).http_status_code(), 400);
        assert_eq!(Error::Authentication("auth".into()).http_status_code(), 401);
        assert_eq!(Error::Authorization("authz".into()).http_status_code(), 403);
        assert_eq!(Error::NotFound("404".into()).http_status_code(), 404);
        assert_eq!(
            Error::Timeout(std::time::Duration::from_secs(30)).http_status_code(),
            408
        );
        assert_eq!(Error::RateLimit.http_status_code(), 429);
        assert_eq!(Error::Internal("500".into()).http_status_code(), 500);
        assert_eq!(Error::Network("net".into()).http_status_code(), 502);
        assert_eq!(Error::Initialization("init".into()).http_status_code(), 503);
    }

    #[test]
    fn test_error_response() {
        let error = Error::InvalidInput("bad input".into());
        let response = ErrorResponse::from(error);

        assert_eq!(response.code, "E013");
        assert_eq!(response.message, "Invalid input: bad input");
        assert!(response.details.is_none());
        assert!(response.request_id.is_none());
    }

    #[test]
    fn test_error_response_with_details() {
        let response = ErrorResponse::new("E001", "test error")
            .with_details(serde_json::json!({"field": "name"}))
            .with_request_id("req-123");

        assert_eq!(response.code, "E001");
        assert_eq!(response.message, "test error");
        assert!(response.details.is_some());
        assert_eq!(response.request_id.as_deref(), Some("req-123"));
    }

    #[test]
    fn test_error_context() {
        let result: std::result::Result<(), std::io::Error> = Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));

        let error = result.context("Failed to read file").unwrap_err();
        assert_eq!(error.to_string(), "Failed to read file");
    }
}
