//! AWS Bedrock Integration with Claude Opus 4.1 and YAML Templates
//!
//! This example demonstrates the Universal Bot framework working with:
//! - AWS Bedrock using Claude Opus 4.1 model
//! - YAML template generation following PDMT patterns

use anyhow::Result;
use aws_config::BehaviorVersion;
use aws_sdk_bedrockruntime::{primitives::Blob, Client as BedrockClient};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_yaml;
use tracing::{debug, info, warn, Level};
use tracing_subscriber::FmtSubscriber;
use universal_bot_core::{Bot, BotConfig};

// Default Claude Opus 4.1 model ID - MUST use inference profile for on-demand
const DEFAULT_BEDROCK_MODEL_ID: &str = "us.anthropic.claude-opus-4-1-20250805-v1:0";

#[derive(Debug, Serialize, Deserialize)]
struct TodoItem {
    id: String,
    content: String,
    status: String,
    priority: String,
    estimated_hours: f32,
    dependencies: Vec<String>,
    tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TodoTemplate {
    todos: Vec<TodoItem>,
    metadata: TodoMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
struct TodoMetadata {
    total_count: usize,
    generated_at: String,
    template_version: String,
    granularity: String,
    project_name: String,
    model_used: String,
}

/// Generate content using Claude Opus 4.1 on AWS Bedrock
async fn generate_with_bedrock(prompt: &str) -> Result<String> {
    info!("🚀 Calling AWS Bedrock with Claude Opus 4.1");
    info!("📍 Model ID: {}", DEFAULT_BEDROCK_MODEL_ID);

    // Initialize AWS SDK
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region("us-east-1")
        .load()
        .await;

    let client = BedrockClient::new(&config);

    // Format request for Claude Messages API (required for Claude 3+ and Opus 4.1)
    let request = json!({
        "anthropic_version": "bedrock-2023-05-31",
        "max_tokens": 2048,
        "temperature": 0.1,  // Low temperature for deterministic output (PDMT style)
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ]
    });

    debug!("Request payload: {:?}", request);

    // Make the API call to Bedrock
    let response = client
        .invoke_model()
        .body(Blob::new(request.to_string()))
        .model_id(DEFAULT_BEDROCK_MODEL_ID)
        .content_type("application/json")
        .accept("application/json")
        .send()
        .await
        .map_err(|e| {
            warn!("Bedrock API call failed: {:?}", e);
            anyhow::anyhow!("Bedrock API error: {}", e)
        })?;

    // Parse response
    let response_bytes = response.body.into_inner();
    let response_body = String::from_utf8(response_bytes)?;
    let response_json: serde_json::Value = serde_json::from_str(&response_body)?;

    // Extract content from Claude's response
    let content = response_json["content"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|c| c["text"].as_str())
        .ok_or_else(|| anyhow::anyhow!("No content in response"))?;

    // Log token usage if available
    if let Some(usage) = response_json["usage"].as_object() {
        info!("📊 Token Usage:");
        info!(
            "  Input: {}",
            usage
                .get("input_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
        );
        info!(
            "  Output: {}",
            usage
                .get("output_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
        );
    }

    Ok(content.to_string())
}

/// Generate a YAML todo list template using Claude Opus 4.1
async fn generate_todo_yaml(project_description: &str) -> Result<TodoTemplate> {
    let prompt = format!(
        r#"Generate a structured todo list in YAML format for the following project:

Project: {}

Create 5-8 specific, actionable todo items with:
- Unique IDs (format: todo_X_Y)
- Clear, specific content descriptions
- Status (all "pending")
- Priority levels (high/medium/low)
- Estimated hours
- Dependencies between tasks
- Relevant tags

Return ONLY valid YAML following this exact structure:

todos:
  - id: "todo_0_0"
    content: "Specific task description"
    status: "pending"
    priority: "high"
    estimated_hours: 4.0
    dependencies: []
    tags: ["tag1", "tag2"]

metadata:
  total_count: X
  generated_at: "ISO8601_TIMESTAMP"
  template_version: "1.0.0"
  granularity: "high"
  project_name: "PROJECT_NAME"
  model_used: "Claude Opus 4.1"

Do not include any explanations or markdown formatting. Output only the YAML."#,
        project_description
    );

    let yaml_content = generate_with_bedrock(&prompt).await?;

    // Clean up the response (remove any markdown if present)
    let yaml_content = yaml_content
        .trim()
        .trim_start_matches("```yaml")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Parse YAML
    let todo_template: TodoTemplate = serde_yaml::from_str(yaml_content)?;

    Ok(todo_template)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("═══════════════════════════════════════════════════════");
    info!("🤖 Universal Bot - AWS Bedrock Claude Opus 4.1 Demo");
    info!("📝 YAML Template Generation (PDMT Style)");
    info!("═══════════════════════════════════════════════════════");

    // Initialize Universal Bot with Claude Opus 4.1 configuration
    let config = BotConfig::builder()
        .model("us.anthropic.claude-opus-4-1-20250805-v1:0") // ONLY Opus 4.1
        .temperature(0.1) // Low temperature for deterministic output
        .max_tokens(2048)
        .enable_logging(true)
        .enable_cost_tracking(true)
        .build()?;

    let bot = Bot::new(config).await?;
    info!("✅ Bot initialized with Claude Opus 4.1 configuration");

    // Test projects for todo generation
    let test_projects = vec![
        "Build a REST API for user authentication with JWT tokens",
        "Create a machine learning pipeline for sentiment analysis",
        "Develop a mobile app for tracking fitness goals",
    ];

    info!("\n🧪 Starting YAML Template Generation Tests:\n");

    for (i, project) in test_projects.iter().enumerate() {
        info!("═══════════════════════════════════════════════════════");
        info!("📋 Project {}: {}", i + 1, project);
        info!("═══════════════════════════════════════════════════════");

        match generate_todo_yaml(project).await {
            Ok(template) => {
                info!("✅ Successfully generated todo template");
                info!("📊 Stats:");
                info!("  Total todos: {}", template.todos.len());
                info!("  Project: {}", template.metadata.project_name);
                info!("  Model: {}", template.metadata.model_used);

                // Display the todos
                info!("\n📝 Generated Todos:");
                for todo in &template.todos {
                    info!(
                        "  [{:6}] {} ({}h)",
                        todo.priority, todo.content, todo.estimated_hours
                    );
                    if !todo.dependencies.is_empty() {
                        info!("           Dependencies: {:?}", todo.dependencies);
                    }
                    info!("           Tags: {:?}", todo.tags);
                }

                // Save to YAML file
                let filename = format!("todo_project_{}.yaml", i + 1);
                let yaml_output = serde_yaml::to_string(&template)?;
                std::fs::write(&filename, yaml_output)?;
                info!("\n💾 Saved to: {}", filename);
            }
            Err(e) => {
                warn!("❌ Failed to generate template: {}", e);
                info!("💡 Note: Ensure AWS credentials are configured and you have access to Claude Opus 4.1");
            }
        }

        info!("");
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    }

    // Test with Universal Bot framework
    info!("═══════════════════════════════════════════════════════");
    info!("🔧 Testing Universal Bot Framework Integration");
    info!("═══════════════════════════════════════════════════════");

    let test_message = universal_bot_core::Message::text(
        "Create a todo list for building a microservices architecture",
    );

    match bot.process(test_message).await {
        Ok(response) => {
            info!("✅ Bot Response: {}", response.content);

            if let Some(usage) = &response.usage {
                info!("💰 Cost Tracking:");
                info!("  Input tokens: {}", usage.input_tokens);
                info!("  Output tokens: {}", usage.output_tokens);
                info!("  Estimated cost: ${:.4}", usage.estimated_cost);
            }
        }
        Err(e) => {
            warn!("❌ Bot Error: {}", e);
        }
    }

    // Display final metrics
    let metrics = bot.metrics();
    info!("\n═══════════════════════════════════════════════════════");
    info!("📊 Final Metrics:");
    info!("  Total requests: {}", metrics.requests_total());
    info!("  Success rate: {:.2}%", metrics.success_rate());

    if let Some(avg_time) = metrics.average_response_time() {
        info!("  Average response time: {:?}", avg_time);
    }

    info!("═══════════════════════════════════════════════════════");
    info!("✅ Demo Complete!");
    info!("🎯 Successfully demonstrated:");
    info!("  • AWS Bedrock integration with Claude Opus 4.1");
    info!("  • YAML template generation (PDMT style)");
    info!("  • Universal Bot framework with real AI responses");
    info!("═══════════════════════════════════════════════════════");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_id_format() {
        // Verify model ID follows expected pattern - MUST be Opus 4.1 inference profile
        assert!(DEFAULT_BEDROCK_MODEL_ID.starts_with("us.anthropic.claude-opus-4"));
        assert!(DEFAULT_BEDROCK_MODEL_ID.ends_with("-v1:0"));
    }

    #[test]
    fn test_yaml_serialization() {
        let todo = TodoItem {
            id: "todo_0_0".to_string(),
            content: "Test task".to_string(),
            status: "pending".to_string(),
            priority: "high".to_string(),
            estimated_hours: 2.0,
            dependencies: vec![],
            tags: vec!["test".to_string()],
        };

        let yaml = serde_yaml::to_string(&todo).unwrap();
        assert!(yaml.contains("todo_0_0"));
        assert!(yaml.contains("Test task"));
    }
}
