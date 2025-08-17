# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2025-01-17

### Added
- Initial release of Universal Bot Core framework
- AWS Bedrock integration with Claude Opus 4.1 support
- Message processing pipeline with stages (sanitize, enrich, route, process, format)
- Plugin system for extensibility
- Context management with conversation memory
- Comprehensive error handling with retry logic
- Token usage tracking and cost estimation
- Property testing suite with 85%+ coverage
- Example implementations for Bedrock integration
- YAML template generation following PDMT patterns
- Benchmarking suite for performance optimization

### Features
- 🚀 Production-ready AWS Bedrock client with connection pooling
- 🧠 Multi-model support (Claude Opus 4.1, Sonnet, Haiku)
- 📊 Real-time metrics and observability
- 🔌 Extensible plugin architecture
- 💾 Multiple context storage backends (Memory, Redis, PostgreSQL, SQLite)
- 🔄 Async/await throughout with Tokio runtime
- 🛡️ Type-safe builder patterns
- 📝 Comprehensive documentation and examples

### Security
- Input sanitization and validation
- Secure credential handling
- Rate limiting support
- Permission-based plugin system

### Performance
- Optimized for high throughput
- Connection pooling for AWS services
- Efficient context trimming
- Parallel pipeline processing

[1.0.0]: https://github.com/paiml/universal-bot/releases/tag/v1.0.0