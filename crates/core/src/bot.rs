//! Bot implementation with AI model orchestration
//!
//! This module provides the main Bot struct that orchestrates AI interactions
//! through various providers and manages the conversation lifecycle.

use std::sync::Arc;

use anyhow::{Context as _, Result};
use parking_lot::RwLock;
use tracing::{debug, info, instrument, warn};

use crate::{
    config::BotConfig,
    context::ContextManager,
    message::{Message, Response},
    pipeline::MessagePipeline,
    plugin::PluginRegistry,
};

/// The main Bot struct that handles all AI interactions
///
/// The Bot coordinates between different components:
/// - Message pipeline for processing
/// - Context manager for state
/// - Plugin registry for extensions
/// - AI providers for generation
#[derive(Clone)]
pub struct Bot {
    config: Arc<BotConfig>,
    pipeline: Arc<MessagePipeline>,
    context_manager: Arc<ContextManager>,
    plugin_registry: Arc<RwLock<PluginRegistry>>,
    metrics: Arc<BotMetrics>,
}

impl Bot {
    /// Create a new Bot instance with the given configuration
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails, such as:
    /// - Invalid configuration
    /// - Failed to connect to AI provider
    /// - Plugin initialization failure
    ///
    /// # Example
    ///
    /// ```rust
    /// # use universal_bot_core::{Bot, BotConfig};
    /// # async fn example() -> anyhow::Result<()> {
    /// let config = BotConfig::default();
    /// let bot = Bot::new(config).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(config))]
    pub async fn new(config: BotConfig) -> Result<Self> {
        info!("Initializing Universal Bot v{}", crate::VERSION);

        // Validate configuration
        config.validate().context("Invalid bot configuration")?;

        // Initialize components
        let pipeline = MessagePipeline::new(&config)
            .await
            .context("Failed to create message pipeline")?;

        let context_manager = ContextManager::new(config.context_config.clone())
            .await
            .context("Failed to create context manager")?;

        let plugin_registry = PluginRegistry::new();

        let metrics = BotMetrics::new();

        let bot = Self {
            config: Arc::new(config),
            pipeline: Arc::new(pipeline),
            context_manager: Arc::new(context_manager),
            plugin_registry: Arc::new(RwLock::new(plugin_registry)),
            metrics: Arc::new(metrics),
        };

        // Load default plugins
        bot.load_default_plugins();

        info!("Bot initialized successfully");
        Ok(bot)
    }

    /// Process a message and generate a response
    ///
    /// This is the main entry point for all bot interactions.
    ///
    /// # Errors
    ///
    /// Returns an error if processing fails at any stage.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use universal_bot_core::{Bot, BotConfig, Message};
    /// # async fn example() -> anyhow::Result<()> {
    /// # let bot = Bot::new(BotConfig::default()).await?;
    /// let message = Message::text("Hello, bot!");
    /// let response = bot.process(message).await?;
    /// println!("Bot says: {}", response.content);
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::future_not_send)]
    #[instrument(skip(self, message), fields(message_id = %message.id))]
    pub async fn process(&self, message: Message) -> Result<Response> {
        let start = std::time::Instant::now();
        self.metrics.increment_requests();

        debug!("Processing message: {:?}", message.message_type);

        // Get or create context
        let context = self
            .context_manager
            .get_or_create(&message.conversation_id)
            .await
            .context("Failed to get conversation context")?;

        // Apply plugins pre-processing
        let message = self.apply_plugins_pre(message).await?;

        // Process through pipeline
        let response = self
            .pipeline
            .process(message, context.clone())
            .await
            .context("Pipeline processing failed")?;

        // Apply plugins post-processing
        let response = self.apply_plugins_post(response).await?;

        // Update context
        self.context_manager
            .update(&response.conversation_id, context)
            .await
            .context("Failed to update context")?;

        // Record metrics
        let duration = start.elapsed();
        self.metrics.record_response_time(duration);

        if response.error.is_some() {
            self.metrics.increment_errors();
            warn!("Response contains error: {:?}", response.error);
        } else {
            self.metrics.increment_success();
        }

        debug!("Message processed in {:?}", duration);
        Ok(response)
    }

    /// Register a plugin with the bot
    ///
    /// # Errors
    ///
    /// Returns an error if plugin registration fails.
    pub fn register_plugin<P>(&self, plugin: P) -> Result<()>
    where
        P: crate::plugin::Plugin + 'static,
    {
        self.plugin_registry.write().register(Box::new(plugin))?;
        Ok(())
    }

    /// Get the current bot configuration
    #[must_use]
    pub fn config(&self) -> &BotConfig {
        &self.config
    }

    /// Get metrics for monitoring
    #[must_use]
    pub fn metrics(&self) -> &BotMetrics {
        &self.metrics
    }

    // Private helper methods

    #[allow(clippy::unused_self)]
    fn load_default_plugins(&self) {
        debug!("Loading default plugins");
        // Load built-in plugins based on configuration
        // This would load various default plugins
    }

    #[allow(clippy::future_not_send)]
    async fn apply_plugins_pre(&self, message: Message) -> Result<Message> {
        let registry = self.plugin_registry.read();
        registry.apply_pre_processing(message).await
    }

    #[allow(clippy::future_not_send)]
    async fn apply_plugins_post(&self, response: Response) -> Result<Response> {
        let registry = self.plugin_registry.read();
        registry.apply_post_processing(response).await
    }
}

