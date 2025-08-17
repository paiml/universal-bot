//! Plugin system demonstration
//!
//! This example demonstrates the plugin architecture by creating custom plugins
//! and showing how they integrate with the bot framework.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example plugin_demo
//! ```

use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use universal_bot_core::{
    plugin::{
        Capability, CapabilityType, Permission, Plugin, PluginConfig, PluginRequest,
        PluginResponse, RequestType,
    },
    Bot, BotBuilder, BotConfig, Message,
};

/// Weather plugin example
struct WeatherPlugin {
    name: String,
    version: String,
    api_key: Option<String>,
}

impl WeatherPlugin {
    fn new() -> Self {
        Self {
            name: "weather".to_string(),
            version: "1.0.0".to_string(),
            api_key: None,
        }
    }
}

#[async_trait]
impl Plugin for WeatherPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn description(&self) -> &str {
        "Provides weather information for specified locations"
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability {
                name: "weather_query".to_string(),
                capability_type: CapabilityType::CommandHandler,
                description: "Handle weather-related queries".to_string(),
                required_permissions: vec![Permission::NetworkAccess, Permission::ReadMessages],
            },
            Capability {
                name: "location_lookup".to_string(),
                capability_type: CapabilityType::ToolProvider,
                description: "Look up location coordinates".to_string(),
                required_permissions: vec![Permission::NetworkAccess],
            },
        ]
    }

    async fn initialize(&mut self, config: PluginConfig) -> Result<()> {
        info!("Initializing Weather Plugin");

        // Extract API key from configuration
        if let Some(key) = config.settings.get("api_key") {
            if let Some(key_str) = key.as_str() {
                self.api_key = Some(key_str.to_string());
                info!("Weather API key configured");
            }
        }

        Ok(())
    }

    async fn process(&self, request: PluginRequest) -> Result<PluginResponse> {
        match request.request_type {
            RequestType::ProcessMessage => self.handle_message(request).await,
            RequestType::ExecuteCommand => self.handle_command(request).await,
            RequestType::InvokeTool => self.handle_tool(request).await,
            _ => Ok(PluginResponse::error(
                request.id,
                "Unsupported request type",
            )),
        }
    }

    fn can_handle(&self, message: &Message) -> bool {
        let content = message.content.to_lowercase();
        content.contains("weather")
            || content.contains("temperature")
            || content.contains("forecast")
    }
}

impl WeatherPlugin {
    async fn handle_message(&self, request: PluginRequest) -> Result<PluginResponse> {
        if let Ok(message) = serde_json::from_value::<Message>(request.data) {
            let location = self.extract_location(&message.content);
            let weather_info = self.get_weather(&location).await?;

            let response_message = Message::text(format!(
                "🌤️ Weather for {}: {}°C, {}",
                location, weather_info.temperature, weather_info.description
            ));

            Ok(PluginResponse::success(
                request.id,
                serde_json::to_value(response_message)?,
            ))
        } else {
            Ok(PluginResponse::error(request.id, "Invalid message format"))
        }
    }

    async fn handle_command(&self, request: PluginRequest) -> Result<PluginResponse> {
        let data = json!({
            "command": "weather",
            "result": "Weather command executed successfully"
        });

        Ok(PluginResponse::success(request.id, data))
    }

    async fn handle_tool(&self, request: PluginRequest) -> Result<PluginResponse> {
        let data = json!({
            "tool": "location_lookup",
            "coordinates": {"lat": 40.7128, "lon": -74.0060},
            "location": "New York, NY"
        });

        Ok(PluginResponse::success(request.id, data))
    }

    fn extract_location(&self, content: &str) -> String {
        // Simple location extraction (in practice, would use NLP)
        if content.to_lowercase().contains("new york") {
            "New York, NY".to_string()
        } else if content.to_lowercase().contains("london") {
            "London, UK".to_string()
        } else if content.to_lowercase().contains("tokyo") {
            "Tokyo, Japan".to_string()
        } else {
            "Unknown Location".to_string()
        }
    }

