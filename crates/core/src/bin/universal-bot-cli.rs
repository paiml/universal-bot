//! Universal Bot CLI - Interactive command-line interface
//!
//! A simple CLI tool that provides direct access to Claude Opus 4.1 via AWS Bedrock.
//! Perfect for testing, learning, and quick AI interactions.
//!
//! # Installation
//!
//! ```bash
//! cargo install universal-bot-core
//! ```
//!
//! # Usage
//!
//! ```bash
//! universal-bot-cli
//! ```
//!
//! # Prerequisites
//!
//! - AWS credentials configured
//! - Access to AWS Bedrock with Claude models

use anyhow::Result;
use aws_config::BehaviorVersion;
use aws_sdk_bedrockruntime::{primitives::Blob, Client as BedrockClient};
use serde_json::json;
use std::io::{self, Write};

/// Claude Opus 4.1 inference profile for AWS Bedrock
const CLAUDE_OPUS_4_1: &str = "us.anthropic.claude-opus-4-1-20250805-v1:0";

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize AWS SDK
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region("us-east-1")
        .load()
        .await;

    let bedrock_client = BedrockClient::new(&aws_config);

    // Welcome message
    println!("\n🤖 Universal Bot CLI v1.0.0");
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

                // Handle help command
                if matches!(input.to_lowercase().as_str(), "help" | "/help" | "?") {
                    show_help();
                    continue;
                }

                query_count += 1;
                print!("⏳ Processing... ");
                io::stdout().flush()?;

                // Make request to Bedrock
                match query_bedrock(&bedrock_client, input).await {
                    Ok((response, usage)) => {
                        println!("✅\n");
                        println!("🤖 Claude: {}\n", response);
                        
                        if let Some((input_tokens, output_tokens)) = usage {
                            let total = input_tokens + output_tokens;
                            let cost = estimate_cost(input_tokens, output_tokens);
                            println!("📊 Tokens: {} in, {} out, {} total | Cost: ~${:.4}", 
                                   input_tokens, output_tokens, total, cost);
                        }
                        println!("─────────────────────────────────────────────\n");
                    }
                    Err(e) => {
                        println!("❌");
                        eprintln!("Error: {}\n", e);
                        println!("💡 Make sure you have AWS credentials configured and Bedrock access.");
                        println!("   Run: aws configure\n");
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Error reading input: {}", e);
                break;
            }
        }
    }

    if query_count > 0 {
        println!("📈 Session Summary:");
        println!("   Total queries: {}", query_count);
        println!("   Model: Claude Opus 4.1");
        println!("   Thank you for using Universal Bot! 🚀");
    }

    Ok(())
}

/// Show help information
fn show_help() {
    println!("\n📚 Universal Bot CLI Help");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Commands:");
    println!("  help, /help, ?     - Show this help");
    println!("  quit, exit, q      - Exit the CLI");
    println!("  <your question>    - Ask Claude anything");
    println!();
    println!("Examples:");
    println!("  What is Rust?");
    println!("  Write a Python function to sort a list");
    println!("  Explain quantum computing in simple terms");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
}

/// Estimate cost based on token usage (Claude Opus 4.1 pricing)
fn estimate_cost(input_tokens: u64, output_tokens: u64) -> f64 {
    // Claude Opus 4.1 pricing (approximate)
    let input_rate = 0.000015;  // $15 per 1M input tokens
    let output_rate = 0.000075; // $75 per 1M output tokens
    
    (input_tokens as f64 * input_rate) + (output_tokens as f64 * output_rate)
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