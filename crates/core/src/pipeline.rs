//! Message processing pipeline
//!
//! This module implements the message processing pipeline that handles
//! sanitization, enrichment, routing, processing, and formatting of messages.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context as _, Result};
use async_trait::async_trait;
use parking_lot::RwLock;
use tracing::{debug, instrument};

use crate::{
    config::{BotConfig, PipelineConfig},
    context::Context,
    error::Error,
    message::{Message, Response},
};

/// Message processing pipeline
pub struct MessagePipeline {
    #[allow(dead_code)]
    config: PipelineConfig,
    stages: Vec<Box<dyn PipelineStage>>,
    middleware: Vec<Box<dyn PipelineMiddleware>>,
    metrics: Arc<PipelineMetrics>,
}

impl MessagePipeline {
    /// Create a new message pipeline
    ///
    /// # Errors
    ///
    /// Returns an error if pipeline initialization fails.
    #[instrument(skip(config))]
    pub async fn new(config: &BotConfig) -> Result<Self> {
        debug!("Creating message pipeline");

        let mut stages: Vec<Box<dyn PipelineStage>> = Vec::new();

        // Add stages based on configuration
        for stage_name in &config.pipeline_config.enabled_stages {
            let stage = Self::create_stage(stage_name, config)?;
            stages.push(stage);
        }

        // Add default middleware
        let middleware = vec![
            Box::new(LoggingMiddleware::new()) as Box<dyn PipelineMiddleware>,
            Box::new(MetricsMiddleware::new()) as Box<dyn PipelineMiddleware>,
            Box::new(TimeoutMiddleware::new(
                config.pipeline_config.max_processing_time,
            )) as Box<dyn PipelineMiddleware>,
        ];

        Ok(Self {
            config: config.pipeline_config.clone(),
            stages,
            middleware,
            metrics: Arc::new(PipelineMetrics::new()),
        })
    }

    /// Process a message through the pipeline
    ///
    /// # Errors
    ///
    /// Returns an error if any stage in the pipeline fails
    #[instrument(skip(self, message, context))]
    pub async fn process(
        &self,
        mut message: Message,
        context: Arc<RwLock<Context>>,
    ) -> Result<Response> {
        let start = std::time::Instant::now();
        self.metrics.increment_requests();

        // Apply middleware pre-processing
        for mw in &self.middleware {
            message = mw.before_pipeline(message).await?;
        }

        // Create pipeline context
        let mut pipeline_ctx = PipelineContext {
            message,
            context,
            metadata: HashMap::default(),
        };

        // Process through stages
        for stage in &self.stages {
            debug!("Processing stage: {}", stage.name());
            pipeline_ctx = stage.process(pipeline_ctx).await?;
        }

        // Generate response
        let mut response = self.generate_response(pipeline_ctx)?;

        // Apply middleware post-processing
        for mw in self.middleware.iter().rev() {
            response = mw.after_pipeline(response).await?;
        }

        // Record metrics
        let duration = start.elapsed();
        self.metrics.record_processing_time(duration);

        debug!("Pipeline processed in {:?}", duration);
        Ok(response)
    }

    /// Add a custom stage to the pipeline
    pub fn add_stage(&mut self, stage: Box<dyn PipelineStage>) {
        self.stages.push(stage);
    }

    /// Add middleware to the pipeline
    pub fn add_middleware(&mut self, middleware: Box<dyn PipelineMiddleware>) {
        self.middleware.push(middleware);
    }

    /// Get pipeline metrics
    #[must_use]
    pub fn metrics(&self) -> &PipelineMetrics {
        &self.metrics
    }

    // Private helper methods

    fn create_stage(name: &str, config: &BotConfig) -> Result<Box<dyn PipelineStage>> {
        match name {
            "sanitize" => Ok(Box::new(SanitizeStage::new())),
            "enrich" => Ok(Box::new(EnrichStage::new())),
            "route" => Ok(Box::new(RouteStage::new())),
            "process" => Ok(Box::new(ProcessStage::new(config.clone()))),
            "format" => Ok(Box::new(FormatStage::new())),
            _ => Err(Error::Configuration(format!("Unknown pipeline stage: {name}")).into()),
        }
    }

