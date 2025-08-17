//! Model definitions and utilities

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported Claude models on Bedrock
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClaudeModel {
    /// Claude 3.5 Sonnet
    #[serde(rename = "anthropic.claude-3-5-sonnet-20241022-v2:0")]
    Claude35Sonnet,
    /// Claude 3 Opus
    #[serde(rename = "anthropic.claude-3-opus-20240229-v1:0")]
    Claude3Opus,
    /// Claude 3 Haiku
    #[serde(rename = "anthropic.claude-3-haiku-20240307-v1:0")]
    Claude3Haiku,
}

impl ClaudeModel {
    /// Get the model identifier string
    pub fn id(&self) -> &'static str {
        match self {
            Self::Claude35Sonnet => "anthropic.claude-3-5-sonnet-20241022-v2:0",
            Self::Claude3Opus => "anthropic.claude-3-opus-20240229-v1:0",
            Self::Claude3Haiku => "anthropic.claude-3-haiku-20240307-v1:0",
        }
    }

    /// Get a human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Claude35Sonnet => "Claude 3.5 Sonnet",
            Self::Claude3Opus => "Claude 3 Opus",
            Self::Claude3Haiku => "Claude 3 Haiku",
        }
    }

    /// Get model capabilities
    pub fn capabilities(&self) -> ModelCapabilities {
        match self {
            Self::Claude35Sonnet => ModelCapabilities {
                max_tokens: 200_000,
                context_window: 200_000,
                supports_vision: true,
                supports_function_calling: true,
                input_cost_per_1k_tokens: 0.003,
                output_cost_per_1k_tokens: 0.015,
                description: "Most capable model for complex reasoning and analysis".to_string(),
            },
            Self::Claude3Opus => ModelCapabilities {
                max_tokens: 200_000,
                context_window: 200_000,
                supports_vision: true,
                supports_function_calling: true,
                input_cost_per_1k_tokens: 0.015,
                output_cost_per_1k_tokens: 0.075,
                description: "Most powerful model for complex tasks".to_string(),
            },
            Self::Claude3Haiku => ModelCapabilities {
                max_tokens: 200_000,
                context_window: 200_000,
                supports_vision: true,
                supports_function_calling: false,
                input_cost_per_1k_tokens: 0.00025,
                output_cost_per_1k_tokens: 0.00125,
                description: "Fastest and most cost-effective model".to_string(),
            },
        }
    }

    /// Get all available models
    pub fn all() -> Vec<Self> {
        vec![Self::Claude35Sonnet, Self::Claude3Opus, Self::Claude3Haiku]
    }

    /// Parse from model ID string
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "anthropic.claude-3-5-sonnet-20241022-v2:0" => Some(Self::Claude35Sonnet),
            "anthropic.claude-3-opus-20240229-v1:0" => Some(Self::Claude3Opus),
            "anthropic.claude-3-haiku-20240307-v1:0" => Some(Self::Claude3Haiku),
            _ => None,
        }
    }

    /// Get recommended model for a task type
    pub fn recommend_for_task(task: TaskType) -> Self {
        match task {
            TaskType::CodeGeneration => Self::Claude35Sonnet,
            TaskType::Analysis => Self::Claude35Sonnet,
            TaskType::CreativeWriting => Self::Claude3Opus,
            TaskType::QuestionAnswering => Self::Claude3Haiku,
            TaskType::Summarization => Self::Claude3Haiku,
            TaskType::Translation => Self::Claude35Sonnet,
            TaskType::Reasoning => Self::Claude3Opus,
        }
    }
}

impl std::fmt::Display for ClaudeModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Model capabilities and pricing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    /// Maximum tokens that can be generated
    pub max_tokens: usize,
    /// Context window size
    pub context_window: usize,
    /// Whether the model supports vision tasks
    pub supports_vision: bool,
    /// Whether the model supports function calling
    pub supports_function_calling: bool,
    /// Input cost per 1K tokens in USD
    pub input_cost_per_1k_tokens: f64,
    /// Output cost per 1K tokens in USD
    pub output_cost_per_1k_tokens: f64,
    /// Model description
    pub description: String,
}

/// Task types for model recommendation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    /// Code generation and programming tasks
    CodeGeneration,
    /// Data analysis and research
    Analysis,
    /// Creative writing and content generation
    CreativeWriting,
    /// Question answering
    QuestionAnswering,
    /// Text summarization
    Summarization,
    /// Language translation
    Translation,
    /// Complex reasoning tasks
    Reasoning,
}