    async fn get_weather(&self, location: &str) -> Result<WeatherInfo> {
        // Simulate API call (in practice, would call real weather API)
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let weather = match location {
            "New York, NY" => WeatherInfo {
                temperature: 22,
                description: "Partly cloudy".to_string(),
                humidity: 65,
                wind_speed: 10.5,
            },
            "London, UK" => WeatherInfo {
                temperature: 15,
                description: "Rainy".to_string(),
                humidity: 80,
                wind_speed: 8.2,
            },
            "Tokyo, Japan" => WeatherInfo {
                temperature: 28,
                description: "Sunny".to_string(),
                humidity: 55,
                wind_speed: 5.1,
            },
            _ => WeatherInfo {
                temperature: 20,
                description: "Unknown conditions".to_string(),
                humidity: 50,
                wind_speed: 0.0,
            },
        };

        Ok(weather)
    }
}

/// Translation plugin example
struct TranslationPlugin {
    name: String,
    version: String,
    supported_languages: Vec<String>,
}

impl TranslationPlugin {
    fn new() -> Self {
        Self {
            name: "translation".to_string(),
            version: "1.0.0".to_string(),
            supported_languages: vec![
                "en".to_string(),
                "es".to_string(),
                "fr".to_string(),
                "de".to_string(),
                "ja".to_string(),
            ],
        }
    }
}

#[async_trait]
impl Plugin for TranslationPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn description(&self) -> &str {
        "Provides text translation between multiple languages"
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability {
            name: "text_translation".to_string(),
            capability_type: CapabilityType::MessageProcessor,
            description: "Translate text between languages".to_string(),
            required_permissions: vec![Permission::ReadMessages, Permission::WriteMessages],
        }]
    }

    async fn process(&self, request: PluginRequest) -> Result<PluginResponse> {
        match request.request_type {
            RequestType::ProcessMessage => self.handle_translation(request).await,
            RequestType::InvokeTool => self.handle_translation_tool(request).await,
            _ => Ok(PluginResponse::error(
                request.id,
                "Unsupported request type",
            )),
        }
    }

    fn can_handle(&self, message: &Message) -> bool {
        let content = message.content.to_lowercase();
        content.contains("translate") || content.starts_with("/translate")
    }
}

impl TranslationPlugin {
    async fn handle_translation(&self, request: PluginRequest) -> Result<PluginResponse> {
        if let Ok(message) = serde_json::from_value::<Message>(request.data) {
            let (text, target_lang) = self.parse_translation_request(&message.content);
            let translated = self.translate(&text, &target_lang).await?;

            let response_message =
                Message::text(format!("🔤 Translation to {}: {}", target_lang, translated));

            Ok(PluginResponse::success(
                request.id,
                serde_json::to_value(response_message)?,
            ))
        } else {
            Ok(PluginResponse::error(request.id, "Invalid message format"))
        }
    }

    async fn handle_translation_tool(&self, request: PluginRequest) -> Result<PluginResponse> {
        let data = json!({
            "tool": "translate",
            "supported_languages": self.supported_languages,
            "result": "Translation tool invoked"
        });

        Ok(PluginResponse::success(request.id, data))
    }

    fn parse_translation_request(&self, content: &str) -> (String, String) {
        // Simple parsing (in practice, would use proper command parsing)
        if content.starts_with("/translate") {
            let parts: Vec<&str> = content.split_whitespace().collect();
            if parts.len() >= 3 {
                let target_lang = parts[1].to_string();
                let text = parts[2..].join(" ");
                (text, target_lang)
            } else {
                (content.to_string(), "en".to_string())
            }
        } else {
            (content.to_string(), "en".to_string())
        }
    }

    async fn translate(&self, text: &str, target_lang: &str) -> Result<String> {
        // Simulate translation API call
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let translated = match (text, target_lang) {
            ("hello", "es") => "hola",
            ("hello", "fr") => "bonjour",
            ("hello", "de") => "hallo",
            ("hello", "ja") => "こんにちは",
            ("goodbye", "es") => "adiós",
            ("goodbye", "fr") => "au revoir",
            ("goodbye", "de") => "auf wiedersehen",
            ("goodbye", "ja") => "さようなら",
            _ => "Translation not available",
        };

        Ok(translated.to_string())
    }
}

