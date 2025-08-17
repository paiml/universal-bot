//! AWS Bedrock client for Universal Bot
//!
//! This crate provides production-ready AWS Bedrock Runtime integration
//! with connection pooling, retry logic, and model orchestration.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_bedrockruntime::types::{ContentBlock, Message as BedrockMessage, SystemContentBlock};
use aws_sdk_bedrockruntime::{Client as BedrockClient, Config};
use backoff::{ExponentialBackoff, ExponentialBackoffBuilder};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::Stream;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

pub use client::*;
pub use config::*;
pub use error::{BedrockError, ErrorCategory, Result};
pub use message::*;
pub use metrics::*;
pub use model::*;
pub use pool::*;
pub use retry::*;
pub use streaming::*;

mod client;
mod config;
mod error;
mod message;
mod metrics;
mod model;
mod pool;
mod retry;
mod streaming;

/// Re-export commonly used types
pub use aws_sdk_bedrockruntime::types::{ContentBlock as AwsContentBlock, Message as AwsMessage};

/// Default model configurations
pub const DEFAULT_CLAUDE_MODEL: &str = "anthropic.claude-3-5-sonnet-20241022-v2:0";
pub const DEFAULT_OPUS_MODEL: &str = "anthropic.claude-3-opus-20240229-v1:0";
pub const DEFAULT_HAIKU_MODEL: &str = "anthropic.claude-3-haiku-20240307-v1:0";

/// Universal Bot Bedrock client
#[derive(Clone)]
pub struct UniversalBedrockClient {
    inner: Arc<BedrockClientInner>,
}

struct BedrockClientInner {
    clients: Vec<BedrockClient>,
    config: BedrockConfig,
    metrics: Arc<RwLock<BedrockMetrics>>,
    semaphore: Semaphore,
    retry_policy: ExponentialBackoff,
}

impl UniversalBedrockClient {
    /// Create a new Bedrock client with default configuration
    ///
    /// # Errors
    ///
    /// Returns an error if AWS configuration cannot be loaded.
    pub async fn new() -> Result<Self> {
        let config = BedrockConfig::default();
        Self::with_config(config).await
    }

    /// Create a new Bedrock client with custom configuration
    ///
    /// # Errors
    ///
    /// Returns an error if AWS configuration cannot be loaded or client pool cannot be created.
    pub async fn with_config(config: BedrockConfig) -> Result<Self> {
        info!(
            "Initializing Universal Bedrock client with {} connections",
            config.pool_size
        );

        let aws_config = aws_config::defaults(BehaviorVersion::latest())
            .region(config.region.clone())
            .load()
            .await;

        let mut clients = Vec::with_capacity(config.pool_size);
        for _ in 0..config.pool_size {
            let client_config = Config::builder()
                .region(config.region.clone())
                .timeout_config(
                    aws_sdk_bedrockruntime::config::timeout::TimeoutConfig::builder()
                        .operation_timeout(Duration::from_secs(config.timeout_seconds))
                        .build(),
                )
                .build();

            let client = BedrockClient::from_conf(client_config);
            clients.push(client);
        }

        let retry_policy = ExponentialBackoffBuilder::new()
            .with_initial_interval(Duration::from_millis(config.retry_initial_interval_ms))
            .with_max_interval(Duration::from_secs(config.retry_max_interval_seconds))
            .with_max_elapsed_time(Some(Duration::from_secs(config.retry_max_elapsed_seconds)))
            .with_multiplier(config.retry_multiplier)
            .build();

        let pool_size = config.pool_size;
        let inner = BedrockClientInner {
            clients,
            config,
            metrics: Arc::new(RwLock::new(BedrockMetrics::new())),
            semaphore: Semaphore::new(pool_size),
            retry_policy,
        };

        info!("Universal Bedrock client initialized successfully");
        Ok(Self {
            inner: Arc::new(inner),
        })
    }