    #[allow(clippy::unused_self)]
    fn generate_response(&self, ctx: PipelineContext) -> Result<Response> {
        // Extract response from pipeline context
        if let Some(response) = ctx.metadata.get("response") {
            let response: Response = serde_json::from_value(response.clone())
                .context("Failed to deserialize response")?;
            Ok(response)
        } else {
            // Create default response if none was generated
            Ok(Response::text(
                ctx.message.conversation_id,
                "Message processed successfully",
            ))
        }
    }
}

/// Pipeline processing context
#[derive(Debug)]
pub struct PipelineContext {
    /// The message being processed
    pub message: Message,
    /// Conversation context
    pub context: Arc<RwLock<Context>>,
    /// Pipeline metadata
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

/// Trait for pipeline stages
#[async_trait]
pub trait PipelineStage: Send + Sync {
    /// Stage name
    fn name(&self) -> &str;

    /// Process the pipeline context
    async fn process(&self, ctx: PipelineContext) -> Result<PipelineContext>;
}

/// Trait for pipeline middleware
#[async_trait]
pub trait PipelineMiddleware: Send + Sync {
    /// Called before pipeline processing
    async fn before_pipeline(&self, message: Message) -> Result<Message> {
        Ok(message)
    }

    /// Called after pipeline processing
    async fn after_pipeline(&self, response: Response) -> Result<Response> {
        Ok(response)
    }
}

/// Sanitization stage - cleans and validates input
struct SanitizeStage;

impl SanitizeStage {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PipelineStage for SanitizeStage {
    fn name(&self) -> &str {
        "sanitize"
    }

    async fn process(&self, mut ctx: PipelineContext) -> Result<PipelineContext> {
        // Sanitize message content
        ctx.message.content = self.sanitize_content(&ctx.message.content);

        // Validate message
        ctx.message
            .validate()
            .context("Message validation failed")?;

        // Remove sensitive data from metadata
        self.sanitize_metadata(&mut ctx.message.metadata);

        Ok(ctx)
    }
}

impl SanitizeStage {
    #[allow(clippy::unused_self)]
    fn sanitize_content(&self, content: &str) -> String {
        // Remove control characters
        let sanitized = content
            .chars()
            .filter(|c| !c.is_control() || c.is_whitespace())
            .collect::<String>();

        // Trim excessive whitespace
        sanitized
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[allow(clippy::unused_self)]
    fn sanitize_metadata(
        &self,
        metadata: &mut std::collections::HashMap<String, serde_json::Value>,
    ) {
        // Remove potentially sensitive keys
        const SENSITIVE_KEYS: &[&str] = &["password", "token", "secret", "api_key", "auth"];

        metadata.retain(|key, _| {
            !SENSITIVE_KEYS
                .iter()
                .any(|&sensitive| key.to_lowercase().contains(sensitive))
        });
    }
}

/// Enrichment stage - adds context and metadata
struct EnrichStage;

impl EnrichStage {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PipelineStage for EnrichStage {
    fn name(&self) -> &str {
        "enrich"
    }

    async fn process(&self, mut ctx: PipelineContext) -> Result<PipelineContext> {
        // Add timestamp if not present
        ctx.metadata.insert(
            "processed_at".to_string(),
            serde_json::json!(chrono::Utc::now()),
        );

        // Add context summary
        let context_summary = {
            let context = ctx.context.read();
            serde_json::json!({
                "message_count": context.metadata.message_count,
                "token_count": context.token_count,
                "age_seconds": context.age().as_secs(),
            })
        };
        ctx.metadata
            .insert("context_summary".to_string(), context_summary);

        // Detect language if needed
        if !ctx.message.metadata.contains_key("language") {
            let language = self.detect_language(&ctx.message.content);
            ctx.message
                .metadata
                .insert("language".to_string(), serde_json::json!(language));
        }

        Ok(ctx)
    }
}

impl EnrichStage {
    #[allow(clippy::unused_self)]
    fn detect_language(&self, _content: &str) -> &str {
        // Simple language detection (would use a proper library in production)
        "en"
    }
}

/// Routing stage - determines processing path
struct RouteStage;

impl RouteStage {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PipelineStage for RouteStage {
    fn name(&self) -> &str {
        "route"
    }

