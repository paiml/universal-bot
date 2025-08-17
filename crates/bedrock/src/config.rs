//! Configuration for AWS Bedrock client

use std::time::Duration;

use aws_sdk_bedrockruntime::config::Region;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Configuration for the Bedrock client
#[derive(Debug, Clone, Serialize, Validate)]
pub struct BedrockConfig {
    /// AWS region
    #[serde(skip)]
    pub region: Region,

    /// Number of clients in the connection pool
    #[validate(range(min = 1, max = 100))]
    pub pool_size: usize,

    /// Request timeout in seconds
    #[validate(range(min = 1, max = 300))]
    pub timeout_seconds: u64,

    /// Initial retry interval in milliseconds
    #[validate(range(min = 100, max = 10000))]
    pub retry_initial_interval_ms: u64,

    /// Maximum retry interval in seconds
    #[validate(range(min = 1, max = 60))]
    pub retry_max_interval_seconds: u64,

    /// Maximum total retry time in seconds
    #[validate(range(min = 10, max = 300))]
    pub retry_max_elapsed_seconds: u64,

    /// Retry backoff multiplier
    #[validate(range(min = 1.0, max = 5.0))]
    pub retry_multiplier: f64,

    /// Maximum concurrent requests per client
    #[validate(range(min = 1, max = 1000))]
    pub max_concurrent_requests: usize,

    /// Enable detailed metrics collection
    pub enable_metrics: bool,

    /// Enable request/response logging
    pub enable_logging: bool,
}

impl Default for BedrockConfig {
    fn default() -> Self {
        Self {
            region: Region::new("us-east-1"),
            pool_size: 5,
            timeout_seconds: 120,
            retry_initial_interval_ms: 500,
            retry_max_interval_seconds: 30,
            retry_max_elapsed_seconds: 300,
            retry_multiplier: 2.0,
            max_concurrent_requests: 100,
            enable_metrics: true,
            enable_logging: false,
        }
    }
}

impl BedrockConfig {
    /// Create a new configuration with custom region
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Region::new(region.into());
        self
    }

    /// Set the connection pool size
    pub fn with_pool_size(mut self, size: usize) -> Self {
        self.pool_size = size;
        self
    }

    /// Set the request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout_seconds = timeout.as_secs();
        self
    }

    /// Enable or disable metrics collection
    pub fn with_metrics(mut self, enable: bool) -> Self {
        self.enable_metrics = enable;
        self
    }

    /// Enable or disable request logging
    pub fn with_logging(mut self, enable: bool) -> Self {
        self.enable_logging = enable;
        self
    }

    /// Create a high-performance configuration
    pub fn high_performance() -> Self {
        Self {
            pool_size: 20,
            max_concurrent_requests: 500,
            timeout_seconds: 60,
            retry_initial_interval_ms: 200,
            retry_max_interval_seconds: 10,
            retry_multiplier: 1.5,
            ..Default::default()
        }
    }

    /// Create a low-latency configuration
    pub fn low_latency() -> Self {
        Self {
            pool_size: 10,
            timeout_seconds: 30,
            retry_initial_interval_ms: 100,
            retry_max_interval_seconds: 5,
            retry_max_elapsed_seconds: 60,
            retry_multiplier: 1.2,
            ..Default::default()
        }
    }

    /// Create a conservative configuration for low-volume usage
    pub fn conservative() -> Self {
        Self {
            pool_size: 2,
            max_concurrent_requests: 10,
            timeout_seconds: 180,
            retry_initial_interval_ms: 1000,
            retry_max_interval_seconds: 60,
            retry_max_elapsed_seconds: 600,
            retry_multiplier: 3.0,
            ..Default::default()
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), validator::ValidationErrors> {
        validator::Validate::validate(self)
    }
}

/// Generation configuration for inference requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConfig {
    /// Maximum number of tokens to generate
    pub max_tokens: Option<usize>,

    /// Temperature for randomness (0.0-1.0)
    pub temperature: Option<f32>,

    /// Top-p for nucleus sampling
    pub top_p: Option<f32>,

    /// System prompt
    pub system_prompt: Option<String>,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            max_tokens: Some(4096),
            temperature: Some(0.7),
            top_p: Some(0.9),
            system_prompt: None,
        }
    }
}

impl GenerationConfig {
    /// Create a configuration optimized for code generation
    pub fn code_generation() -> Self {
        Self {
            max_tokens: Some(8192),
            temperature: Some(0.1),
            top_p: Some(0.95),
            system_prompt: Some(
                "You are an expert programmer. Provide clean, efficient, and well-documented code."
                    .to_string(),
            ),
        }
    }

    /// Create a configuration optimized for creative writing
    pub fn creative_writing() -> Self {
        Self {
            max_tokens: Some(4096),
            temperature: Some(0.9),
            top_p: Some(0.9),
            system_prompt: Some(
                "You are a creative writer. Be imaginative and engaging.".to_string(),
            ),
        }
    }

    /// Create a configuration optimized for analysis tasks
    pub fn analysis() -> Self {
        Self {
            max_tokens: Some(4096),
            temperature: Some(0.3),
            top_p: Some(0.95),
            system_prompt: Some(
                "You are an expert analyst. Provide thorough, objective analysis.".to_string(),
            ),
        }
    }

    /// Create a deterministic configuration (temperature = 0)
    pub fn deterministic() -> Self {
        Self {
            max_tokens: Some(4096),
            temperature: Some(0.0),
            top_p: Some(1.0),
            system_prompt: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_validation() {
        let config = BedrockConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_high_performance_config() {
        let config = BedrockConfig::high_performance();
        assert_eq!(config.pool_size, 20);
        assert_eq!(config.max_concurrent_requests, 500);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_generation_config_presets() {
        let code_config = GenerationConfig::code_generation();
        assert_eq!(code_config.temperature, Some(0.1));

        let creative_config = GenerationConfig::creative_writing();
        assert_eq!(creative_config.temperature, Some(0.9));

        let deterministic_config = GenerationConfig::deterministic();
        assert_eq!(deterministic_config.temperature, Some(0.0));
    }

    #[test]
    fn test_config_builder_pattern() {
        let config = BedrockConfig::default()
            .with_region("us-west-2")
            .with_pool_size(10)
            .with_metrics(false);

        assert_eq!(config.region.as_ref(), "us-west-2");
        assert_eq!(config.pool_size, 10);
        assert!(!config.enable_metrics);
    }
}