/// Builder for creating Bot instances with custom configuration
pub struct BotBuilder {
    config: BotConfig,
    plugins: Vec<Box<dyn crate::plugin::Plugin>>,
}

impl BotBuilder {
    /// Create a new builder with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: BotConfig::default(),
            plugins: Vec::new(),
        }
    }

    /// Set the bot configuration
    #[must_use]
    pub fn config(mut self, config: BotConfig) -> Self {
        self.config = config;
        self
    }

    /// Add a plugin to be registered on initialization
    #[must_use]
    pub fn plugin<P>(mut self, plugin: P) -> Self
    where
        P: crate::plugin::Plugin + 'static,
    {
        self.plugins.push(Box::new(plugin));
        self
    }

    /// Build the Bot instance
    ///
    /// # Errors
    ///
    /// Returns an error if bot creation fails.
    pub async fn build(self) -> Result<Bot> {
        let bot = Bot::new(self.config).await?;

        for plugin in self.plugins {
            let mut registry = bot.plugin_registry.write();
            registry.register(plugin)?;
        }

        Ok(bot)
    }
}

impl Default for BotBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics for monitoring bot performance
#[derive(Debug)]
pub struct BotMetrics {
    requests_total: Arc<RwLock<u64>>,
    success_total: Arc<RwLock<u64>>,
    errors_total: Arc<RwLock<u64>>,
    response_times: Arc<RwLock<Vec<std::time::Duration>>>,
}

impl BotMetrics {
    fn new() -> Self {
        Self {
            requests_total: Arc::new(RwLock::new(0)),
            success_total: Arc::new(RwLock::new(0)),
            errors_total: Arc::new(RwLock::new(0)),
            response_times: Arc::new(RwLock::new(Vec::new())),
        }
    }

    fn increment_requests(&self) {
        *self.requests_total.write() += 1;
    }

    fn increment_success(&self) {
        *self.success_total.write() += 1;
    }

    fn increment_errors(&self) {
        *self.errors_total.write() += 1;
    }

    fn record_response_time(&self, duration: std::time::Duration) {
        let mut times = self.response_times.write();
        times.push(duration);
        // Keep only last 1000 response times
        if times.len() > 1000 {
            times.remove(0);
        }
    }

    /// Get the total number of requests
    #[must_use]
    pub fn requests_total(&self) -> u64 {
        *self.requests_total.read()
    }

    /// Get the total number of successful responses
    #[must_use]
    pub fn success_total(&self) -> u64 {
        *self.success_total.read()
    }

    /// Get the total number of errors
    #[must_use]
    pub fn errors_total(&self) -> u64 {
        *self.errors_total.read()
    }

    /// Get the average response time
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn average_response_time(&self) -> Option<std::time::Duration> {
        let times = self.response_times.read();
        if times.is_empty() {
            return None;
        }

        let total: std::time::Duration = times.iter().sum();
        Some(total / times.len() as u32)
    }

    /// Get the success rate as a percentage
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn success_rate(&self) -> f64 {
        let requests = self.requests_total();
        if requests == 0 {
            return 100.0;
        }

        let success = self.success_total();
        (success as f64 / requests as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bot_creation() {
        let config = BotConfig::default();
        let bot = Bot::new(config).await;
        assert!(bot.is_ok());
    }

    #[tokio::test]
    async fn test_bot_builder() {
        let bot = BotBuilder::new().config(BotConfig::default()).build().await;
        assert!(bot.is_ok());
    }

    #[test]
    fn test_metrics() {
        let metrics = BotMetrics::new();

        assert_eq!(metrics.requests_total(), 0);
        assert_eq!(metrics.success_total(), 0);
        assert_eq!(metrics.errors_total(), 0);
        assert_eq!(metrics.success_rate(), 100.0);

        metrics.increment_requests();
        metrics.increment_success();
        assert_eq!(metrics.requests_total(), 1);
        assert_eq!(metrics.success_total(), 1);
        assert_eq!(metrics.success_rate(), 100.0);

        metrics.increment_requests();
        metrics.increment_errors();
        assert_eq!(metrics.requests_total(), 2);
        assert_eq!(metrics.errors_total(), 1);
        assert_eq!(metrics.success_rate(), 50.0);
    }

    #[test]
    fn test_metrics_response_time() {
        let metrics = BotMetrics::new();

        assert!(metrics.average_response_time().is_none());

        metrics.record_response_time(std::time::Duration::from_millis(100));
        metrics.record_response_time(std::time::Duration::from_millis(200));

        let avg = metrics.average_response_time().unwrap();
        assert_eq!(avg, std::time::Duration::from_millis(150));
    }

    #[cfg(feature = "property-testing")]
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn test_metrics_success_rate_bounds(
                requests in 0u64..1000,
                success in 0u64..1000
            ) {
                let metrics = BotMetrics::new();

                for _ in 0..requests {
                    metrics.increment_requests();
                }

                for _ in 0..success.min(requests) {
                    metrics.increment_success();
                }

                let rate = metrics.success_rate();
                prop_assert!(rate >= 0.0);
                prop_assert!(rate <= 100.0);
            }
        }
    }
}
