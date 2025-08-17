//! Message and response types for Universal Bot
//!
//! This module defines the core message structures used for communication
//! between the bot and its users, as well as internal message passing.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::error::{Error, Result};

/// A message sent to the bot
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Message {
    /// Unique message ID
    pub id: Uuid,

    /// Conversation ID for context tracking
    pub conversation_id: String,

    /// User ID who sent the message
    pub user_id: String,

    /// Message type
    pub message_type: MessageType,

    /// Message content
    #[validate(length(min = 1, max = 100_000))]
    pub content: String,

    /// Optional attachments
    pub attachments: Vec<Attachment>,

    /// Message metadata
    pub metadata: HashMap<String, serde_json::Value>,

    /// Timestamp when the message was created
    pub timestamp: DateTime<Utc>,

    /// Optional parent message ID for threading
    pub parent_id: Option<Uuid>,

    /// Message flags
    pub flags: MessageFlags,
}

impl Message {
    /// Create a new text message
    ///
    /// # Example
    ///
    /// ```rust
    /// use universal_bot_core::Message;
    ///
    /// let message = Message::text("Hello, bot!");
    /// assert_eq!(message.content, "Hello, bot!");
    /// ```
    #[must_use]
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            conversation_id: Uuid::new_v4().to_string(),
            user_id: "anonymous".to_string(),
            message_type: MessageType::Text,
            content: content.into(),
            attachments: Vec::new(),
            metadata: HashMap::new(),
            timestamp: Utc::now(),
            parent_id: None,
            flags: MessageFlags::default(),
        }
    }

    /// Create a new message with a specific type
    #[must_use]
    pub fn with_type(content: impl Into<String>, message_type: MessageType) -> Self {
        let mut message = Self::text(content);
        message.message_type = message_type;
        message
    }

    /// Set the conversation ID
    #[must_use]
    pub fn with_conversation_id(mut self, id: impl Into<String>) -> Self {
        self.conversation_id = id.into();
        self
    }

    /// Set the user ID
    #[must_use]
    pub fn with_user_id(mut self, id: impl Into<String>) -> Self {
        self.user_id = id.into();
        self
    }

    /// Add an attachment
    #[must_use]
    pub fn with_attachment(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Add metadata
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Set the parent message ID for threading
    #[must_use]
    pub fn with_parent(mut self, parent_id: Uuid) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Set message flags
    #[must_use]
    pub fn with_flags(mut self, flags: MessageFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Validate the message
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails.
    pub fn validate(&self) -> Result<()> {
        Validate::validate(self).map_err(|e| Error::Validation(e.to_string()))?;

        // Additional validation
        if self.content.is_empty() && self.attachments.is_empty() {
            return Err(Error::InvalidInput(
                "Message must have content or attachments".to_string(),
            ));
        }

        Ok(())
    }

    /// Check if this is a system message
    #[must_use]
    pub fn is_system(&self) -> bool {
        matches!(self.message_type, MessageType::System)
    }

    /// Check if this message has attachments
    #[must_use]
    pub fn has_attachments(&self) -> bool {
        !self.attachments.is_empty()
    }

    /// Get the total size of attachments in bytes
    #[must_use]
    pub fn attachment_size(&self) -> usize {
        self.attachments.iter().map(|a| a.size).sum()
    }
}

/// Type of message
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    /// Plain text message
    Text,
    /// Command to the bot
    Command,
    /// System message
    System,
    /// Error message
    Error,
    /// Embedded content
    Embed,
    /// File attachment
    File,
    /// Image attachment
    Image,
    /// Audio attachment
    Audio,
    /// Video attachment
    Video,
}

impl MessageType {
    /// Check if this is a media type
    #[must_use]
    pub fn is_media(&self) -> bool {
        matches!(self, Self::File | Self::Image | Self::Audio | Self::Video)
    }
}

/// Message flags for special handling
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageFlags {
    /// Message is urgent
    pub urgent: bool,
    /// Message is private
    pub private: bool,
    /// Message should be ephemeral
    pub ephemeral: bool,
    /// Message contains sensitive content
    pub sensitive: bool,
    /// Message should bypass filters
    pub bypass_filters: bool,
    /// Message should not be logged
    pub no_log: bool,
}