/// Model registry for managing available models
#[derive(Debug, Clone)]
pub struct ModelRegistry {
    models: HashMap<String, ModelInfo>,
}

/// Information about a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Model capabilities
    pub capabilities: ModelCapabilities,
    /// Whether the model is currently available
    pub available: bool,
    /// Model version
    pub version: String,
    /// Provider (e.g., "anthropic")
    pub provider: String,
}

impl ModelRegistry {
    /// Create a new model registry with default Claude models
    pub fn new() -> Self {
        let mut registry = Self {
            models: HashMap::new(),
        };

        // Register Claude models
        for model in ClaudeModel::all() {
            let info = ModelInfo {
                id: model.id().to_string(),
                name: model.name().to_string(),
                capabilities: model.capabilities(),
                available: true,
                version: "1.0".to_string(),
                provider: "anthropic".to_string(),
            };
            registry.models.insert(model.id().to_string(), info);
        }

        registry
    }

    /// Get model information by ID
    pub fn get(&self, id: &str) -> Option<&ModelInfo> {
        self.models.get(id)
    }

    /// List all available models
    pub fn list_available(&self) -> Vec<&ModelInfo> {
        self.models.values().filter(|m| m.available).collect()
    }

    /// Register a new model
    pub fn register(&mut self, info: ModelInfo) {
        self.models.insert(info.id.clone(), info);
    }

    /// Mark a model as unavailable
    pub fn mark_unavailable(&mut self, id: &str) {
        if let Some(model) = self.models.get_mut(id) {
            model.available = false;
        }
    }

    /// Get models that support a specific capability
    pub fn models_with_capability(&self, capability: ModelCapability) -> Vec<&ModelInfo> {
        self.models
            .values()
            .filter(|m| m.available && self.supports_capability(m, capability))
            .collect()
    }

    fn supports_capability(&self, model: &ModelInfo, capability: ModelCapability) -> bool {
        match capability {
            ModelCapability::Vision => model.capabilities.supports_vision,
            ModelCapability::FunctionCalling => model.capabilities.supports_function_calling,
            ModelCapability::LargeContext => model.capabilities.context_window >= 100_000,
            ModelCapability::LowCost => {
                model.capabilities.input_cost_per_1k_tokens < 0.001
                    && model.capabilities.output_cost_per_1k_tokens < 0.002
            }
        }
    }
}

/// Model capabilities for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelCapability {
    /// Vision/image processing support
    Vision,
    /// Function calling support
    FunctionCalling,
    /// Large context window (>100k tokens)
    LargeContext,
    /// Low cost (< $0.001 input, < $0.002 output per 1k tokens)
    LowCost,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_model_ids() {
        assert_eq!(
            ClaudeModel::Claude35Sonnet.id(),
            "anthropic.claude-3-5-sonnet-20241022-v2:0"
        );
        assert_eq!(
            ClaudeModel::Claude3Opus.id(),
            "anthropic.claude-3-opus-20240229-v1:0"
        );
        assert_eq!(
            ClaudeModel::Claude3Haiku.id(),
            "anthropic.claude-3-haiku-20240307-v1:0"
        );
    }

    #[test]
    fn test_model_parsing() {
        assert_eq!(
            ClaudeModel::from_id("anthropic.claude-3-5-sonnet-20241022-v2:0"),
            Some(ClaudeModel::Claude35Sonnet)
        );
        assert_eq!(ClaudeModel::from_id("invalid-model"), None);
    }

    #[test]
    fn test_task_recommendations() {
        assert_eq!(
            ClaudeModel::recommend_for_task(TaskType::CodeGeneration),
            ClaudeModel::Claude35Sonnet
        );
        assert_eq!(
            ClaudeModel::recommend_for_task(TaskType::QuestionAnswering),
            ClaudeModel::Claude3Haiku
        );
    }

    #[test]
    fn test_model_registry() {
        let registry = ModelRegistry::new();

        let available_models = registry.list_available();
        assert_eq!(available_models.len(), 3);

        let sonnet_info = registry.get(ClaudeModel::Claude35Sonnet.id()).unwrap();
        assert_eq!(sonnet_info.name, "Claude 3.5 Sonnet");
        assert!(sonnet_info.capabilities.supports_vision);
    }

    #[test]
    fn test_capability_filtering() {
        let registry = ModelRegistry::new();

        let vision_models = registry.models_with_capability(ModelCapability::Vision);
        assert_eq!(vision_models.len(), 3); // All Claude models support vision

        let low_cost_models = registry.models_with_capability(ModelCapability::LowCost);
        assert_eq!(low_cost_models.len(), 1); // Only Haiku is low cost
    }
}