    async fn process(&self, mut ctx: PipelineContext) -> Result<PipelineContext> {
        use crate::message::MessageType;

        // Determine route based on message type
        let route = match ctx.message.message_type {
            MessageType::Command => "command",
            MessageType::System => "system",
            MessageType::Error => "error",
            _ if ctx.message.has_attachments() => "media",
            _ => "default",
        };

        ctx.metadata
            .insert("route".to_string(), serde_json::json!(route));

        // Add route-specific metadata
        match route {
            "command" => {
                if let Some(command) = self.extract_command(&ctx.message.content) {
                    ctx.metadata
                        .insert("command".to_string(), serde_json::json!(command));
                }
            }
            "media" => {
                let media_types: Vec<String> = ctx
                    .message
                    .attachments
                    .iter()
                    .map(|a| a.mime_type.clone())
                    .collect();
                ctx.metadata
                    .insert("media_types".to_string(), serde_json::json!(media_types));
            }
            _ => {}
        }

        Ok(ctx)
    }
}

impl RouteStage {
    #[allow(clippy::unused_self)]
    fn extract_command(&self, content: &str) -> Option<String> {
        if content.starts_with('/') {
            content
                .split_whitespace()
                .next()
                .map(|cmd| cmd.trim_start_matches('/').to_string())
        } else {
            None
        }
    }
}

/// Processing stage - main AI processing
struct ProcessStage {
    #[allow(dead_code)]
    config: BotConfig,
}

impl ProcessStage {
    fn new(config: BotConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl PipelineStage for ProcessStage {
    fn name(&self) -> &str {
        "process"
    }

    async fn process(&self, mut ctx: PipelineContext) -> Result<PipelineContext> {
        // This is where we would integrate with AI providers
        // For now, create a simple response

        let route = ctx
            .metadata
            .get("route")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        let response_content = match route {
            "command" => self.process_command(&ctx),
            "system" => "System message received".to_string(),
            "error" => "Error processed".to_string(),
            "media" => format!("Received {} attachment(s)", ctx.message.attachments.len()),
            _ => format!("Processing message: {}", ctx.message.content),
        };

        let response = Response::text(ctx.message.conversation_id.clone(), response_content);

        ctx.metadata
            .insert("response".to_string(), serde_json::to_value(response)?);

        Ok(ctx)
    }
}

impl ProcessStage {
    #[allow(clippy::unused_self)]
    fn process_command(&self, ctx: &PipelineContext) -> String {
        let command = ctx
            .metadata
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        format!("Executing command: {command}")
    }
}

/// Formatting stage - formats the response
struct FormatStage;

impl FormatStage {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PipelineStage for FormatStage {
    fn name(&self) -> &str {
        "format"
    }

    async fn process(&self, mut ctx: PipelineContext) -> Result<PipelineContext> {
        if let Some(response_value) = ctx.metadata.get_mut("response") {
            if let Ok(mut response) = serde_json::from_value::<Response>(response_value.clone()) {
                // Apply formatting based on preferences
                if let Some(format_pref) = ctx.message.metadata.get("format") {
                    if let Some(format) = format_pref.as_str() {
                        match format {
                            "markdown" => {
                                response.response_type = crate::message::ResponseType::Markdown;
                            }
                            "html" => {
                                response.response_type = crate::message::ResponseType::Html;
                                response.content = self.to_html(&response.content);
                            }
                            "json" => {
                                response.response_type = crate::message::ResponseType::Json;
                            }
                            _ => {}
                        }
                    }
                }

                *response_value = serde_json::to_value(response)?;
            }
        }

        Ok(ctx)
    }
}

impl FormatStage {
    #[allow(clippy::unused_self)]
    fn to_html(&self, content: &str) -> String {
        // Simple HTML conversion
        format!(
            "<p>{}</p>",
            content
                .lines()
                .map(|line| format!("{}<br>", html_escape::encode_text(line)))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

/// Logging middleware
struct LoggingMiddleware {
    enabled: bool,
}

impl LoggingMiddleware {
    fn new() -> Self {
        Self { enabled: true }
    }
}

#[async_trait]
impl PipelineMiddleware for LoggingMiddleware {
    async fn before_pipeline(&self, message: Message) -> Result<Message> {
        if self.enabled {
            debug!("Pipeline processing message: {}", message.id);
        }
        Ok(message)
    }

    async fn after_pipeline(&self, response: Response) -> Result<Response> {
        if self.enabled {
            debug!("Pipeline generated response: {}", response.id);
        }
        Ok(response)
    }
}

/// Metrics middleware
struct MetricsMiddleware {
    start_time: Arc<RwLock<Option<std::time::Instant>>>,
}

impl MetricsMiddleware {
    fn new() -> Self {
        Self {
            start_time: Arc::new(RwLock::new(None)),
        }
    }
}

#[async_trait]
impl PipelineMiddleware for MetricsMiddleware {
    async fn before_pipeline(&self, message: Message) -> Result<Message> {
        *self.start_time.write() = Some(std::time::Instant::now());
        Ok(message)
    }

    async fn after_pipeline(&self, response: Response) -> Result<Response> {
        if let Some(start) = *self.start_time.read() {
            let duration = start.elapsed();
            debug!("Pipeline processing took {:?}", duration);
        }
        Ok(response)
    }
}

/// Timeout middleware
struct TimeoutMiddleware {
    #[allow(dead_code)]
    timeout: Duration,
}

impl TimeoutMiddleware {
    fn new(timeout: Duration) -> Self {
        Self { timeout }
    }
}

#[async_trait]
impl PipelineMiddleware for TimeoutMiddleware {
    async fn before_pipeline(&self, message: Message) -> Result<Message> {
        // Timeout would be enforced at the pipeline level
        Ok(message)
    }
}

/// Pipeline metrics
#[derive(Debug)]
pub struct PipelineMetrics {
    requests_total: Arc<RwLock<u64>>,
    processing_times: Arc<RwLock<Vec<Duration>>>,
}

impl PipelineMetrics {
    fn new() -> Self {
        Self {
            requests_total: Arc::new(RwLock::new(0)),
            processing_times: Arc::new(RwLock::new(Vec::new())),
        }
    }

    fn increment_requests(&self) {
        *self.requests_total.write() += 1;
    }

    fn record_processing_time(&self, duration: Duration) {
        let mut times = self.processing_times.write();
        times.push(duration);
        if times.len() > 1000 {
            times.remove(0);
        }
    }

    /// Get total requests processed
    #[must_use]
    pub fn requests_total(&self) -> u64 {
        *self.requests_total.read()
    }

    /// Get average processing time
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn average_processing_time(&self) -> Option<Duration> {
        let times = self.processing_times.read();
        if times.is_empty() {
            return None;
        }

        let total: Duration = times.iter().sum();
        Some(total / times.len() as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pipeline_creation() {
        let config = BotConfig::default();
        let pipeline = MessagePipeline::new(&config).await;
        assert!(pipeline.is_ok());
    }

    #[test]
    fn test_sanitize_stage() {
        let stage = SanitizeStage::new();
        let content = "Hello\x00World\x01Test";
        let sanitized = stage.sanitize_content(content);
        assert!(!sanitized.contains('\x00'));
        assert!(!sanitized.contains('\x01'));
    }

    #[test]
    fn test_route_stage_command_extraction() {
        let stage = RouteStage::new();
        assert_eq!(stage.extract_command("/help me"), Some("help".to_string()));
        assert_eq!(stage.extract_command("not a command"), None);
    }
}
