//! Universal Bot Core - Enterprise AI Automation Framework
//!
//! This crate provides the core functionality for the Universal Bot framework,
//! including message pipelines, context management, and plugin architecture.
//!
//! # Example
//!
//! ```rust
//! use universal_bot_core::{Bot, BotConfig, Message};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = BotConfig::builder()
//!     .model("anthropic.claude-opus-4-1")
//!     .temperature(0.1)
//!     .build()?;
//!
//! let bot = Bot::new(config).await?;
//! let response = bot.process(Message::text("Hello")).await?;
//! println!("{}", response.content);
//! # Ok(())
//! # }
//! ```

#![warn(
    missing_docs,
    rust_2018_idioms,
    clippy::all,
    clippy::pedantic,
    clippy::nursery
)]
#![allow(clippy::module_name_repetitions, clippy::must_use_candidate)]

pub mod bot;
pub mod config;
pub mod context;
pub mod error;
pub mod message;
pub mod pipeline;
pub mod plugin;

// Re-exports
pub use bot::{Bot, BotBuilder};
pub use config::{BotConfig, BotConfigBuilder};
pub use context::{Context, ContextManager, ContextStore};
pub use error::{Error, Result};
pub use message::{Message, MessageType, Response};
pub use pipeline::{MessagePipeline, PipelineStage};
pub use plugin::{Plugin, PluginRegistry};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the library with default settings
///
/// This function sets up logging, tracing, and other global configurations.
///
/// # Errors
///
/// Returns an error if initialization fails.
///
/// # Example
///
/// ```rust
/// # fn main() -> anyhow::Result<()> {
/// universal_bot_core::init()?;
/// # Ok(())
/// # }
/// ```
pub fn init() -> Result<()> {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .try_init()
        .map_err(|e| Error::Initialization(e.to_string()))?;

    tracing::info!("Universal Bot Core v{} initialized", VERSION);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
        assert!(VERSION.contains('.'));
    }

    #[test]
    fn test_init() {
        // Initialize should work multiple times without error
        let _ = init();
        let _ = init();
    }
}
