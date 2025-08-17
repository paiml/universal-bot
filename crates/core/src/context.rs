//! Context management for conversation state
//!
//! This module provides context tracking and management for maintaining
//! conversation state across multiple interactions.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};
use uuid::Uuid;

use crate::{
    config::{ContextConfig, StorageBackend},
    error::Error,
    message::{Message, Response},
};

/// Conversation context containing state and history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    /// Unique context ID
    pub id: String,

    /// Conversation history
    pub history: VecDeque<ContextMessage>,

    /// User information
    pub user: UserContext,

    /// Session variables
    pub variables: HashMap<String, serde_json::Value>,

    /// Context metadata
    pub metadata: ContextMetadata,

    /// Token count for the context
    pub token_count: usize,
}

impl Context {
    /// Create a new context
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            history: VecDeque::new(),
            user: UserContext::default(),
            variables: HashMap::new(),
            metadata: ContextMetadata::new(),
            token_count: 0,
        }
    }

    /// Add a message to the history
    pub fn add_message(&mut self, message: &Message) {
        let context_msg = ContextMessage::from_message(message);
        self.token_count += context_msg.estimated_tokens();
        self.history.push_back(context_msg);
        self.metadata.last_activity = Utc::now();
        self.metadata.message_count += 1;
    }

    /// Add a response to the history
    pub fn add_response(&mut self, response: &Response) {
        let context_msg = ContextMessage::from_response(response);
        self.token_count += context_msg.estimated_tokens();
        self.history.push_back(context_msg);
        self.metadata.last_activity = Utc::now();
        self.metadata.message_count += 1;

        if let Some(usage) = &response.usage {
            self.metadata.total_tokens += usage.total_tokens;
            self.metadata.total_cost += usage.estimated_cost;
        }
    }

    /// Trim history to fit within token limit
    pub fn trim_to_token_limit(&mut self, max_tokens: usize) {
        while self.token_count > max_tokens && !self.history.is_empty() {
            if let Some(removed) = self.history.pop_front() {
                self.token_count = self.token_count.saturating_sub(removed.estimated_tokens());
            }
        }
    }

    /// Get a variable value
    pub fn get_variable(&self, key: &str) -> Option<&serde_json::Value> {
        self.variables.get(key)
    }

    /// Set a variable value
    pub fn set_variable(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.variables.insert(key.into(), value);
    }

    /// Clear all history
    pub fn clear_history(&mut self) {
        self.history.clear();
        self.token_count = 0;
        self.metadata.message_count = 0;
    }

    /// Get the age of the context
    #[must_use]
    pub fn age(&self) -> Duration {
        let now = Utc::now();
        (now - self.metadata.created_at)
            .to_std()
            .unwrap_or(Duration::ZERO)
    }

    /// Check if the context is expired
    #[must_use]
    pub fn is_expired(&self, ttl: Duration) -> bool {
        self.age() > ttl
    }

    /// Get a summary of the context
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "Context {} - Messages: {}, Tokens: {}, Age: {:?}",
            self.id,
            self.metadata.message_count,
            self.token_count,
            self.age()
        )
    }
}

/// A message in the context history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMessage {
    /// Message role
    pub role: MessageRole,
    /// Message content
    pub content: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Optional message ID
    pub message_id: Option<Uuid>,
}

impl ContextMessage {
    /// Create from a user message
    pub fn from_message(message: &Message) -> Self {
        Self {
            role: MessageRole::User,
            content: message.content.clone(),
            timestamp: message.timestamp,
            message_id: Some(message.id),
        }
    }

    /// Create from a bot response
    pub fn from_response(response: &Response) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: response.content.clone(),
            timestamp: response.timestamp,
            message_id: Some(response.id),
        }
    }

    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            timestamp: Utc::now(),
            message_id: None,
        }
    }

    /// Estimate token count (rough approximation)
    const fn estimated_tokens(&self) -> usize {
        // Rough estimate: 1 token per 4 characters
        self.content.len() / 4
    }
}

/// Message role in conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// System message
    System,
    /// User message
    User,
    /// Assistant message
    Assistant,
}

/// User context information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserContext {
    /// User ID
    pub id: Option<String>,
    /// User name
    pub name: Option<String>,
    /// User preferences
    pub preferences: HashMap<String, serde_json::Value>,
    /// User attributes
    pub attributes: HashMap<String, String>,
}

/// Context metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMetadata {
    /// When the context was created
    pub created_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
    /// Total message count
    pub message_count: usize,
    /// Total tokens used
    pub total_tokens: usize,
    /// Total cost incurred
    pub total_cost: f64,
    /// Custom tags
    pub tags: Vec<String>,
}