    /// Generate a text response using the specified model
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or times out.
    #[instrument(skip(self, messages), fields(model = %model, message_count = messages.len()))]
    pub async fn generate_text(
        &self,
        model: &str,
        messages: Vec<UniversalMessage>,
        config: Option<GenerationConfig>,
    ) -> Result<GenerationResponse> {
        let start = std::time::Instant::now();
        let request_id = Uuid::new_v4();

        debug!("Starting text generation request {}", request_id);

        // Update metrics
        {
            let mut metrics = self.inner.metrics.write();
            metrics.total_requests += 1;
            metrics.active_requests += 1;
        }

        let result = self
            ._generate_text_with_retry(model, messages, config, request_id)
            .await;

        // Update metrics
        {
            let mut metrics = self.inner.metrics.write();
            metrics.active_requests -= 1;
            match &result {
                Ok(_) => metrics.successful_requests += 1,
                Err(_) => metrics.failed_requests += 1,
            }
            metrics.total_latency_ms += start.elapsed().as_millis() as u64;
        }

        result
    }

    async fn _generate_text_with_retry(
        &self,
        model: &str,
        messages: Vec<UniversalMessage>,
        config: Option<GenerationConfig>,
        request_id: Uuid,
    ) -> Result<GenerationResponse> {
        let operation = || async {
            self._generate_text_once(model, &messages, &config, request_id)
                .await
        };

        backoff::future::retry(self.inner.retry_policy.clone(), operation)
            .await
            .map_err(|e| BedrockError::RequestFailed(format!("All retries exhausted: {}", e)))
            .context("Failed to generate text after retries")
    }

    async fn _generate_text_once(
        &self,
        model: &str,
        messages: &[UniversalMessage],
        config: &Option<GenerationConfig>,
        request_id: Uuid,
    ) -> Result<GenerationResponse, backoff::Error<BedrockError>> {
        let _permit =
            self.inner.semaphore.acquire().await.map_err(|e| {
                backoff::Error::permanent(BedrockError::PoolExhausted(e.to_string()))
            })?;

        // Get a client from the pool
        let client_index = request_id.as_u128() as usize % self.inner.clients.len();
        let client = &self.inner.clients[client_index];

        // Convert messages to Bedrock format
        let bedrock_messages = messages
            .iter()
            .map(|msg| msg.to_bedrock_message())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| backoff::Error::permanent(BedrockError::InvalidInput(e.to_string())))?;

        // Build the request
        let mut request = client
            .converse()
            .model_id(model)
            .set_messages(Some(bedrock_messages));

        // Apply generation config
        if let Some(config) = config {
            let inference_config = aws_sdk_bedrockruntime::types::InferenceConfiguration::builder()
                .set_max_tokens(config.max_tokens.map(|t| t as i32))
                .set_temperature(config.temperature)
                .set_top_p(config.top_p)
                .build();
            request = request.inference_config(inference_config);

            if let Some(system) = &config.system_prompt {
                let system_block = SystemContentBlock::Text(system.clone());
                request = request.system(vec![system_block]);
            }
        }

        debug!("Sending request {} to model {}", request_id, model);

        // Execute the request
        let response = request.send().await.map_err(|e| {
            warn!("Request {} failed: {}", request_id, e);
            if e.as_service_error().is_some() {
                backoff::Error::transient(BedrockError::ServiceError(e.to_string()))
            } else {
                backoff::Error::permanent(BedrockError::RequestFailed(e.to_string()))
            }
        })?;

        debug!("Request {} completed successfully", request_id);

        // Parse response
        let content = response
            .output()
            .as_ref()
            .and_then(|output| output.as_message())
            .and_then(|msg| msg.content().first())
            .and_then(|block| block.as_text())
            .ok_or_else(|| {
                backoff::Error::permanent(BedrockError::InvalidResponse(
                    "No text content in response".to_string(),
                ))
            })?;

        let usage = response.usage().map(|u| TokenUsage {
            input_tokens: u.input_tokens() as usize,
            output_tokens: u.output_tokens() as usize,
            total_tokens: u.total_tokens() as usize,
            estimated_cost: calculate_cost(
                u.input_tokens() as usize,
                u.output_tokens() as usize,
                model,
            ),
            model: model.to_string(),
        });

