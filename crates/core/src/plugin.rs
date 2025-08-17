//! Plugin system for extending bot functionality
//!
//! This module provides a plugin architecture that allows extending
//! the bot's capabilities without modifying core code.

use std::collections::HashMap;
use std::fmt;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument, warn};

use crate::{
    error::Error,
    message::{Message, Response},
};

/// Plugin trait for extending bot functionality
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Get the plugin name
    fn name(&self) -> &str;

    /// Get the plugin version
    fn version(&self) -> &str;

    /// Get plugin description
    fn description(&self) -> &str {
        "No description provided"
    }

    /// Get plugin capabilities
    fn capabilities(&self) -> Vec<Capability>;

    /// Initialize the plugin
    async fn initialize(&mut self, _config: PluginConfig) -> Result<()> {
        Ok(())
    }

    /// Process a request
    async fn process(&self, request: PluginRequest) -> Result<PluginResponse>;

    /// Shutdown the plugin
    async fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }

    /// Check if the plugin can handle a message
    fn can_handle(&self, _message: &Message) -> bool {
        true
    }

    /// Get plugin metadata
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: self.name().to_string(),
            version: self.version().to_string(),
            description: self.description().to_string(),
            author: None,
            homepage: None,
            license: None,
        }
    }
}

/// Plugin capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    /// Capability name
    pub name: String,
    /// Capability type
    pub capability_type: CapabilityType,
    /// Description
    pub description: String,
    /// Required permissions
    pub required_permissions: Vec<Permission>,
}

/// Type of capability
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityType {
    /// Message processing
    MessageProcessor,
    /// Command handler
    CommandHandler,
    /// Event listener
    EventListener,
    /// Tool provider
    ToolProvider,
    /// Middleware
    Middleware,
    /// Custom capability
    Custom(String),
}

/// Plugin permission
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// Read messages
    ReadMessages,
    /// Write messages
    WriteMessages,
    /// Access context
    AccessContext,
    /// Modify context
    ModifyContext,
    /// Make network requests
    NetworkAccess,
    /// Access filesystem
    FileSystemAccess,
    /// Execute commands
    ExecuteCommands,
    /// Access database
    DatabaseAccess,
    /// All permissions
    All,
}

/// Plugin configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Plugin-specific settings
    pub settings: HashMap<String, serde_json::Value>,
    /// Enabled features
    pub enabled_features: Vec<String>,
    /// Granted permissions
    pub permissions: Vec<Permission>,
    /// Resource limits
    pub resource_limits: ResourceLimits,
}

/// Resource limits for plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory in bytes
    pub max_memory: Option<usize>,
    /// Maximum CPU percentage
    pub max_cpu: Option<f32>,
    /// Maximum execution time
    pub max_execution_time: Option<std::time::Duration>,
    /// Maximum concurrent operations
    pub max_concurrent_ops: Option<usize>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory: Some(100 * 1024 * 1024), // 100MB
            max_cpu: Some(50.0),                 // 50%
            max_execution_time: Some(std::time::Duration::from_secs(30)),
            max_concurrent_ops: Some(10),
        }
    }
}

/// Plugin request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRequest {
    /// Request ID
    pub id: String,
    /// Request type
    pub request_type: RequestType,
    /// Request data
    pub data: serde_json::Value,
    /// Request metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Request type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestType {
    /// Process a message
    ProcessMessage,
    /// Execute a command
    ExecuteCommand,
    /// Handle an event
    HandleEvent,
    /// Invoke a tool
    InvokeTool,
    /// Custom request
    Custom(String),
}

/// Plugin response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResponse {
    /// Response ID
    pub id: String,
    /// Success status
    pub success: bool,
    /// Response data
    pub data: serde_json::Value,
    /// Error message if failed
    pub error: Option<String>,
    /// Response metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl PluginResponse {
    /// Create a successful response
    pub fn success(id: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            success: true,
            data,
            error: None,
            metadata: HashMap::new(),
        }
    }

    /// Create an error response
    pub fn error(id: impl Into<String>, error: impl fmt::Display) -> Self {
        Self {
            id: id.into(),
            success: false,
            data: serde_json::Value::Null,
            error: Some(error.to_string()),
            metadata: HashMap::new(),
        }
    }
}

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Plugin name
    pub name: String,
    /// Plugin version
    pub version: String,
    /// Plugin description
    pub description: String,
    /// Plugin author
    pub author: Option<String>,
    /// Plugin homepage
    pub homepage: Option<String>,
    /// Plugin license
    pub license: Option<String>,
}

