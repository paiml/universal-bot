//! Configuration management for Universal Bot
//!
//! This module provides configuration structures and builders for the bot,
//! following the builder pattern for ergonomic configuration.

use std::time::Duration;

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

use crate::error::Error;

/// Main bot configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct BotConfig {
    /// AI model to use for generation
    #[validate(custom(function = "validate_model"))]
    pub model: String,

    /// Temperature for generation (0.0 to 1.0)
    #[validate(range(min = 0.0, max = 1.0))]
    pub temperature: f32,

    /// Maximum tokens to generate
    #[validate(range(min = 1, max = 100000))]
    pub max_tokens: usize,

    /// Request timeout
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,

    /// Number of retries for failed requests
    #[validate(range(min = 0, max = 10))]
    pub max_retries: u32,

    /// Enable request logging
    pub enable_logging: bool,

    /// Enable cost tracking
    pub enable_cost_tracking: bool,

    /// Context configuration
    pub context_config: ContextConfig,

    /// Pipeline configuration
    pub pipeline_config: PipelineConfig,

    /// Plugin configuration
    pub plugin_config: PluginConfig,
}

impl BotConfig {
    /// Create a new configuration builder
    #[must_use]
    pub fn builder() -> BotConfigBuilder {
        BotConfigBuilder::default()
    }

    /// Validate the configuration
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails.
    pub fn validate(&self) -> Result<()> {
        Validate::validate(self).map_err(|e| Error::Validation(e.to_string()))?;
        Ok(())
    }

    /// Load configuration from environment variables
    ///
    /// # Errors
    ///
    /// Returns an error if required environment variables are missing.
    pub fn from_env() -> Result<Self> {
        let model = std::env::var("DEFAULT_MODEL")
            .unwrap_or_else(|_| "anthropic.claude-opus-4-1".to_string());

        let temperature = std::env::var("TEMPERATURE")
            .unwrap_or_else(|_| "0.1".to_string())
            .parse()
            .context("Invalid TEMPERATURE value")?;

        let max_tokens = std::env::var("MAX_TOKENS")
            .unwrap_or_else(|_| "2048".to_string())
            .parse()
            .context("Invalid MAX_TOKENS value")?;

        Ok(Self {
            model,
            temperature,
            max_tokens,
            ..Default::default()
        })
    }
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            model: "anthropic.claude-opus-4-1".to_string(),
            temperature: 0.1,
            max_tokens: 2048,
            timeout: Duration::from_secs(30),
            max_retries: 3,
            enable_logging: true,
            enable_cost_tracking: true,
            context_config: ContextConfig::default(),
            pipeline_config: PipelineConfig::default(),
            plugin_config: PluginConfig::default(),
        }
    }
}

/// Configuration for context management
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ContextConfig {
    /// Maximum context size in tokens
    #[validate(range(min = 100, max = 100000))]
    pub max_context_tokens: usize,

    /// Context TTL
    #[serde(with = "humantime_serde")]
    pub context_ttl: Duration,

    /// Enable context persistence
    pub persist_context: bool,

    /// Context storage backend
    pub storage_backend: StorageBackend,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: 4096,
            context_ttl: Duration::from_secs(3600),
            persist_context: false,
            storage_backend: StorageBackend::Memory,
        }
    }
}

/// Storage backend for context persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    /// In-memory storage (default)
    Memory,
    /// Redis storage
    Redis {
        /// Redis connection URL
        url: String,
    },
    /// PostgreSQL storage
    Postgres {
        /// PostgreSQL connection URL
        url: String,
    },
    /// SQLite storage
    Sqlite {
        /// SQLite database file path
        path: String,
    },
}

/// Configuration for the message pipeline
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PipelineConfig {
    /// Enable message sanitization
    pub enable_sanitization: bool,

    /// Enable message enrichment
    pub enable_enrichment: bool,

    /// Maximum pipeline processing time
    #[serde(with = "humantime_serde")]
    pub max_processing_time: Duration,

    /// Pipeline stages to enable
    pub enabled_stages: Vec<String>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            enable_sanitization: true,
            enable_enrichment: true,
            max_processing_time: Duration::from_secs(10),
            enabled_stages: vec![
                "sanitize".to_string(),
                "enrich".to_string(),
                "route".to_string(),
                "process".to_string(),
                "format".to_string(),
            ],
        }
    }
}

/// Configuration for plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Enable plugin system
    pub enable_plugins: bool,

    /// Plugin directories to scan
    pub plugin_dirs: Vec<String>,

    /// Plugins to auto-load
    pub auto_load: Vec<String>,

    /// Plugin timeout
    #[serde(with = "humantime_serde")]
    pub plugin_timeout: Duration,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enable_plugins: true,
            plugin_dirs: vec!["plugins".to_string()],
            auto_load: Vec::new(),
            plugin_timeout: Duration::from_secs(5),
        }
    }
}