impl ContextMetadata {
    fn new() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            last_activity: now,
            message_count: 0,
            total_tokens: 0,
            total_cost: 0.0,
            tags: Vec::new(),
        }
    }
}

/// Context manager for handling multiple conversation contexts
pub struct ContextManager {
    config: ContextConfig,
    store: Arc<dyn ContextStore>,
    cache: Arc<DashMap<String, Arc<RwLock<Context>>>>,
}

impl ContextManager {
    /// Create a new context manager
    ///
    /// # Errors
    ///
    /// Returns an error if store initialization fails.
    #[instrument(skip(config))]
    pub async fn new(config: ContextConfig) -> Result<Self> {
        debug!("Creating context manager with config: {:?}", config);

        let store: Arc<dyn ContextStore> = match &config.storage_backend {
            StorageBackend::Memory => Arc::new(MemoryContextStore::new()),
            StorageBackend::Redis { url: _ } => {
                // Would initialize Redis store here
                return Err(Error::new("Redis store not yet implemented").into());
            }
            StorageBackend::Postgres { url: _ } => {
                // Would initialize Postgres store here
                return Err(Error::new("Postgres store not yet implemented").into());
            }
            StorageBackend::Sqlite { path: _ } => {
                // Would initialize SQLite store here
                return Err(Error::new("SQLite store not yet implemented").into());
            }
        };

        Ok(Self {
            config,
            store,
            cache: Arc::new(DashMap::new()),
        })
    }

    /// Get or create a context
    ///
    /// # Errors
    ///
    /// Returns an error if context creation or retrieval fails
    #[instrument(skip(self))]
    pub async fn get_or_create(&self, id: &str) -> Result<Arc<RwLock<Context>>> {
        // Check cache first
        if let Some(context) = self.cache.get(id) {
            let ctx = context.clone();

            // Check if expired
            if ctx.read().is_expired(self.config.context_ttl) {
                debug!("Context {} is expired, removing", id);
                self.cache.remove(id);
            } else {
                debug!("Found context {} in cache", id);
                return Ok(ctx);
            }
        }

        // Try to load from store
        if let Some(context) = self.store.get(id).await? {
            if !context.is_expired(self.config.context_ttl) {
                debug!("Loaded context {} from store", id);
                let ctx = Arc::new(RwLock::new(context));
                self.cache.insert(id.to_string(), ctx.clone());
                return Ok(ctx);
            }
        }

        // Create new context
        debug!("Creating new context {}", id);
        let context = Context::new(id);
        let ctx = Arc::new(RwLock::new(context));
        self.cache.insert(id.to_string(), ctx.clone());

        // Persist if configured
        if self.config.persist_context {
            let context = ctx.read().clone();
            self.store.set(id, context, self.config.context_ttl).await?;
        }

        Ok(ctx)
    }

    /// Update a context
    ///
    /// # Errors
    ///
    /// Returns an error if the update operation fails
    #[instrument(skip(self, context))]
    pub async fn update(&self, id: &str, context: Arc<RwLock<Context>>) -> Result<()> {
        // Trim to token limit
        {
            let mut ctx = context.write();
            ctx.trim_to_token_limit(self.config.max_context_tokens);
        }

        // Update cache
        self.cache.insert(id.to_string(), context.clone());

        // Persist if configured
        if self.config.persist_context {
            let ctx = context.read().clone();
            self.store.set(id, ctx, self.config.context_ttl).await?;
        }

        Ok(())
    }

    /// Delete a context
    ///
    /// # Errors
    ///
    /// Returns an error if the deletion fails
    #[instrument(skip(self))]
    pub async fn delete(&self, id: &str) -> Result<()> {
        debug!("Deleting context {}", id);
        self.cache.remove(id);
        self.store.delete(id).await?;
        Ok(())
    }

    /// Clear expired contexts
    ///
    /// # Errors
    ///
    /// Returns an error if clearing expired contexts fails
    #[instrument(skip(self))]
    pub async fn clear_expired(&self) -> Result<usize> {
        let mut removed = 0;
        let expired_keys: Vec<String> = self
            .cache
            .iter()
            .filter(|entry| entry.value().read().is_expired(self.config.context_ttl))
            .map(|entry| entry.key().clone())
            .collect();

        for key in expired_keys {
            self.cache.remove(&key);
            self.store.delete(&key).await?;
            removed += 1;
        }

        debug!("Removed {} expired contexts", removed);
        Ok(removed)
    }