/// Weather information structure
#[derive(Debug, Clone)]
struct WeatherInfo {
    temperature: i32,
    description: String,
    humidity: u32,
    wind_speed: f32,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Universal Bot Plugin Demo");

    // Create bot configuration
    let config = BotConfig::builder()
        .model("anthropic.claude-opus-4-1")
        .temperature(0.1)
        .max_tokens(1024)
        .build()?;

    // Create plugins
    let mut weather_plugin = WeatherPlugin::new();
    let weather_config = PluginConfig {
        settings: {
            let mut settings = HashMap::new();
            settings.insert("api_key".to_string(), json!("demo-api-key-12345"));
            settings
        },
        permissions: vec![Permission::NetworkAccess, Permission::ReadMessages],
        ..Default::default()
    };
    weather_plugin.initialize(weather_config).await?;

    let translation_plugin = TranslationPlugin::new();

    // Build bot with plugins
    let bot = BotBuilder::new()
        .config(config)
        .plugin(weather_plugin)
        .plugin(translation_plugin)
        .build()
        .await?;

    info!("Bot with plugins initialized successfully");

    // Test messages that will trigger different plugins
    let test_messages = vec![
        Message::text("What's the weather like in New York?"),
        Message::text("Can you translate hello to Spanish?"),
        Message::text("/translate es goodbye"),
        Message::text("What's the temperature in London?"),
        Message::text("Translate this to French: hello world"),
        Message::text("Is it raining in Tokyo?"),
        Message::text("How hot is it outside?"),
        Message::text("Regular message without plugin triggers"),
    ];

    // Process each message
    for (i, message) in test_messages.into_iter().enumerate() {
        info!("📨 Processing message {}: {}", i + 1, message.content);

        match bot.process(message).await {
            Ok(response) => {
                info!("✅ Response {}: {}", i + 1, response.content);

                if response.is_error() {
                    if let Some(error) = &response.error {
                        info!("⚠️ Response contains error: {}", error.message);
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Error processing message {}: {}", i + 1, e);
            }
        }

        // Add delay between messages
        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
    }

    // Display final metrics
    let metrics = bot.metrics();
    info!("📊 Final Plugin Demo Metrics:");
    info!("  Total requests: {}", metrics.requests_total());
    info!("  Success rate: {:.2}%", metrics.success_rate());

    if let Some(avg_time) = metrics.average_response_time() {
        info!("  Average response time: {:?}", avg_time);
    }

    info!("Universal Bot Plugin Demo completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_weather_plugin() {
        let mut plugin = WeatherPlugin::new();
        let config = PluginConfig::default();

        plugin.initialize(config).await.unwrap();

        let message = Message::text("What's the weather in New York?");
        assert!(plugin.can_handle(&message));

        let request = PluginRequest {
            id: "test".to_string(),
            request_type: RequestType::ProcessMessage,
            data: serde_json::to_value(&message).unwrap(),
            metadata: HashMap::new(),
        };

        let response = plugin.process(request).await.unwrap();
        assert!(response.success);
    }

    #[tokio::test]
    async fn test_translation_plugin() {
        let plugin = TranslationPlugin::new();

        let message = Message::text("/translate es hello");
        assert!(plugin.can_handle(&message));

        let (text, lang) = plugin.parse_translation_request(&message.content);
        assert_eq!(text, "hello");
        assert_eq!(lang, "es");

        let translated = plugin.translate("hello", "es").await.unwrap();
        assert_eq!(translated, "hola");
    }

    #[test]
    fn test_plugin_capabilities() {
        let weather_plugin = WeatherPlugin::new();
        let capabilities = weather_plugin.capabilities();

        assert_eq!(capabilities.len(), 2);
        assert_eq!(capabilities[0].name, "weather_query");

        let translation_plugin = TranslationPlugin::new();
        let capabilities = translation_plugin.capabilities();

        assert_eq!(capabilities.len(), 1);
        assert_eq!(capabilities[0].name, "text_translation");
    }
}