/// Builder for `BotConfig`
#[derive(Default)]
pub struct BotConfigBuilder {
    model: Option<String>,
    temperature: Option<f32>,
    max_tokens: Option<usize>,
    timeout: Option<Duration>,
    max_retries: Option<u32>,
    enable_logging: Option<bool>,
    enable_cost_tracking: Option<bool>,
    context_config: Option<ContextConfig>,
    pipeline_config: Option<PipelineConfig>,
    plugin_config: Option<PluginConfig>,
}

impl BotConfigBuilder {
    /// Set the AI model
    #[must_use]
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the temperature
    #[must_use]
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set the maximum tokens
    #[must_use]
    pub fn max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set the timeout
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set the maximum retries
    #[must_use]
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = Some(max_retries);
        self
    }

    /// Enable or disable logging
    #[must_use]
    pub fn enable_logging(mut self, enable: bool) -> Self {
        self.enable_logging = Some(enable);
        self
    }

    /// Enable or disable cost tracking
    #[must_use]
    pub fn enable_cost_tracking(mut self, enable: bool) -> Self {
        self.enable_cost_tracking = Some(enable);
        self
    }

    /// Set the context configuration
    #[must_use]
    pub fn context_config(mut self, config: ContextConfig) -> Self {
        self.context_config = Some(config);
        self
    }

    /// Set the pipeline configuration
    #[must_use]
    pub fn pipeline_config(mut self, config: PipelineConfig) -> Self {
        self.pipeline_config = Some(config);
        self
    }

    /// Set the plugin configuration
    #[must_use]
    pub fn plugin_config(mut self, config: PluginConfig) -> Self {
        self.plugin_config = Some(config);
        self
    }

    /// Build the configuration
    ///
    /// # Errors
    ///
    /// Returns an error if required fields are missing or validation fails.
    pub fn build(self) -> Result<BotConfig> {
        let config = BotConfig {
            model: self.model.context("model is required")?,
            temperature: self.temperature.unwrap_or(0.1),
            max_tokens: self.max_tokens.unwrap_or(2048),
            timeout: self.timeout.unwrap_or(Duration::from_secs(30)),
            max_retries: self.max_retries.unwrap_or(3),
            enable_logging: self.enable_logging.unwrap_or(true),
            enable_cost_tracking: self.enable_cost_tracking.unwrap_or(true),
            context_config: self.context_config.unwrap_or_default(),
            pipeline_config: self.pipeline_config.unwrap_or_default(),
            plugin_config: self.plugin_config.unwrap_or_default(),
        };

        config.validate()?;
        Ok(config)
    }
}

/// Validate model name
fn validate_model(model: &str) -> Result<(), ValidationError> {
    const ALLOWED_MODELS: &[&str] = &[
        "anthropic.claude-opus-4-1",
        "us.anthropic.claude-opus-4-1-20250805-v1:0", // Opus 4.1 inference profile
        "anthropic.claude-sonnet-4",
        "anthropic.claude-haiku",
        "meta.llama3-70b-instruct",
        "meta.llama3-8b-instruct",
        "amazon.titan-text-express",
        "ai21.j2-ultra",
        "ai21.j2-mid",
    ];

    if ALLOWED_MODELS.contains(&model) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_model"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = BotConfig::default();
        assert_eq!(config.model, "anthropic.claude-opus-4-1");
        assert_eq!(config.temperature, 0.1);
        assert_eq!(config.max_tokens, 2048);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_builder() {
        let config = BotConfig::builder()
            .model("anthropic.claude-sonnet-4")
            .temperature(0.5)
            .max_tokens(4096)
            .timeout(Duration::from_secs(60))
            .build();

        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.model, "anthropic.claude-sonnet-4");
        assert_eq!(config.temperature, 0.5);
        assert_eq!(config.max_tokens, 4096);
    }

    #[test]
    fn test_invalid_model() {
        let config = BotConfig::builder().model("invalid-model").build();

        assert!(config.is_err());
    }

    #[test]
    fn test_invalid_temperature() {
        let mut config = BotConfig::default();
        config.temperature = 1.5;
        assert!(config.validate().is_err());

        config.temperature = -0.1;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_from_env() {
        std::env::set_var("DEFAULT_MODEL", "anthropic.claude-opus-4-1");
        std::env::set_var("TEMPERATURE", "0.7");
        std::env::set_var("MAX_TOKENS", "4096");

        let config = BotConfig::from_env();
        assert!(config.is_ok());

        let config = config.unwrap();
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.max_tokens, 4096);
    }

    #[cfg(feature = "property-testing")]
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn test_temperature_validation(temp in -10.0f32..10.0) {
                let mut config = BotConfig::default();
                config.temperature = temp;

                let result = config.validate();
                if (0.0..=1.0).contains(&temp) {
                    prop_assert!(result.is_ok());
                } else {
                    prop_assert!(result.is_err());
                }
            }

            #[test]
            fn test_max_tokens_validation(tokens in 0usize..200_000) {
                let mut config = BotConfig::default();
                config.max_tokens = tokens;

                let result = config.validate();
                if tokens > 0 && tokens <= 100_000 {
                    prop_assert!(result.is_ok());
                } else {
                    prop_assert!(result.is_err());
                }
            }
        }
    }
}
