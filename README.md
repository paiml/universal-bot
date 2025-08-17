# Universal Bot - Enterprise AI Automation Framework

[![Crates.io](https://img.shields.io/crates/v/universal-bot-core.svg)](https://crates.io/crates/universal-bot-core)
[![Documentation](https://docs.rs/universal-bot-core/badge.svg)](https://docs.rs/universal-bot-core)
[![Build Status](https://github.com/paiml/universal-bot/actions/workflows/ci.yml/badge.svg)](https://github.com/paiml/universal-bot/actions)
[![Coverage](https://img.shields.io/badge/coverage-85%25-green.svg)](https://github.com/paiml/universal-bot/coverage)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![AWS Bedrock](https://img.shields.io/badge/AWS_Bedrock-Production-yellow.svg)](https://aws.amazon.com/bedrock/)
[![Downloads](https://img.shields.io/crates/d/universal-bot-core.svg)](https://crates.io/crates/universal-bot-core)

> **Enterprise-grade AI automation framework** integrating AWS Bedrock, PDMT templating, and AssetGen content generation into a unified, production-ready platform.

## 🚀 Overview

Universal Bot is a comprehensive AI-powered automation framework that combines cutting-edge technologies from production systems:

- **AWS Bedrock Integration**: Enterprise-grade AI model orchestration with Claude Opus 4.1, Sonnet 4, and more
- **PDMT Templating**: Deterministic, reproducible content generation with quality gates
- **AssetGen Engine**: Automated educational and marketing content creation
- **MCP Protocol**: Native Model Context Protocol support for AI assistants
- **Production-Ready**: 85%+ test coverage, property testing, and comprehensive validation

## 🏗️ Core Technologies

### AWS Bedrock Runtime
- **Connection Pooling**: Enterprise-grade connection management with retry logic
- **Model Orchestra**: Claude Opus 4.1, Sonnet 4, Llama, Titan multi-model support
- **Token Management**: Automatic usage tracking and cost optimization
- **Streaming**: Real-time response streaming for interactive applications
- **Metrics**: Comprehensive observability with latency and success tracking

### PDMT (Pragmatic Deterministic MCP Templating)
- **Zero-Temperature Generation**: Reproducible outputs with deterministic templates
- **Quality Gates**: PMAT enforcement with coverage, complexity, and SATD detection
- **Todo Validation**: Actionability scoring and dependency analysis
- **MCP Native**: Full Model Context Protocol support via PMCP SDK

### AssetGen Content Engine
- **Multi-Format Generation**: Quizzes, labs, blog posts, marketing content
- **Platform-Specific**: MailChimp, LinkedIn, Discord, Bluesky optimized content
- **Meta-Aware Validation**: Automatic detection and removal of transcript artifacts
- **GitHub Integration**: Automated publishing pipeline with issue tracking

## 📦 Key Features

### AI Model Integration
- ✅ **AWS Bedrock Runtime**: Production-ready with retry logic and pooling
- ✅ **Multi-Model Support**: Claude, Llama, Titan, Jurassic orchestration
- ✅ **Token Optimization**: Automatic counting and cost tracking
- ✅ **Streaming Responses**: Real-time token generation

### Content Generation
- ✅ **Educational Materials**: Automated quiz, lab, and reflection creation
- ✅ **Marketing Content**: Platform-specific for MailChimp, LinkedIn, Discord
- ✅ **Blog Generation**: Technical blogs with Zola/Hugo support
- ✅ **Transcription Processing**: Whisper.cpp integration

### Quality Assurance
- ✅ **Property Testing**: 100+ property tests ensuring robustness
- ✅ **Coverage Tracking**: 85%+ test coverage with reporting
- ✅ **Validation Pipeline**: Multi-stage validation with quality gates
- ✅ **CI/CD Integration**: GitHub Actions and self-hosted runners

## 🏗️ Architecture

```
universal-bot/
├── core/                    # Core Rust components
│   ├── bedrock/            # AWS Bedrock client implementation
│   │   ├── client.rs       # Connection pooling & retry logic
│   │   ├── models.rs       # Model configuration & selection
│   │   └── metrics.rs      # Token usage & cost tracking
│   ├── pdmt/               # Template engine
│   │   ├── engine.rs       # Handlebars-based generation
│   │   ├── validators.rs   # Todo validation & scoring
│   │   └── quality.rs      # Quality gate enforcement
│   └── providers/          # AI provider abstractions
│       ├── bedrock.rs      # AWS Bedrock provider
│       └── mcp.rs          # MCP protocol provider
├── generators/             # Content generation modules
│   ├── quiz/              # Quiz generation with validation
│   ├── blog/              # Blog post generation
│   ├── marketing/         # Multi-platform marketing
│   └── educational/       # Labs, reflections, key terms
├── validators/            # Validation components
│   ├── content.rs        # Meta-aware content validation
│   ├── quality.rs        # Quality gate checks
│   └── structure.rs      # Course structure validation
└── integrations/         # External integrations
    ├── github/          # GitHub API & Actions
    ├── aws/             # S3, Bedrock services
    └── mcp/             # Model Context Protocol
```

## 🛠️ Prerequisites

### Required
- Rust 1.75+ (latest stable)
- AWS Account with Bedrock access
- 8GB RAM minimum
- 2GB disk space

### Optional
- Node.js 18+ (for TypeScript integrations)
- Docker (for containerized deployment)
- GitHub account (for CI/CD)

## 📦 Installation

### From crates.io
```toml
[dependencies]
universal-bot-core = "1.0"
```

### From source
```bash
# Clone the repository
git clone https://github.com/paiml/universal-bot
cd universal-bot

# Install dependencies
make install

# Configure AWS credentials
aws configure

# Run tests to verify setup
make test

# Start the bot
cargo run --bin universal-bot
```

## 📁 Repository Structure

```
universal-bot-rust/
│
├── 📦 core/                        # The 80% - Universal AI Brain
│   ├── src/
│   │   ├── bedrock/               # AWS Bedrock client & models
│   │   │   ├── client.rs         # Connection management
│   │   │   ├── models.rs         # Model orchestration
│   │   │   └── streaming.rs      # Real-time responses
│   │   │
│   │   ├── conversation/         # Conversation engine
│   │   │   ├── pipeline.rs      # Message processing
│   │   │   ├── state.rs         # State machines
│   │   │   └── context.rs       # Memory management
│   │   │
│   │   ├── plugins/              # Extension system
│   │   │   ├── traits.rs        # Plugin interfaces
│   │   │   ├── registry.rs      # Plugin management
│   │   │   └── builtin/         # Core plugins
│   │   │
│   │   └── lib.rs               # Public API
│   │
│   ├── examples/                 # Runnable demonstrations
│   │   ├── basic_conversation.rs
│   │   ├── multi_model_chat.rs
│   │   ├── stream_conversation.rs
│   │   └── plugin_demo.rs
│   │
│   └── Cargo.toml               # Dependencies
│
├── 📚 adapters/                   # The 20% - Platform Theory
│   ├── discord_theory.md        # Discord architecture (no code)
│   ├── slack_theory.md          # Slack patterns (conceptual)
│   ├── web_api_theory.md        # REST endpoints (theory)
│   └── integration_patterns.md  # General adapter design
│
├── 🎓 course/                    # Video course materials
│   ├── module_1/                # Foundation videos
│   ├── module_2/                # Core engine videos
│   ├── module_3/                # Advanced patterns
│   ├── module_4/                # Plugin architecture
│   └── module_5/                # Platform theory
│
├── 📖 docs/                      # Documentation
│   ├── architecture.md          # System design
│   ├── bedrock_setup.md        # AWS configuration
│   ├── rust_patterns.md        # Rust best practices
│   └── deployment.md           # Production guide
│
├── 🧪 tests/                     # Test suite
│   ├── integration/             # End-to-end tests
│   └── unit/                    # Component tests
│
└── 🔧 scripts/                   # Utility scripts
    ├── setup.sh                 # Environment setup
    ├── test.sh                  # Run all tests
    └── deploy.sh                # Deployment helper
```

## 💡 Key Concepts Visualized

### The Message Pipeline
Think of messages flowing through a factory assembly line:

```
     Raw Input                  Enriched                   Routed
         │                         │                         │
         ▼                         ▼                         ▼
    ┌─────────┐             ┌─────────┐              ┌─────────┐
    │ Sanitize│ ──────────► │ Context │ ───────────► │  Model  │
    └─────────┘             └─────────┘              └─────────┘
                                                           │
                                                           ▼
    ┌─────────┐             ┌─────────┐              ┌─────────┐
    │ Display │ ◄────────── │ Format  │ ◄──────────  │   AI    │
    └─────────┘             └─────────┘              └─────────┘
     Final Output             Structured               Response
```

### The Plugin System
Like LEGO blocks with standardized connectors:

```rust
pub trait BotPlugin {
    async fn process(&self, input: Message) -> Result<Message>;
    fn capabilities(&self) -> Vec<Capability>;
}

// Any plugin can connect if it fits the trait
impl BotPlugin for WeatherPlugin { ... }
impl BotPlugin for DatabasePlugin { ... }
impl BotPlugin for CustomPlugin { ... }
```

## 🧪 Testing

```bash
# Run all tests
make test

# Run specific test suite
cargo test --package universal-bot-core

# Run property tests
cargo test --features property-testing

# Run integration tests
cargo test --test integration

# Generate coverage report
make coverage

# Run linting and formatting
make lint
cargo fmt --check
cargo clippy -- -D warnings
```

## 🚀 Quick Start Examples

### Interactive CLI (Perfect for Courses!)
Get started immediately with a Claude Code-like interface:

```bash
# Run the interactive CLI
cargo run --bin universal-bot-cli

# Or install globally
cargo install universal-bot-core
universal-bot-cli
```

Then ask anything:
```
🧠 You: What is Rust?
🤖 Claude: Rust is a systems programming language...

🧠 You: Write a Python function to sort a list
🤖 Claude: Here's a Python function to sort a list...

🧠 You: quit
👋 Goodbye!
```

### Available Examples

```bash
# Interactive CLI (perfect for courses)
cargo run --bin universal-bot-cli

# Framework examples
cargo run --example basic_bot
cargo run --example interactive_cli  
cargo run --example bedrock_integration
cargo run --example step_by_step_demo
```

### Programmatic Usage

```rust
use universal_bot_core::{Bot, BotConfig, Message};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = BotConfig::builder()
        .model("anthropic.claude-opus-4-1")
        .temperature(0.1)
        .build()?;
    
    let bot = Bot::new(config).await?;
    let response = bot.process(Message::text("Hello!")).await?;
    
    println!("🤖 {}", response.content);
    Ok(())
}
```

## 🎓 Course Philosophy

### Why 80/20?
- **80% Universal**: The AI logic, conversation management, and business logic remain constant
- **20% Specific**: Only the platform connection changes

### Why Rust?
- **Memory Safety**: No null pointer exceptions in production
- **Performance**: Near C++ speed with high-level abstractions
- **Concurrency**: True parallel processing with Tokio
- **Type Safety**: Catch errors at compile time, not runtime

### Why AWS Bedrock?
- **Multi-Model**: Access to Claude, Llama, and more
- **Enterprise Ready**: Built for production scale
- **Streaming**: Real-time token generation
- **Managed**: No model hosting headaches

## 🤝 Community & Support

### Get Help
- 🐛 **Issues**: [Report bugs](https://github.com/paiml/universal-bot/issues)
- 💡 **Discussions**: [Share ideas](https://github.com/paiml/universal-bot/discussions)
- 🌐 **Website**: [paiml.com](https://paiml.com)

### Contributing
We welcome contributions! Feel free to submit issues and pull requests.

## 📈 Progress Tracking

Track your learning journey:

```markdown
- [x] Module 1: Foundation
- [x] Module 2: Core Engine
- [ ] Module 3: Advanced AI
- [ ] Module 4: Plugins
- [ ] Module 5: Platform Theory
- [ ] Final Project: Custom Bot Brain
```

## 🏆 Certification Path

Complete all modules and build a custom bot brain to earn:
- **Certificate of Completion**
- **Portfolio Project** for GitHub
- **LinkedIn Badge** for your profile
- **Community Recognition** as a Universal Bot Architect

## 📊 Success Metrics

### What Success Looks Like
```
Week 1: "I can connect to AWS Bedrock from Rust"
Week 2: "I built a conversation engine"
Week 3: "My bot uses multiple AI models"
Week 4: "I created custom plugins"
Week 5: "I understand how to adapt this anywhere"
Final:  "I have a production-ready AI brain"
```

## 🔮 Beyond the Course

### Where This Leads
- **Senior Positions**: Architect-level thinking
- **Startup Ready**: Build AI products quickly
- **Open Source**: Contribute to major projects
- **Consulting**: Help others modernize their bots

### Next Steps After Completion
1. Build a production bot for a real use case
2. Create your own platform adapter
3. Contribute plugins to the community
4. Teach others the 80/20 approach

## 📝 License

This project is licensed under the MIT License - see [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **AWS Bedrock Team** for the powerful AI platform
- **Rust Community** for the amazing ecosystem
- **Tokio Project** for async runtime excellence
- **You** for choosing to think differently about bots

---

<div align="center">

**🧠 Stop Building Bots. Start Building Brains. 🧠**

*The future isn't platform-specific. It's universally intelligent.*

[Learn More](https://paiml.com) • [GitHub](https://github.com/paiml/universal-bot) • [Crates.io](https://crates.io/crates/universal-bot-core)

</div>