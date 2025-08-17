//! Interactive CLI bot example for Universal Bot
//!
//! This example creates a simple command-line interface that allows you to
//! interact with Claude Opus 4.1 through AWS Bedrock, just like Claude Code.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example interactive_cli
//! ```
//!
//! Then type your queries and press Enter. Type 'quit' or 'exit' to stop.
//!
//! # Prerequisites
//!
//! - AWS credentials configured (`aws configure`)
//! - Access to AWS Bedrock with Claude models

use anyhow::Result;
use aws_config::BehaviorVersion;
use aws_sdk_bedrockruntime::{primitives::Blob, Client as BedrockClient};
use serde_json::json;
use std::io::{self, Write};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// Claude Opus 4.1 inference profile for AWS Bedrock
const CLAUDE_OPUS_4_1: &str = "us.anthropic.claude-opus-4-1-20250805-v1:0";

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Initialize AWS SDK
    info!("🔧 Initializing AWS Bedrock client...");
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region("us-east-1")
        .load()
        .await;

    let bedrock_client = BedrockClient::new(&aws_config);
    info!("✅ AWS Bedrock client initialized");

    // Welcome message
    println!("\n🤖 Universal Bot Interactive CLI");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("💬 Connected to Claude Opus 4.1 via AWS Bedrock");
    println!("📝 Type your questions and press Enter");
    println!("🚪 Type 'quit', 'exit', or Ctrl+C to stop");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let mut query_count = 0;

    loop {
        // Prompt for user input
        print!("🧠 You: ");
        io::stdout().flush()?;

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let input = input.trim();

                // Check for exit commands
                if input.is_empty() {
                    continue;
                }

                if matches!(input.to_lowercase().as_str(), "quit" | "exit" | "q") {
                    println!("\n👋 Goodbye! Thanks for using Universal Bot!");
                    break;
                }

                query_count += 1;
                println!("⏳ Processing query {}...", query_count);

                // Make request to Bedrock
                match query_bedrock(&bedrock_client, input).await {
                    Ok((response, usage)) => {
                        println!("🤖 Claude: {}\n", response);
                        
                        if let Some((input_tokens, output_tokens)) = usage {
                            println!("📊 Tokens - Input: {}, Output: {}, Total: {}", 
                                   input_tokens, output_tokens, input_tokens + output_tokens);
                        }
                        println!("─────────────────────────────────────────────\n");
                    }
                    Err(e) => {
                        eprintln!("❌ Error: {}\n", e);
                        println!("💡 Make sure you have AWS credentials configured and Bedrock access.\n");
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Error reading input: {}", e);
                break;
            }
        }
    }

    println!("📈 Session Summary:");
    println!("   Total queries processed: {}", query_count);
    println!("   Model used: Claude Opus 4.1");
    println!("   Thank you for using Universal Bot! 🚀");

    Ok(())
}

/// Query AWS Bedrock with Claude Opus 4.1
async fn query_bedrock(
    client: &BedrockClient,
    prompt: &str,
) -> Result<(String, Option<(u64, u64)>)> {
    // Format request for Claude Messages API
    let request_body = json!({
        "anthropic_version": "bedrock-2023-05-31",
        "max_tokens": 2048,
        "temperature": 0.1,
        "messages": [{
            "role": "user",
            "content": prompt
        }]
    });

    // Send request to Bedrock
    let response = client
        .invoke_model()
        .model_id(CLAUDE_OPUS_4_1)
        .content_type("application/json")
        .body(Blob::new(request_body.to_string()))
        .send()
        .await?;

    // Parse response
    let response_bytes = response.body.into_inner();
    let response_json: serde_json::Value = serde_json::from_slice(&response_bytes)?;

    // Extract the text response
    let content = response_json["content"][0]["text"]
        .as_str()
        .unwrap_or("No response received")
        .to_string();

    // Extract usage information
    let usage = if let Some(usage_obj) = response_json["usage"].as_object() {
        let input_tokens = usage_obj["input_tokens"].as_u64().unwrap_or(0);
        let output_tokens = usage_obj["output_tokens"].as_u64().unwrap_or(0);
        Some((input_tokens, output_tokens))
    } else {
        None
    };

    Ok((content, usage))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert!(!CLAUDE_OPUS_4_1.is_empty());
        assert!(CLAUDE_OPUS_4_1.contains("claude-opus"));
    }

    #[tokio::test]
    async fn test_query_format() {
        let test_prompt = "Hello";
        let request_body = json!({
            "anthropic_version": "bedrock-2023-05-31",
            "max_tokens": 2048,
            "temperature": 0.1,
            "messages": [{
                "role": "user",
                "content": test_prompt
            }]
        });

        assert!(request_body["messages"].is_array());
        assert_eq!(request_body["messages"][0]["content"], test_prompt);
    }
}