/// An attachment to a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    /// Unique attachment ID
    pub id: Uuid,
    /// Filename
    pub filename: String,
    /// MIME type
    pub mime_type: String,
    /// Size in bytes
    pub size: usize,
    /// URL or path to the attachment
    pub url: String,
    /// Optional thumbnail URL
    pub thumbnail_url: Option<String>,
    /// Attachment metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Attachment {
    /// Create a new attachment
    #[must_use]
    pub fn new(
        filename: impl Into<String>,
        mime_type: impl Into<String>,
        size: usize,
        url: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            filename: filename.into(),
            mime_type: mime_type.into(),
            size,
            url: url.into(),
            thumbnail_url: None,
            metadata: HashMap::new(),
        }
    }

    /// Check if this is an image attachment
    #[must_use]
    pub fn is_image(&self) -> bool {
        self.mime_type.starts_with("image/")
    }

    /// Check if this is a video attachment
    #[must_use]
    pub fn is_video(&self) -> bool {
        self.mime_type.starts_with("video/")
    }

    /// Check if this is an audio attachment
    #[must_use]
    pub fn is_audio(&self) -> bool {
        self.mime_type.starts_with("audio/")
    }
}

/// A response from the bot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// Unique response ID
    pub id: Uuid,
    /// Conversation ID
    pub conversation_id: String,
    /// Response content
    pub content: String,
    /// Response type
    pub response_type: ResponseType,
    /// Optional error information
    pub error: Option<ResponseError>,
    /// Response metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Token usage information
    pub usage: Option<TokenUsage>,
    /// Response flags
    pub flags: ResponseFlags,
    /// Optional suggested actions
    pub suggestions: Vec<Suggestion>,
}

impl Response {
    /// Create a new text response
    #[must_use]
    pub fn text(conversation_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            conversation_id: conversation_id.into(),
            content: content.into(),
            response_type: ResponseType::Text,
            error: None,
            metadata: HashMap::new(),
            timestamp: Utc::now(),
            usage: None,
            flags: ResponseFlags::default(),
            suggestions: Vec::new(),
        }
    }

    /// Create an error response
    #[must_use]
    pub fn error(conversation_id: impl Into<String>, error: ResponseError) -> Self {
        let mut response = Self::text(conversation_id, error.message.clone());
        response.response_type = ResponseType::Error;
        response.error = Some(error);
        response
    }

    /// Add token usage information
    #[must_use]
    pub fn with_usage(mut self, usage: TokenUsage) -> Self {
        self.usage = Some(usage);
        self
    }

    /// Add a suggestion
    #[must_use]
    pub fn with_suggestion(mut self, suggestion: Suggestion) -> Self {
        self.suggestions.push(suggestion);
        self
    }

    /// Set response flags
    #[must_use]
    pub fn with_flags(mut self, flags: ResponseFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Check if this response contains an error
    #[must_use]
    pub fn is_error(&self) -> bool {
        self.error.is_some() || matches!(self.response_type, ResponseType::Error)
    }

    /// Get the total tokens used
    #[must_use]
    pub fn total_tokens(&self) -> usize {
        self.usage.as_ref().map_or(0, |u| u.total_tokens)
    }
}

/// Type of response
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResponseType {
    /// Plain text response
    Text,
    /// Markdown formatted response
    Markdown,
    /// HTML formatted response
    Html,
    /// JSON structured response
    Json,
    /// Error response
    Error,
    /// Streaming response chunk
    Stream,
    /// Response with embedded content
    Embed,
}

/// Response error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseError {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Whether the error is retryable
    pub retryable: bool,
    /// Optional retry after duration in seconds
    pub retry_after: Option<u64>,
}

impl ResponseError {
    /// Create a new response error
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable: false,
            retry_after: None,
        }
    }

    /// Set whether the error is retryable
    #[must_use]
    pub fn retryable(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }

    /// Set retry after duration
    #[must_use]
    pub fn retry_after(mut self, seconds: u64) -> Self {
        self.retry_after = Some(seconds);
        self
    }
}

/// Response flags
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResponseFlags {
    /// Response was truncated
    pub truncated: bool,
    /// Response is partial (streaming)
    pub partial: bool,
    /// Response was cached
    pub cached: bool,
    /// Response contains sensitive content
    pub sensitive: bool,
    /// Response should not be cached
    pub no_cache: bool,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Input tokens used
    pub input_tokens: usize,
    /// Output tokens generated
    pub output_tokens: usize,
    /// Total tokens (input + output)
    pub total_tokens: usize,
    /// Estimated cost in USD
    pub estimated_cost: f64,
    /// Model used
    pub model: String,
}

