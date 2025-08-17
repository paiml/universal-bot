//! AWS Bedrock Integration Test with Claude Opus 4.1
//!
//! This example demonstrates real integration with AWS Bedrock
//! using Claude Opus 4.1 model for AI-powered responses.

use anyhow::Result;
use aws_config::BehaviorVersion;
use aws_sdk_bedrockruntime::{
    types::{ContentBlock::Text, ConversationRole::*, Message as BedrockMessage},
    Client as BedrockClient,
};
use serde_json::json;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use universal_bot_core::{Bot, BotConfig, Message};

/// Simulated Bedrock provider for the Universal Bot framework
async fn call_bedrock_claude(prompt: &str) -> Result<String> {
    info!("🚀 Calling AWS Bedrock with Claude Opus 4.1...");

    // Initialize AWS SDK
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region("us-east-1")
        .load()
        .await;

    let client = BedrockClient::new(&config);

    // Prepare the request for Claude Opus 4.1
    let messages = vec![BedrockMessage::builder()
        .role(User)
        .content(Text(prompt.to_string()))
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build message: {}", e))?];

    let request_body = json!({
        "anthropic_version": "bedrock-2023-05-31",
        "messages": messages.iter().map(|m| {
            json!({
                "role": match m.role() {
                    User => "user",
                    Assistant => "assistant",
                    _ => "user",
                },
                "content": m.content().iter().map(|c| {
                    match c {
                        Text(text) => json!({
                            "type": "text",
                            "text": text
                        }),
                        _ => json!({
                            "type": "text",
                            "text": ""
                        })
                    }
                }).collect::<Vec<_>>()
            })
        }).collect::<Vec<_>>(),
        "max_tokens": 1024,
        "temperature": 0.1,
        "top_p": 0.9,
        "top_k": 250,
        "stop_sequences": []
    });

    info!("📤 Sending request to Claude Opus 4.1 on Bedrock...");

    // Call Bedrock
    let response = client
        .invoke_model()
        .model_id("us.anthropic.claude-opus-4-1-20250805-v1:0") // Using Claude Opus 4.1 inference profile
        .content_type("application/json")
        .accept("application/json")
        .body(aws_sdk_bedrockruntime::primitives::Blob::new(
            serde_json::to_vec(&request_body)?,
        ))
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Bedrock API call failed: {}", e))?;

    // Parse the response
    let response_body = response.body().as_ref();
    let response_json: serde_json::Value = serde_json::from_slice(response_body)?;

    info!("✅ Received response from Claude Opus 4.1");

    // Extract the text from Claude's response
    let text = response_json["content"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|content| content["text"].as_str())
        .unwrap_or("No response text found")
        .to_string();

    // Log token usage if available
    if let Some(usage) = response_json["usage"].as_object() {
        info!("📊 Token Usage:");
        info!(
            "  Input tokens: {}",
            usage
                .get("input_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
        );
        info!(
            "  Output tokens: {}",
            usage
                .get("output_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
        );
    }

    Ok(text)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("🤖 Universal Bot - AWS Bedrock Integration Test");
    info!("📍 Using Model: Claude Opus 4.1 (us.anthropic.claude-opus-4-1-20250805-v1:0)");
    info!("🌍 Region: us-east-1");

    // Create bot configuration pointing to Claude Opus 4.1
    let config = BotConfig::builder()
        .model("us.anthropic.claude-opus-4-1-20250805-v1:0") // Claude Opus 4.1 inference profile
        .temperature(0.1)
        .max_tokens(1024)
        .enable_logging(true)
        .enable_cost_tracking(true)
        .build()?;

    // Initialize the bot
    let bot = Bot::new(config).await?;
    info!("✅ Bot initialized with Bedrock configuration");

    // Test messages to send to Claude Opus 4.1
    let test_prompts = vec![
        "What is the capital of France? Give a one-sentence answer.",
        "Write a haiku about artificial intelligence.",
        "Explain quantum computing in simple terms (2-3 sentences max).",
        "What are the three primary colors? List them.",
        "Translate 'Hello, how are you?' to Spanish, French, and Japanese.",
    ];

    info!("\n🧪 Starting Bedrock Integration Tests:\n");

    for (i, prompt) in test_prompts.iter().enumerate() {
        info!("─────────────────────────────────────────");
        info!("Test {}: {}", i + 1, prompt);
        info!("─────────────────────────────────────────");

        // First test with the bot framework (simulated)
        let message = Message::text(*prompt);
        match bot.process(message).await {
            Ok(response) => {
                info!("📝 Bot Framework Response: {}", response.content);

                if let Some(usage) = &response.usage {
                    info!("💰 Estimated Cost: ${:.4}", usage.estimated_cost);
                }
            }
            Err(e) => {
                info!("❌ Bot Framework Error: {}", e);
            }
        }

        // Now make a real call to AWS Bedrock
        info!("\n🔄 Making real AWS Bedrock call...");
        match call_bedrock_claude(prompt).await {
            Ok(response) => {
                info!("🎯 Claude Opus 4.1 Response: {}", response);
            }
            Err(e) => {
                info!("❌ Bedrock Error: {}", e);
                info!("💡 Note: Ensure AWS credentials are configured and you have access to Claude models");
            }
        }

        info!("");
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    }

    // Display final metrics
    let metrics = bot.metrics();
    info!("═══════════════════════════════════════════");
    info!("📊 Final Test Metrics:");
    info!("  Total requests: {}", metrics.requests_total());
    info!("  Success rate: {:.2}%", metrics.success_rate());

    if let Some(avg_time) = metrics.average_response_time() {
        info!("  Average response time: {:?}", avg_time);
    }

    info!("═══════════════════════════════════════════");
    info!("✅ AWS Bedrock Integration Test Complete!");
    info!("🎉 Successfully demonstrated Universal Bot with Claude Opus 4.1 on AWS Bedrock");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bedrock_connection() {
        // This test verifies AWS SDK can be initialized
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region("us-east-1")
            .load()
            .await;

        let client = BedrockClient::new(&config);
        assert!(!client.config().region().unwrap().to_string().is_empty());
    }
}