/// Plugin registry for managing plugins
pub struct PluginRegistry {
    plugins: HashMap<String, Box<dyn Plugin>>,
    hooks: HashMap<HookType, Vec<String>>,
    permissions: HashMap<String, Vec<Permission>>,
}

impl PluginRegistry {
    /// Create a new plugin registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            hooks: HashMap::new(),
            permissions: HashMap::new(),
        }
    }

    /// Register a plugin
    ///
    /// # Errors
    ///
    /// Returns an error if a plugin with the same name already exists.
    #[instrument(skip(self, plugin))]
    pub fn register(&mut self, mut plugin: Box<dyn Plugin>) -> Result<()> {
        let name = plugin.name().to_string();

        if self.plugins.contains_key(&name) {
            return Err(Error::Plugin(format!("Plugin '{name}' already registered")).into());
        }

        info!("Registering plugin: {} v{}", name, plugin.version());

        // Initialize plugin with default config
        let config = PluginConfig::default();
        futures::executor::block_on(plugin.initialize(config))?;

        // Register capabilities
        for capability in plugin.capabilities() {
            self.register_hook(&name, &capability);
        }

        self.plugins.insert(name.clone(), plugin);
        self.permissions.insert(name, vec![Permission::All]);

        Ok(())
    }

    /// Unregister a plugin
    #[instrument(skip(self))]
    pub async fn unregister(&mut self, name: &str) -> Result<()> {
        if let Some(mut plugin) = self.plugins.remove(name) {
            info!("Unregistering plugin: {}", name);
            plugin.shutdown().await?;

            // Remove from hooks
            for hooks in self.hooks.values_mut() {
                hooks.retain(|n| n != name);
            }

            self.permissions.remove(name);
            Ok(())
        } else {
            Err(Error::NotFound(format!("Plugin '{name}' not found")).into())
        }
    }

    /// Get a plugin by name
    pub fn get(&self, name: &str) -> Option<&dyn Plugin> {
        self.plugins.get(name).map(std::convert::AsRef::as_ref)
    }

    /// List all registered plugins
    pub fn list(&self) -> Vec<PluginMetadata> {
        self.plugins.values().map(|p| p.metadata()).collect()
    }

    /// Apply pre-processing plugins
    #[instrument(skip(self, message))]
    pub async fn apply_pre_processing(&self, mut message: Message) -> Result<Message> {
        for plugin in self.plugins.values() {
            if plugin.can_handle(&message) {
                let request = PluginRequest {
                    id: uuid::Uuid::new_v4().to_string(),
                    request_type: RequestType::ProcessMessage,
                    data: serde_json::to_value(&message)?,
                    metadata: HashMap::new(),
                };

                match plugin.process(request).await {
                    Ok(response) if response.success => {
                        if let Ok(processed) = serde_json::from_value(response.data) {
                            message = processed;
                        }
                    }
                    Ok(response) => {
                        warn!(
                            "Plugin {} failed to process message: {:?}",
                            plugin.name(),
                            response.error
                        );
                    }
                    Err(e) => {
                        warn!("Plugin {} error: {}", plugin.name(), e);
                    }
                }
            }
        }

        Ok(message)
    }

    /// Apply post-processing plugins
    #[instrument(skip(self, response))]
    pub async fn apply_post_processing(&self, mut response: Response) -> Result<Response> {
        for plugin in self.plugins.values() {
            let request = PluginRequest {
                id: uuid::Uuid::new_v4().to_string(),
                request_type: RequestType::Custom("post_process".to_string()),
                data: serde_json::to_value(&response)?,
                metadata: HashMap::new(),
            };

            match plugin.process(request).await {
                Ok(plugin_response) if plugin_response.success => {
                    if let Ok(processed) = serde_json::from_value(plugin_response.data) {
                        response = processed;
                    }
                }
                Ok(plugin_response) => {
                    debug!(
                        "Plugin {} post-processing failed: {:?}",
                        plugin.name(),
                        plugin_response.error
                    );
                }
                Err(e) => {
                    debug!("Plugin {} post-processing error: {}", plugin.name(), e);
                }
            }
        }

        Ok(response)
    }

    /// Check if a plugin has permission
    pub fn has_permission(&self, plugin_name: &str, permission: &Permission) -> bool {
        self.permissions
            .get(plugin_name)
            .is_some_and(|perms| perms.contains(permission) || perms.contains(&Permission::All))
    }

    // Private helper methods

    fn register_hook(&mut self, plugin_name: &str, capability: &Capability) {
        let hook_type = match &capability.capability_type {
            CapabilityType::MessageProcessor => HookType::MessageProcessor,
            CapabilityType::CommandHandler => HookType::CommandHandler,
            CapabilityType::EventListener => HookType::EventListener,
            CapabilityType::ToolProvider => HookType::ToolProvider,
            CapabilityType::Middleware => HookType::Middleware,
            CapabilityType::Custom(name) => HookType::Custom(name.clone()),
        };

        self.hooks
            .entry(hook_type)
            .or_default()
            .push(plugin_name.to_string());
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Hook type for plugin registration
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HookType {
    /// Message processor hook
    MessageProcessor,
    /// Command handler hook
    CommandHandler,
    /// Event listener hook
    EventListener,
    /// Tool provider hook
    ToolProvider,
    /// Middleware hook
    Middleware,
    /// Custom hook
    Custom(String),
}

/// Example plugin implementation
pub struct EchoPlugin {
    name: String,
    version: String,
}

impl EchoPlugin {
    /// Create a new echo plugin
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "echo".to_string(),
            version: "1.0.0".to_string(),
        }
    }
}

