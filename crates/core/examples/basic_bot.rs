//! Basic bot example demonstrating core functionality
//!
//! This example shows how to create a simple bot with default configuration
//! and process messages through the Universal Bot framework.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example basic_bot
//! ```

use anyhow::Result;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use universal_bot_core::{Bot, BotConfig, Message, MessageType};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Universal Bot Basic Example");

    // Create bot configuration
    let config = BotConfig::builder()
        .model("anthropic.claude-opus-4-1")
        .temperature(0.1)
        .max_tokens(1024)
        .enable_logging(true)
        .build()?;

    // Initialize the bot
    let bot = Bot::new(config).await?;
    info!("Bot initialized successfully");

    // Create test messages
    let messages = vec![
        Message::text("Hello, Universal Bot!"),
        Message::with_type("What can you do?", MessageType::Text),
        Message::text("/help")
            .with_conversation_id("test-conversation")
            .with_user_id("test-user"),
        Message::text("Can you help me with a complex task?"),
        Message::text("Thank you for your help!"),
    ];

    // Process each message
    for (i, message) in messages.into_iter().enumerate() {
        info!("Processing message {}: {}", i + 1, message.content);

        match bot.process(message).await {
            Ok(response) => {
                info!("✅ Response {}: {}", i + 1, response.content);

                if let Some(usage) = &response.usage {
                    info!(
                        "📊 Tokens used - Input: {}, Output: {}, Total: {}, Cost: ${:.4}",
                        usage.input_tokens,
                        usage.output_tokens,
                        usage.total_tokens,
                        usage.estimated_cost
                    );
                }
            }
            Err(e) => {
                eprintln!("❌ Error processing message {}: {}", i + 1, e);
            }
        }

        // Add a small delay between messages
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    // Display bot metrics
    let metrics = bot.metrics();
    info!("📈 Bot Metrics:");
    info!("  Total requests: {}", metrics.requests_total());
    info!("  Successful responses: {}", metrics.success_total());
    info!("  Error responses: {}", metrics.errors_total());
    info!("  Success rate: {:.2}%", metrics.success_rate());

    if let Some(avg_time) = metrics.average_response_time() {
        info!("  Average response time: {:?}", avg_time);
    }

    info!("Universal Bot Basic Example completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_bot_example() {
        // Initialize minimal logging for tests
        let _ = tracing_subscriber::fmt::try_init();

        // Create a simple test configuration
        let config = BotConfig::builder()
            .model("anthropic.claude-opus-4-1")
            .temperature(0.0)
            .max_tokens(100)
            .build()
            .expect("Failed to build config");

        // Create bot
        let bot = Bot::new(config).await.expect("Failed to create bot");

        // Test with a simple message
        let message = Message::text("Test message");
        let response = bot
            .process(message)
            .await
            .expect("Failed to process message");

        // Verify response
        assert!(!response.content.is_empty());
        assert!(!response.is_error());
    }
}
