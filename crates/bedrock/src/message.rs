//! Message types and conversions for Bedrock client

use std::collections::HashMap;

use aws_sdk_bedrockruntime::types::{ContentBlock, Message as BedrockMessage};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{BedrockError, Result};

/// Universal message format for the bot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalMessage {
    /// Message role (user, assistant, system)
    pub role: MessageRole,
    /// Message content
    pub content: String,
    /// Optional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Message role enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// User message
    User,
    /// Assistant message
    Assistant,
    /// System message
    System,
}

impl UniversalMessage {
    /// Create a new user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            metadata: HashMap::new(),
        }
    }

    /// Create a new assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            metadata: HashMap::new(),
        }
    }

    /// Create a new system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the message
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Convert to AWS Bedrock message format
    pub fn to_bedrock_message(&self) -> Result<BedrockMessage> {
        let content = ContentBlock::Text(self.content.clone());

        let role = match self.role {
            MessageRole::User => aws_sdk_bedrockruntime::types::ConversationRole::User,
            MessageRole::Assistant => aws_sdk_bedrockruntime::types::ConversationRole::Assistant,
            MessageRole::System => {
                return Err(BedrockError::InvalidInput(
                    "System messages should be handled separately in Bedrock".to_string(),
                ));
            }
        };

        Ok(BedrockMessage::builder()
            .role(role)
            .content(content)
            .build()
            .map_err(|e| BedrockError::InvalidInput(format!("Failed to build message: {}", e)))?)
    }

    /// Create from AWS Bedrock message
    pub fn from_bedrock_message(message: &BedrockMessage) -> Result<Self> {
        let role = match message.role() {
            aws_sdk_bedrockruntime::types::ConversationRole::User => MessageRole::User,
            aws_sdk_bedrockruntime::types::ConversationRole::Assistant => MessageRole::Assistant,
            _ => {
                return Err(BedrockError::InvalidResponse(
                    "Unknown message role".to_string(),
                ))
            }
        };

        let content = message
            .content()
            .first()
            .and_then(|block| block.as_text().ok())
            .ok_or_else(|| BedrockError::InvalidResponse("No text content found".to_string()))?;

        Ok(Self {
            role,
            content: content.to_string(),
            metadata: HashMap::new(),
        })
    }
}

/// Response from text generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResponse {
    /// Unique response ID
    pub id: Uuid,
    /// Generated content
    pub content: String,
    /// Model used for generation
    pub model: String,
    /// Token usage information
    pub usage: Option<TokenUsage>,
    /// Response metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Response timestamp
    pub timestamp: DateTime<Utc>,
    /// Reason the generation finished
    pub finish_reason: String,
}

impl GenerationResponse {
    /// Check if the response was truncated due to token limits
    pub fn is_truncated(&self) -> bool {
        self.finish_reason == "max_tokens" || self.finish_reason == "length"
    }

    /// Check if the response was stopped by content filtering
    pub fn is_content_filtered(&self) -> bool {
        self.finish_reason == "content_filter"
    }

    /// Get the total tokens used
    pub fn total_tokens(&self) -> usize {
        self.usage.as_ref().map_or(0, |u| u.total_tokens)
    }

    /// Get the estimated cost
    pub fn estimated_cost(&self) -> f64 {
        self.usage.as_ref().map_or(0.0, |u| u.estimated_cost)
    }
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Input tokens
    pub input_tokens: usize,
    /// Output tokens
    pub output_tokens: usize,
    /// Total tokens
    pub total_tokens: usize,
    /// Estimated cost in USD
    pub estimated_cost: f64,
    /// Model identifier
    pub model: String,
}

impl TokenUsage {
    /// Create new token usage
    pub fn new(
        input_tokens: usize,
        output_tokens: usize,
        model: impl Into<String>,
        estimated_cost: f64,
    ) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
            estimated_cost,
            model: model.into(),
        }
    }
}

/// Stream chunk for streaming responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// Chunk ID
    pub id: Uuid,
    /// Chunk content
    pub content: String,
    /// Whether this is the final chunk
    pub is_final: bool,
    /// Token usage (only present in final chunk)
    pub usage: Option<TokenUsage>,
    /// Chunk metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Chunk timestamp
    pub timestamp: DateTime<Utc>,
}

impl StreamChunk {
    /// Create a new content chunk
    pub fn content(content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            content: content.into(),
            is_final: false,
            usage: None,
            metadata: HashMap::new(),
            timestamp: Utc::now(),
        }
    }

    /// Create a final chunk with usage information
    pub fn final_chunk(usage: TokenUsage) -> Self {
        Self {
            id: Uuid::new_v4(),
            content: String::new(),
            is_final: true,
            usage: Some(usage),
            metadata: HashMap::new(),
            timestamp: Utc::now(),
        }
    }
}