impl Default for EchoPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for EchoPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn description(&self) -> &'static str {
        "Simple echo plugin for testing"
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability {
            name: "echo".to_string(),
            capability_type: CapabilityType::MessageProcessor,
            description: "Echoes messages back".to_string(),
            required_permissions: vec![Permission::ReadMessages, Permission::WriteMessages],
        }]
    }

    async fn process(&self, request: PluginRequest) -> Result<PluginResponse> {
        match request.request_type {
            RequestType::ProcessMessage => {
                if let Ok(message) = serde_json::from_value::<Message>(request.data) {
                    let echo_message = Message::text(format!("Echo: {}", message.content));
                    Ok(PluginResponse::success(
                        request.id,
                        serde_json::to_value(echo_message)?,
                    ))
                } else {
                    Ok(PluginResponse::error(request.id, "Invalid message data"))
                }
            }
            _ => Ok(PluginResponse::error(
                request.id,
                "Unsupported request type",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_registry() {
        let mut registry = PluginRegistry::new();
        let plugin = Box::new(EchoPlugin::new());

        assert!(registry.register(plugin).is_ok());
        assert!(registry.get("echo").is_some());

        let plugins = registry.list();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "echo");
    }

    #[tokio::test]
    async fn test_plugin_unregister() {
        let mut registry = PluginRegistry::new();
        let plugin = Box::new(EchoPlugin::new());

        registry.register(plugin).unwrap();
        assert!(registry.unregister("echo").await.is_ok());
        assert!(registry.get("echo").is_none());
    }

    #[test]
    fn test_plugin_permissions() {
        let mut registry = PluginRegistry::new();
        let plugin = Box::new(EchoPlugin::new());

        // Before registering, plugin shouldn't have permissions
        assert!(!registry.has_permission("echo", &Permission::All));

        // After registering, plugin gets all permissions by default
        registry.register(plugin).unwrap();
        assert!(registry.has_permission("echo", &Permission::All));
        assert!(registry.has_permission("echo", &Permission::ReadMessages));

        // Non-existent plugins don't have permissions
        assert!(!registry.has_permission("nonexistent", &Permission::ReadMessages));
    }

    #[tokio::test]
    async fn test_echo_plugin() {
        let plugin = EchoPlugin::new();
        let message = Message::text("Hello, world!");

        let request = PluginRequest {
            id: "test-123".to_string(),
            request_type: RequestType::ProcessMessage,
            data: serde_json::to_value(message).unwrap(),
            metadata: HashMap::new(),
        };

        let response = plugin.process(request).await.unwrap();
        assert!(response.success);

        let echo_message: Message = serde_json::from_value(response.data).unwrap();
        assert_eq!(echo_message.content, "Echo: Hello, world!");
    }

    #[test]
    fn test_plugin_response() {
        let response = PluginResponse::success("test", serde_json::json!({"key": "value"}));
        assert!(response.success);
        assert!(response.error.is_none());

        let error_response = PluginResponse::error("test", "Something went wrong");
        assert!(!error_response.success);
        assert_eq!(
            error_response.error.as_deref(),
            Some("Something went wrong")
        );
    }
}