        Ok(GenerationResponse {
            id: request_id,
            content: content.to_string(),
            model: model.to_string(),
            usage,
            metadata: HashMap::new(),
            timestamp: Utc::now(),
            finish_reason: response
                .stop_reason()
                .map(|r| r.as_str().to_string())
                .unwrap_or_else(|| "unknown".to_string()),
        })
    }

    /// Stream a text response using the specified model
    ///
    /// # Errors
    ///
    /// Returns an error if the streaming request fails to start.
    pub async fn stream_text(
        &self,
        model: &str,
        messages: Vec<UniversalMessage>,
        config: Option<GenerationConfig>,
    ) -> Result<impl Stream<Item = Result<StreamChunk>>> {
        let _permit = self
            .inner
            .semaphore
            .acquire()
            .await
            .context("Failed to acquire semaphore permit")?;

        let client_index = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos() as usize
            % self.inner.clients.len();
        let client = &self.inner.clients[client_index];

        // Convert messages to Bedrock format
        let bedrock_messages = messages
            .iter()
            .map(|msg| msg.to_bedrock_message())
            .collect::<Result<Vec<_>, _>>()?;

        // Build the request
        let mut request = client
            .converse_stream()
            .model_id(model)
            .set_messages(Some(bedrock_messages));

        // Apply generation config
        if let Some(config) = &config {
            let inference_config = aws_sdk_bedrockruntime::types::InferenceConfiguration::builder()
                .set_max_tokens(config.max_tokens.map(|t| t as i32))
                .set_temperature(config.temperature)
                .set_top_p(config.top_p)
                .build();
            request = request.inference_config(inference_config);

            if let Some(system) = &config.system_prompt {
                let system_block = SystemContentBlock::Text(system.clone());
                request = request.system(vec![system_block]);
            }
        }

        let response = request
            .send()
            .await
            .context("Failed to start streaming request")?;

        Ok(StreamingResponse::new(response.stream, model.to_string()))
    }

    /// Get current client metrics
    pub fn metrics(&self) -> BedrockMetrics {
        self.inner.metrics.read().clone()
    }

    /// Get client configuration
    pub fn config(&self) -> &BedrockConfig {
        &self.inner.config
    }

    /// Health check for the client
    ///
    /// # Errors
    ///
    /// Returns an error if the health check fails.
    pub async fn health_check(&self) -> Result<HealthStatus> {
        let start = std::time::Instant::now();

        // Try a simple request to check connectivity
        let test_message = UniversalMessage {
            role: MessageRole::User,
            content: "Hello".to_string(),
            metadata: HashMap::new(),
        };

        let config = GenerationConfig {
            max_tokens: Some(1),
            temperature: Some(0.0),
            top_p: None,
            system_prompt: None,
        };

        match self
            .generate_text(DEFAULT_HAIKU_MODEL, vec![test_message], Some(config))
            .await
        {
            Ok(_) => Ok(HealthStatus {
                healthy: true,
                latency_ms: start.elapsed().as_millis() as u64,
                error: None,
                timestamp: Utc::now(),
            }),
            Err(e) => Ok(HealthStatus {
                healthy: false,
                latency_ms: start.elapsed().as_millis() as u64,
                error: Some(e.to_string()),
                timestamp: Utc::now(),
            }),
        }
    }
}

/// Calculate estimated cost for token usage
fn calculate_cost(input_tokens: usize, output_tokens: usize, model: &str) -> f64 {
    // Cost per 1K tokens (example rates, update with actual pricing)
    let (input_rate, output_rate) = match model {
        m if m.contains("claude-3-opus") => (0.015, 0.075),
        m if m.contains("claude-3-5-sonnet") => (0.003, 0.015),
        m if m.contains("claude-3-haiku") => (0.00025, 0.00125),
        _ => (0.001, 0.002), // Default rates
    };

    (input_tokens as f64 / 1000.0 * input_rate) + (output_tokens as f64 / 1000.0 * output_rate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_calculation() {
        let cost = calculate_cost(1000, 500, "anthropic.claude-3-5-sonnet-20241022-v2:0");
        assert!(cost > 0.0);
        assert!(cost < 1.0); // Reasonable bounds
    }

    #[tokio::test]
    async fn test_client_creation() {
        let config = BedrockConfig::default();
        // This test would need AWS credentials to actually work
        // In a real test, we'd use mocking
        assert!(config.pool_size > 0);
    }

    #[test]
    fn test_message_conversion() {
        let msg = UniversalMessage {
            role: MessageRole::User,
            content: "Test message".to_string(),
            metadata: HashMap::new(),
        };

        let bedrock_msg = msg.to_bedrock_message().unwrap();
        // Verify the conversion worked
        assert!(!bedrock_msg.content().is_empty());
    }
}