impl TokenUsage {
    /// Create new token usage information
    #[must_use]
    pub fn new(input_tokens: usize, output_tokens: usize, model: impl Into<String>) -> Self {
        let model_string = model.into();
        let total_tokens = input_tokens + output_tokens;
        let estimated_cost = Self::calculate_cost(input_tokens, output_tokens, &model_string);

        Self {
            input_tokens,
            output_tokens,
            total_tokens,
            estimated_cost,
            model: model_string,
        }
    }

    fn calculate_cost(input_tokens: usize, output_tokens: usize, model: &str) -> f64 {
        // Cost per 1K tokens (example rates)
        let (input_rate, output_rate) = match model {
            "anthropic.claude-opus-4-1" => (0.015, 0.075),
            "anthropic.claude-sonnet-4" => (0.003, 0.015),
            "anthropic.claude-haiku" => (0.00025, 0.00125),
            _ => (0.001, 0.002),
        };

        (input_tokens as f64 / 1000.0)
            .mul_add(input_rate, output_tokens as f64 / 1000.0 * output_rate)
    }
}

/// A suggestion for follow-up actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    /// Suggestion text
    pub text: String,
    /// Action to take if selected
    pub action: SuggestionAction,
    /// Optional icon
    pub icon: Option<String>,
}

/// Action for a suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionAction {
    /// Send a message
    Message(String),
    /// Execute a command
    Command(String),
    /// Open a URL
    Url(String),
    /// Custom action
    Custom(serde_json::Value),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let message = Message::text("Hello, bot!");
        assert_eq!(message.content, "Hello, bot!");
        assert_eq!(message.message_type, MessageType::Text);
        assert!(message.validate().is_ok());
    }

    #[test]
    fn test_message_builder() {
        let attachment = Attachment::new(
            "image.png",
            "image/png",
            1024,
            "http://example.com/image.png",
        );
        let message = Message::text("Check this out")
            .with_conversation_id("conv-123")
            .with_user_id("user-456")
            .with_attachment(attachment)
            .with_metadata("key", serde_json::json!("value"));

        assert_eq!(message.conversation_id, "conv-123");
        assert_eq!(message.user_id, "user-456");
        assert_eq!(message.attachments.len(), 1);
        assert!(message.metadata.contains_key("key"));
    }

    #[test]
    fn test_empty_message_validation() {
        let mut message = Message::text("");
        message.content.clear();
        assert!(message.validate().is_err());
    }

    #[test]
    fn test_response_creation() {
        let response = Response::text("conv-123", "Hello, user!");
        assert_eq!(response.content, "Hello, user!");
        assert_eq!(response.conversation_id, "conv-123");
        assert!(!response.is_error());
    }

    #[test]
    fn test_error_response() {
        let error = ResponseError::new("E001", "Something went wrong")
            .retryable(true)
            .retry_after(60);
        let response = Response::error("conv-123", error);

        assert!(response.is_error());
        assert!(response.error.is_some());

        let error = response.error.unwrap();
        assert_eq!(error.code, "E001");
        assert!(error.retryable);
        assert_eq!(error.retry_after, Some(60));
    }

    #[test]
    fn test_token_usage() {
        let usage = TokenUsage::new(100, 50, "anthropic.claude-opus-4-1");
        assert_eq!(usage.total_tokens, 150);
        assert!(usage.estimated_cost > 0.0);
    }

    #[test]
    fn test_attachment_types() {
        let image = Attachment::new(
            "photo.jpg",
            "image/jpeg",
            2048,
            "http://example.com/photo.jpg",
        );
        assert!(image.is_image());
        assert!(!image.is_video());
        assert!(!image.is_audio());

        let video = Attachment::new(
            "movie.mp4",
            "video/mp4",
            1_048_576,
            "http://example.com/movie.mp4",
        );
        assert!(!video.is_image());
        assert!(video.is_video());
        assert!(!video.is_audio());

        let audio = Attachment::new(
            "song.mp3",
            "audio/mpeg",
            4096,
            "http://example.com/song.mp3",
        );
        assert!(!audio.is_image());
        assert!(!audio.is_video());
        assert!(audio.is_audio());
    }

    #[cfg(feature = "property-testing")]
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn test_message_id_uniqueness(content in any::<String>()) {
                let msg1 = Message::text(content.clone());
                let msg2 = Message::text(content);
                prop_assert_ne!(msg1.id, msg2.id);
            }

            #[test]
            fn test_token_cost_calculation(
                input in 0usize..100_000,
                output in 0usize..100_000
            ) {
                let usage = TokenUsage::new(input, output, "test-model");
                prop_assert_eq!(usage.total_tokens, input + output);
                prop_assert!(usage.estimated_cost >= 0.0);
            }
        }
    }
}