/// Conversation context for multi-turn interactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    /// Conversation ID
    pub id: String,
    /// Messages in the conversation
    pub messages: Vec<UniversalMessage>,
    /// Context metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Total tokens used in conversation
    pub total_tokens: usize,
    /// Conversation timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl ConversationContext {
    /// Create a new conversation context
    pub fn new(id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            messages: Vec::new(),
            metadata: HashMap::new(),
            total_tokens: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a message to the conversation
    pub fn add_message(&mut self, message: UniversalMessage) {
        self.messages.push(message);
        self.updated_at = Utc::now();
    }

    /// Add a user message
    pub fn add_user_message(&mut self, content: impl Into<String>) {
        self.add_message(UniversalMessage::user(content));
    }

    /// Add an assistant message with token usage
    pub fn add_assistant_message(&mut self, content: impl Into<String>, tokens: Option<usize>) {
        self.add_message(UniversalMessage::assistant(content));
        if let Some(token_count) = tokens {
            self.total_tokens += token_count;
        }
    }

    /// Get the last N messages
    pub fn last_messages(&self, n: usize) -> &[UniversalMessage] {
        let start = self.messages.len().saturating_sub(n);
        &self.messages[start..]
    }

    /// Trim the conversation to fit within token limits
    pub fn trim_to_token_limit(&mut self, max_tokens: usize) {
        // Simple implementation: remove oldest messages
        // In practice, you'd want more sophisticated strategies
        while self.total_tokens > max_tokens && !self.messages.is_empty() {
            self.messages.remove(0);
            // Recalculate tokens (simplified)
            self.total_tokens = self.messages.len() * 100; // Rough estimate
        }
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let user_msg = UniversalMessage::user("Hello, bot!");
        assert_eq!(user_msg.role, MessageRole::User);
        assert_eq!(user_msg.content, "Hello, bot!");

        let assistant_msg = UniversalMessage::assistant("Hello, user!");
        assert_eq!(assistant_msg.role, MessageRole::Assistant);
        assert_eq!(assistant_msg.content, "Hello, user!");
    }

    #[test]
    fn test_message_with_metadata() {
        let msg = UniversalMessage::user("Test").with_metadata("key", serde_json::json!("value"));

        assert!(msg.metadata.contains_key("key"));
        assert_eq!(msg.metadata["key"], serde_json::json!("value"));
    }

    #[test]
    fn test_bedrock_conversion() {
        let user_msg = UniversalMessage::user("Test message");
        let bedrock_msg = user_msg.to_bedrock_message().unwrap();

        // Verify the conversion worked
        assert!(!bedrock_msg.content().is_empty());

        // Convert back
        let converted_back = UniversalMessage::from_bedrock_message(&bedrock_msg).unwrap();
        assert_eq!(converted_back.role, MessageRole::User);
        assert_eq!(converted_back.content, "Test message");
    }

    #[test]
    fn test_system_message_conversion_error() {
        let system_msg = UniversalMessage::system("System prompt");
        let result = system_msg.to_bedrock_message();
        assert!(result.is_err());
    }

    #[test]
    fn test_generation_response() {
        let usage = TokenUsage::new(100, 50, "test-model", 0.01);
        let response = GenerationResponse {
            id: Uuid::new_v4(),
            content: "Generated text".to_string(),
            model: "test-model".to_string(),
            usage: Some(usage),
            metadata: HashMap::new(),
            timestamp: Utc::now(),
            finish_reason: "stop".to_string(),
        };

        assert_eq!(response.total_tokens(), 150);
        assert_eq!(response.estimated_cost(), 0.01);
        assert!(!response.is_truncated());
        assert!(!response.is_content_filtered());
    }

    #[test]
    fn test_conversation_context() {
        let mut context = ConversationContext::new("test-conversation");

        context.add_user_message("Hello");
        context.add_assistant_message("Hi there!", Some(20));

        assert_eq!(context.messages.len(), 2);
        assert_eq!(context.total_tokens, 20);

        let last_two = context.last_messages(2);
        assert_eq!(last_two.len(), 2);
    }

    #[test]
    fn test_stream_chunk() {
        let chunk = StreamChunk::content("Hello");
        assert_eq!(chunk.content, "Hello");
        assert!(!chunk.is_final);
        assert!(chunk.usage.is_none());

        let usage = TokenUsage::new(10, 5, "test", 0.001);
        let final_chunk = StreamChunk::final_chunk(usage);
        assert!(final_chunk.is_final);
        assert!(final_chunk.usage.is_some());
    }
}
