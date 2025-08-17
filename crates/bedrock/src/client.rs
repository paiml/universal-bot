//! High-level Bedrock client interface

use async_trait::async_trait;
use std::collections::HashMap;

use crate::config::GenerationConfig;
use crate::error::Result;
use crate::message::{GenerationResponse, StreamChunk, UniversalMessage};
use crate::metrics::HealthStatus;

/// High-level trait for Bedrock clients
#[async_trait]
pub trait BedrockClient: Send + Sync {
    /// Generate text using the specified model
    async fn generate_text(
        &self,
        model: &str,
        messages: Vec<UniversalMessage>,
        config: Option<GenerationConfig>,
    ) -> Result<GenerationResponse>;

    /// Stream text generation
    async fn stream_text(
        &self,
        model: &str,
        messages: Vec<UniversalMessage>,
        config: Option<GenerationConfig>,
    ) -> Result<Box<dyn futures::Stream<Item = Result<StreamChunk>> + Send + Unpin>>;

    /// Health check
    async fn health_check(&self) -> Result<HealthStatus>;

    /// List available models
    async fn list_models(&self) -> Result<Vec<String>>;
}

/// Mock client for testing
#[cfg(feature = "mock-client")]
pub struct MockBedrockClient {
    responses: HashMap<String, String>,
}

#[cfg(feature = "mock-client")]
impl MockBedrockClient {
    /// Create a new mock client
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
        }
    }

    /// Add a mock response for a model
    pub fn add_response(&mut self, model: &str, response: &str) {
        self.responses
            .insert(model.to_string(), response.to_string());
    }
}

#[cfg(feature = "mock-client")]
#[async_trait]
impl BedrockClient for MockBedrockClient {
    async fn generate_text(
        &self,
        model: &str,
        _messages: Vec<UniversalMessage>,
        _config: Option<GenerationConfig>,
    ) -> Result<GenerationResponse> {
        let content = self
            .responses
            .get(model)
            .cloned()
            .unwrap_or_else(|| format!("Mock response from {}", model));

        Ok(GenerationResponse {
            id: uuid::Uuid::new_v4(),
            content,
            model: model.to_string(),
            usage: None,
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now(),
            finish_reason: "stop".to_string(),
        })
    }

    async fn stream_text(
        &self,
        _model: &str,
        _messages: Vec<UniversalMessage>,
        _config: Option<GenerationConfig>,
    ) -> Result<Box<dyn futures::Stream<Item = Result<StreamChunk>> + Send + Unpin>> {
        use futures::stream;

        let chunks = vec![
            StreamChunk::content("Mock"),
            StreamChunk::content(" streaming"),
            StreamChunk::content(" response"),
        ];

        let stream = stream::iter(chunks.into_iter().map(Ok));
        Ok(Box::new(stream))
    }

    async fn health_check(&self) -> Result<HealthStatus> {
        Ok(HealthStatus {
            healthy: true,
            latency_ms: 1,
            error: None,
            timestamp: chrono::Utc::now(),
        })
    }

    async fn list_models(&self) -> Result<Vec<String>> {
        Ok(vec!["mock-model-1".to_string(), "mock-model-2".to_string()])
    }
}

#[cfg(feature = "mock-client")]
impl Default for MockBedrockClient {
    fn default() -> Self {
        Self::new()
    }
}
