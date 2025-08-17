//! Step-by-step demonstration of Universal Bot with Claude Opus 4.1

use anyhow::Result;
use aws_config::BehaviorVersion;
use aws_sdk_bedrockruntime::{primitives::Blob, Client as BedrockClient};
use serde_json::json;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use universal_bot_core::{Bot, BotConfig, Message};

const CLAUDE_OPUS_4_1: &str = "us.anthropic.claude-opus-4-1-20250805-v1:0";

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("════════════════════════════════════════════════════════");
    info!("🚀 STEP-BY-STEP DEMO: Universal Bot + Claude Opus 4.1");
    info!("════════════════════════════════════════════════════════");

    // STEP 1: Initialize the Bot with Claude Opus 4.1
    info!("\n📍 STEP 1: Initialize Bot with Claude Opus 4.1");
    info!("─────────────────────────────────────────────");

    let config = BotConfig::builder()
        .model(CLAUDE_OPUS_4_1) // ONLY Claude Opus 4.1
        .temperature(0.1)
        .max_tokens(500)
        .enable_logging(true)
        .enable_cost_tracking(true)
        .build()?;

    info!("✅ Configuration created:");
    info!("   Model: {}", CLAUDE_OPUS_4_1);
    info!("   Temperature: 0.1 (deterministic)");
    info!("   Max tokens: 500");

    let bot = Bot::new(config).await?;
    info!("✅ Bot initialized successfully!");

    // STEP 2: Process a simple message through the framework
    info!("\n📍 STEP 2: Process Message Through Framework");
    info!("─────────────────────────────────────────────");

    let test_message = Message::text("What is the capital of Japan?");
    info!("📝 Input message: \"{}\"", test_message.content);
    info!("🔄 Processing through pipeline...");

    let response = bot.process(test_message).await?;
    info!("✅ Framework response: \"{}\"", response.content);

    if let Some(usage) = &response.usage {
        info!(
            "📊 Token usage - Input: {}, Output: {}",
            usage.input_tokens, usage.output_tokens
        );
        info!("💰 Estimated cost: ${:.6}", usage.estimated_cost);
    }

    // STEP 3: Make a REAL call to AWS Bedrock
    info!("\n📍 STEP 3: Real AWS Bedrock Call to Claude Opus 4.1");
    info!("─────────────────────────────────────────────────────");

    info!("🔧 Initializing AWS SDK...");
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region("us-east-1")
        .load()
        .await;

    let bedrock_client = BedrockClient::new(&aws_config);
    info!("✅ AWS client initialized");

    let prompt = "Write a haiku about robots. Return only the haiku, no explanation.";
    info!("📝 Prompt: \"{}\"", prompt);

    // Format request for Claude Messages API
    let request_body = json!({
        "anthropic_version": "bedrock-2023-05-31",
        "max_tokens": 100,
        "temperature": 0.1,
        "messages": [{
            "role": "user",
            "content": prompt
        }]
    });

    info!("📤 Sending request to Bedrock...");
    info!("   Endpoint: AWS Bedrock");
    info!("   Model: {}", CLAUDE_OPUS_4_1);

    let start = std::time::Instant::now();
    let response = bedrock_client
        .invoke_model()
        .model_id(CLAUDE_OPUS_4_1)
        .content_type("application/json")
        .accept("application/json")
        .body(Blob::new(request_body.to_string()))
        .send()
        .await?;

    let duration = start.elapsed();
    info!("⏱️ Response time: {:?}", duration);

    // Parse response
    let response_bytes = response.body.into_inner();
    let response_json: serde_json::Value = serde_json::from_slice(&response_bytes)?;

    let haiku = response_json["content"][0]["text"]
        .as_str()
        .unwrap_or("No response");

    info!("✅ Claude Opus 4.1 Response:");
    info!("   {}", haiku);

    if let Some(usage) = response_json["usage"].as_object() {
        info!("📊 Actual token usage:");
        info!("   Input: {}", usage["input_tokens"].as_u64().unwrap_or(0));
        info!(
            "   Output: {}",
            usage["output_tokens"].as_u64().unwrap_or(0)
        );
    }

    // STEP 4: Generate structured YAML content
    info!("\n📍 STEP 4: Generate YAML Template (PDMT Style)");
    info!("─────────────────────────────────────────────");

    let yaml_prompt = r#"Generate a YAML todo list with exactly 3 tasks for: "Create a blog website"

Output ONLY valid YAML in this exact format:
todos:
  - id: "todo_0_0"
    content: "Task description"
    priority: "high"
    hours: 2
  - id: "todo_0_1"
    content: "Task description"
    priority: "medium"
    hours: 3
  - id: "todo_0_2"
    content: "Task description"
    priority: "low"
    hours: 1"#;

    info!("📝 Requesting structured YAML generation...");

    let yaml_request = json!({
        "anthropic_version": "bedrock-2023-05-31",
        "max_tokens": 300,
        "temperature": 0.1,  // Low temp for deterministic output
        "messages": [{
            "role": "user",
            "content": yaml_prompt
        }]
    });

    let yaml_response = bedrock_client
        .invoke_model()
        .model_id(CLAUDE_OPUS_4_1)
        .content_type("application/json")
        .accept("application/json")
        .body(Blob::new(yaml_request.to_string()))
        .send()
        .await?;

    let yaml_bytes = yaml_response.body.into_inner();
    let yaml_json: serde_json::Value = serde_json::from_slice(&yaml_bytes)?;

    let yaml_content = yaml_json["content"][0]["text"]
        .as_str()
        .unwrap_or("No YAML generated");

    info!("✅ Generated YAML Template:");
    for line in yaml_content.lines() {
        info!("   {}", line);
    }

    // Save YAML to file
    std::fs::write("demo_output.yaml", yaml_content)?;
    info!("💾 Saved to: demo_output.yaml");

    // STEP 5: Show metrics
    info!("\n📍 STEP 5: Bot Metrics Summary");
    info!("─────────────────────────────────────────────");

    let metrics = bot.metrics();
    info!("📊 Framework Metrics:");
    info!("   Total requests: {}", metrics.requests_total());
    info!("   Success rate: {:.1}%", metrics.success_rate());

    if let Some(avg_time) = metrics.average_response_time() {
        info!("   Avg response time: {:?}", avg_time);
    }

    info!("\n════════════════════════════════════════════════════════");
    info!("✅ DEMO COMPLETE!");
    info!("🎯 Successfully demonstrated:");
    info!("   1. Bot initialization with Claude Opus 4.1");
    info!("   2. Message processing through framework");
    info!("   3. Real AWS Bedrock API calls");
    info!("   4. YAML template generation (PDMT style)");
    info!("   5. Metrics and cost tracking");
    info!("════════════════════════════════════════════════════════");

    Ok(())
}