    /// Get statistics about managed contexts
    #[must_use]
    pub fn stats(&self) -> ContextStats {
        let total = self.cache.len();
        let mut total_tokens = 0;
        let mut total_messages = 0;

        for entry in self.cache.iter() {
            let ctx = entry.value().read();
            total_tokens += ctx.token_count;
            total_messages += ctx.metadata.message_count;
        }

        ContextStats {
            total_contexts: total,
            total_tokens,
            total_messages,
            cache_size: total,
        }
    }
}

/// Context store trait for persistence
#[async_trait::async_trait]
pub trait ContextStore: Send + Sync {
    /// Get a context by ID
    async fn get(&self, key: &str) -> Result<Option<Context>>;

    /// Set a context with TTL
    async fn set(&self, key: &str, context: Context, ttl: Duration) -> Result<()>;

    /// Delete a context
    async fn delete(&self, key: &str) -> Result<()>;

    /// List all context keys
    async fn list_keys(&self, pattern: &str) -> Result<Vec<String>>;
}

/// In-memory context store implementation
struct MemoryContextStore {
    data: Arc<DashMap<String, (Context, DateTime<Utc>)>>,
}

impl MemoryContextStore {
    fn new() -> Self {
        Self {
            data: Arc::new(DashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl ContextStore for MemoryContextStore {
    async fn get(&self, key: &str) -> Result<Option<Context>> {
        Ok(self.data.get(key).map(|entry| entry.0.clone()))
    }

    async fn set(&self, key: &str, context: Context, ttl: Duration) -> Result<()> {
        let expiry = Utc::now() + chrono::Duration::from_std(ttl)?;
        self.data.insert(key.to_string(), (context, expiry));
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.data.remove(key);
        Ok(())
    }

    async fn list_keys(&self, pattern: &str) -> Result<Vec<String>> {
        let keys = self
            .data
            .iter()
            .filter(|entry| entry.key().contains(pattern))
            .map(|entry| entry.key().clone())
            .collect();
        Ok(keys)
    }
}

/// Statistics about managed contexts
#[derive(Debug, Clone)]
pub struct ContextStats {
    /// Total number of contexts
    pub total_contexts: usize,
    /// Total tokens across all contexts
    pub total_tokens: usize,
    /// Total messages across all contexts
    pub total_messages: usize,
    /// Number of contexts in cache
    pub cache_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let context = Context::new("test-123");
        assert_eq!(context.id, "test-123");
        assert!(context.history.is_empty());
        assert_eq!(context.token_count, 0);
    }

    #[test]
    fn test_context_message_addition() {
        let mut context = Context::new("test");
        let message = Message::text("Hello");

        context.add_message(&message);
        assert_eq!(context.history.len(), 1);
        assert!(context.token_count > 0);
        assert_eq!(context.metadata.message_count, 1);
    }

    #[test]
    fn test_context_trimming() {
        let mut context = Context::new("test");

        // Add multiple messages
        for i in 0..10 {
            let msg = Message::text(format!("Message {i}"));
            context.add_message(&msg);
        }

        let original_count = context.history.len();
        context.trim_to_token_limit(10); // Very low limit

        assert!(context.history.len() < original_count);
        assert!(context.token_count <= 10);
    }

    #[test]
    fn test_context_variables() {
        let mut context = Context::new("test");

        context.set_variable("key", serde_json::json!("value"));
        assert_eq!(
            context.get_variable("key"),
            Some(&serde_json::json!("value"))
        );
        assert_eq!(context.get_variable("missing"), None);
    }

    #[test]
    fn test_context_expiry() {
        let context = Context::new("test");
        assert!(!context.is_expired(Duration::from_secs(3600)));

        // Can't easily test actual expiry without mocking time
    }

    #[tokio::test]
    async fn test_context_manager() {
        let config = ContextConfig::default();
        let manager = ContextManager::new(config).await.unwrap();

        let ctx1 = manager.get_or_create("test-1").await.unwrap();
        let ctx2 = manager.get_or_create("test-1").await.unwrap();

        // Should get the same context
        assert_eq!(ctx1.read().id, ctx2.read().id);
    }

    #[tokio::test]
    async fn test_memory_store() {
        let store = MemoryContextStore::new();
        let context = Context::new("test");

        store
            .set("test", context.clone(), Duration::from_secs(60))
            .await
            .unwrap();

        let loaded = store.get("test").await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().id, "test");

        store.delete("test").await.unwrap();
        let deleted = store.get("test").await.unwrap();
        assert!(deleted.is_none());
    }
}
