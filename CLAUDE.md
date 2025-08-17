# CLAUDE.md - Universal Bot Development Context

## Project Overview

Universal Bot is an enterprise-grade AI automation framework combining AWS Bedrock, PDMT templating, and AssetGen content generation. This document provides context for AI assistants to maintain code quality and consistency.

## PMAT Quality Standards

### Code Quality Requirements

- **Test Coverage**: Minimum 85% line coverage, 75% branch coverage
- **Property Testing**: All critical paths must have property-based tests
- **Documentation**: All public APIs require comprehensive rustdoc comments
- **Complexity**: Maximum cyclomatic complexity of 10 per function
- **Error Handling**: Use `anyhow::Result` for fallible operations
- **Async Safety**: All async functions must be `Send + Sync`

### Rust Best Practices

```rust
// GOOD: Clear error handling with context
pub async fn process_request(input: &str) -> Result<Response> {
    let parsed = parse_input(input)
        .context("Failed to parse input")?;
    
    let validated = validate(&parsed)
        .context("Validation failed")?;
    
    let response = generate_response(validated).await
        .context("Failed to generate response")?;
    
    Ok(response)
}

// BAD: Unwrap and poor error messages
pub async fn process_request(input: &str) -> Response {
    let parsed = parse_input(input).unwrap();
    let validated = validate(&parsed).expect("validation error");
    generate_response(validated).await.unwrap()
}
```

### Testing Standards

```rust
// Unit tests with descriptive names
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_empty_input_returns_error() {
        let result = process_input("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty input"));
    }

    // Property tests for invariants
    proptest! {
        #[test]
        fn test_sanitized_output_never_contains_meta_phrases(
            input in any::<String>()
        ) {
            let sanitized = sanitize_content(&input);
            prop_assert!(!sanitized.contains("the speaker says"));
            prop_assert!(!sanitized.contains("the video shows"));
        }
    }
}
```

## Project Structure

```
universal-bot/
├── Cargo.toml          # Workspace configuration
├── Makefile            # Build automation
├── CLAUDE.md           # This file
├── README.md           # User documentation
├── LICENSE             # MIT License
├── docs/
│   └── 1.0-spec.md    # Technical specification
├── crates/
│   ├── core/          # Core bot functionality
│   ├── bedrock/       # AWS Bedrock integration
│   ├── pdmt/          # Template engine
│   ├── assetgen/      # Content generation
│   └── mcp/           # MCP protocol support
├── examples/          # Runnable examples
├── tests/            # Integration tests
└── benches/          # Performance benchmarks
```

## Key Components

### 1. AWS Bedrock Client

- Located in: `crates/bedrock/`
- Purpose: Manage AWS Bedrock connections with retry logic and pooling
- Key traits: `BedrockClient`, `ModelSelector`
- Testing: Mock AWS responses for unit tests, use LocalStack for integration

### 2. PDMT Template Engine

- Located in: `crates/pdmt/`
- Purpose: Zero-temperature deterministic template rendering
- Key traits: `TemplateEngine`, `TodoValidator`
- Testing: Property tests for determinism, golden tests for templates

### 3. AssetGen Generator

- Located in: `crates/assetgen/`
- Purpose: Generate educational and marketing content
- Key traits: `ContentGenerator`, `MetaAwareDetector`
- Testing: Validate all generated content, test meta-aware detection

### 4. MCP Server with Real-Time Quality Proxy

- Located in: `crates/mcp/`
- Purpose: Model Context Protocol server with PMAT quality enforcement
- Key traits: `MCPHandler`, `ToolRegistry`, `QualityProxy`
- Features:
  - Real-time quality gate enforcement
  - Live code quality scoring
  - Automatic SATD (Self-Admitted Technical Debt) detection
  - Property test generation integration
  - Coverage-driven development feedback
- Testing: Protocol compliance tests, message validation, quality proxy integration

### 5. PMAT Quality Proxy Integration

- Located in: `crates/quality/`
- Purpose: Real-time quality enforcement and feedback
- Key features:
  - **Live Quality Scoring**: Real-time analysis of code quality metrics
  - **SATD Detection**: Automatic detection of technical debt markers
  - **Coverage Enforcement**: Dynamic coverage threshold enforcement
  - **Property Test Integration**: Automatic property test generation
  - **Complexity Analysis**: Real-time cyclomatic complexity monitoring
  - **Documentation Coverage**: Rustdoc coverage tracking and enforcement

## Development Workflow

### Before Making Changes

1. Read relevant documentation in `docs/`
2. Check existing tests for patterns
3. Run `make lint` to understand code style
4. Review similar implementations in codebase

### Making Changes

1. Write tests first (TDD approach)
2. Implement minimal code to pass tests
3. Refactor for clarity and performance
4. Add comprehensive documentation
5. Ensure all quality gates pass

### Quality Gates (Automated)

```bash
make quality  # Runs all quality checks

# Individual checks:
make test          # Run all tests
make test-doc      # Run doctests
make test-property # Run property tests
make coverage      # Generate coverage report
make lint          # Run clippy and fmt
make bench         # Run benchmarks
make security      # Security audit
```

## MCP Quality Proxy Patterns

### Real-Time Quality Enforcement

```rust
use universal_bot_quality::{QualityProxy, QualityGate, SATDDetector};

pub struct MCPQualityProxy {
    gates: Vec<QualityGate>,
    satd_detector: SATDDetector,
    coverage_enforcer: CoverageEnforcer,
    property_generator: PropertyTestGenerator,
}

impl MCPQualityProxy {
    pub async fn enforce_quality(&self, code: &str) -> Result<QualityReport> {
        // Real-time SATD detection
        let satd_issues = self.satd_detector.scan(code).await?;
        
        // Live complexity analysis
        let complexity = self.analyze_complexity(code)?;
        
        // Coverage gap detection
        let coverage_gaps = self.coverage_enforcer.detect_gaps(code).await?;
        
        // Auto-generate property tests
        let property_tests = self.property_generator.generate(code).await?;
        
        Ok(QualityReport {
            satd_issues,
            complexity_score: complexity,
            coverage_gaps,
            suggested_property_tests: property_tests,
            overall_score: self.calculate_score(&satd_issues, complexity, &coverage_gaps),
        })
    }
}
```

### SATD Detection Integration

```rust
use regex::Regex;
use tree_sitter::{Language, Parser, Query};

pub struct SATDDetector {
    // Pattern-based detection
    patterns: Vec<Regex>,
    // AST-based detection
    parser: Parser,
    queries: Vec<Query>,
}

impl SATDDetector {
    pub fn new() -> Result<Self> {
        let patterns = vec![
            Regex::new(r"(?i)TODO\s*:?\s*(.+)")?,
            Regex::new(r"(?i)FIXME\s*:?\s*(.+)")?,
            Regex::new(r"(?i)HACK\s*:?\s*(.+)")?,
            Regex::new(r"(?i)BUG\s*:?\s*(.+)")?,
            Regex::new(r"(?i)XXX\s*:?\s*(.+)")?,
            Regex::new(r"(?i)TEMP\s*:?\s*(.+)")?,
            Regex::new(r"(?i)KLUDGE\s*:?\s*(.+)")?,
            // Detect code smells in comments
            Regex::new(r"(?i)this\s+is\s+(ugly|bad|wrong|broken)")?,
            Regex::new(r"(?i)(quick|dirty)\s+fix")?,
            Regex::new(r"(?i)technical\s+debt")?,
        ];
        
        Ok(Self { patterns, parser, queries })
    }
    
    pub async fn scan(&self, code: &str) -> Result<Vec<SATDIssue>> {
        let mut issues = Vec::new();
        
        // Pattern-based scanning
        for (line_num, line) in code.lines().enumerate() {
            for pattern in &self.patterns {
                if let Some(captures) = pattern.captures(line) {
                    issues.push(SATDIssue {
                        line: line_num + 1,
                        column: captures.get(0).unwrap().start(),
                        severity: self.determine_severity(&captures[0]),
                        description: captures.get(1).map_or("", |m| m.as_str()).to_string(),
                        suggestion: self.generate_suggestion(&captures[0]),
                    });
                }
            }
        }
        
        // AST-based scanning for complex patterns
        issues.extend(self.scan_ast(code).await?);
        
        Ok(issues)
    }
}
```

### Property Test Auto-Generation

```rust
use syn::{parse_str, Item, ItemFn, Type};
use quote::quote;

pub struct PropertyTestGenerator {
    type_strategies: HashMap<String, String>,
}

impl PropertyTestGenerator {
    pub async fn generate(&self, code: &str) -> Result<Vec<PropertyTest>> {
        let ast = parse_str::<syn::File>(code)?;
        let mut property_tests = Vec::new();
        
        for item in ast.items {
            if let Item::Fn(func) = item {
                if self.should_generate_property_test(&func) {
                    property_tests.extend(self.generate_for_function(&func)?);
                }
            }
        }
        
        Ok(property_tests)
    }
    
    fn generate_for_function(&self, func: &ItemFn) -> Result<Vec<PropertyTest>> {
        let func_name = &func.sig.ident;
        let mut tests = Vec::new();
        
        // Generate round-trip property tests
        if self.is_serializable_function(func) {
            tests.push(self.generate_roundtrip_test(func_name)?);
        }
        
        // Generate idempotency tests
        if self.is_idempotent_function(func) {
            tests.push(self.generate_idempotency_test(func_name)?);
        }
        
        // Generate invariant tests
        if let Some(invariants) = self.extract_invariants(func) {
            for invariant in invariants {
                tests.push(self.generate_invariant_test(func_name, &invariant)?);
            }
        }
        
        Ok(tests)
    }
    
    fn generate_roundtrip_test(&self, func_name: &syn::Ident) -> Result<PropertyTest> {
        let test_code = quote! {
            proptest! {
                #[test]
                fn test_roundtrip_#func_name(input in any::<ValidInput>()) {
                    let serialized = #func_name(&input);
                    let deserialized = reverse_#func_name(&serialized)?;
                    prop_assert_eq!(input, deserialized);
                }
            }
        };
        
        Ok(PropertyTest {
            name: format!("test_roundtrip_{}", func_name),
            code: test_code.to_string(),
            test_type: PropertyTestType::RoundTrip,
        })
    }
}
```

### Live Coverage Enforcement

```rust
use tarpaulin::Config as TarpaulinConfig;
use std::process::Command;

pub struct CoverageEnforcer {
    threshold: f32,
    config: TarpaulinConfig,
}

impl CoverageEnforcer {
    pub async fn detect_gaps(&self, code: &str) -> Result<Vec<CoverageGap>> {
        // Run coverage analysis on the code
        let coverage_report = self.run_coverage_analysis(code).await?;
        
        let mut gaps = Vec::new();
        
        // Identify uncovered lines
        for line in coverage_report.uncovered_lines {
            if self.is_critical_line(code, line.number) {
                gaps.push(CoverageGap {
                    line: line.number,
                    severity: CoverageSeverity::Critical,
                    reason: "Critical path not covered".to_string(),
                    suggested_test: self.suggest_test_for_line(code, line.number),
                });
            }
        }
        
        // Identify uncovered branches
        for branch in coverage_report.uncovered_branches {
            gaps.push(CoverageGap {
                line: branch.line,
                severity: CoverageSeverity::High,
                reason: format!("Branch {} not covered", branch.branch_id),
                suggested_test: self.suggest_test_for_branch(code, &branch),
            });
        }
        
        Ok(gaps)
    }
    
    pub async fn enforce_realtime(&self, file_path: &str) -> Result<EnforcementResult> {
        let current_coverage = self.get_current_coverage(file_path).await?;
        
        if current_coverage < self.threshold {
            return Ok(EnforcementResult::Failed {
                current: current_coverage,
                required: self.threshold,
                message: format!(
                    "Coverage {:.2}% below threshold {:.2}%",
                    current_coverage, self.threshold
                ),
                suggestions: self.generate_coverage_suggestions(file_path).await?,
            });
        }
        
        Ok(EnforcementResult::Passed {
            coverage: current_coverage,
        })
    }
}
```

### MCP Tool Integration

```rust
use mcp::{Tool, ToolInput, ToolResult};

#[derive(Debug)]
pub struct QualityGateTool {
    proxy: Arc<MCPQualityProxy>,
}

#[async_trait]
impl Tool for QualityGateTool {
    fn name(&self) -> &str {
        "quality_gate"
    }
    
    fn description(&self) -> &str {
        "Real-time quality gate enforcement with PMAT standards"
    }
    
    async fn execute(&self, input: ToolInput) -> Result<ToolResult> {
        let code = input.get_string("code")?;
        let enforce_strict = input.get_bool("strict").unwrap_or(true);
        
        // Run comprehensive quality analysis
        let quality_report = self.proxy.enforce_quality(&code).await?;
        
        // Apply enforcement rules
        let enforcement_result = if enforce_strict {
            self.apply_strict_enforcement(&quality_report)?
        } else {
            self.apply_advisory_enforcement(&quality_report)?
        };
        
        Ok(ToolResult::success(serde_json::json!({
            "quality_score": quality_report.overall_score,
            "satd_issues": quality_report.satd_issues,
            "coverage_gaps": quality_report.coverage_gaps,
            "property_tests": quality_report.suggested_property_tests,
            "enforcement_result": enforcement_result,
            "recommendations": self.generate_recommendations(&quality_report)
        })))
    }
}

#[derive(Debug)]
pub struct PropertyTestGeneratorTool {
    generator: Arc<PropertyTestGenerator>,
}

#[async_trait]
impl Tool for PropertyTestGeneratorTool {
    fn name(&self) -> &str {
        "generate_property_tests"
    }
    
    fn description(&self) -> &str {
        "Auto-generate property tests for Rust functions"
    }
    
    async fn execute(&self, input: ToolInput) -> Result<ToolResult> {
        let code = input.get_string("code")?;
        let test_types = input.get_string_array("test_types")
            .unwrap_or_else(|_| vec!["roundtrip".to_string(), "idempotency".to_string()]);
        
        let property_tests = self.generator.generate(&code).await?;
        
        // Filter by requested test types
        let filtered_tests: Vec<_> = property_tests
            .into_iter()
            .filter(|test| test_types.contains(&test.test_type.to_string()))
            .collect();
        
        Ok(ToolResult::success(serde_json::json!({
            "generated_tests": filtered_tests,
            "total_count": filtered_tests.len(),
            "estimated_coverage_improvement": self.estimate_coverage_improvement(&filtered_tests)
        })))
    }
}
```

## Common Patterns

### Error Handling

```rust
use anyhow::{Context, Result};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BotError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Model unavailable: {model}")]
    ModelUnavailable { model: String },
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
}

// Use anyhow for function results
pub async fn operation() -> Result<String> {
    do_something()
        .await
        .context("Failed to perform operation")?;
    Ok("success".to_string())
}
```

### Async Patterns

```rust
use tokio::sync::{RwLock, Semaphore};
use std::sync::Arc;

pub struct ConnectionPool {
    connections: Arc<RwLock<Vec<Connection>>>,
    semaphore: Arc<Semaphore>,
}

impl ConnectionPool {
    pub async fn get(&self) -> Result<Connection> {
        // Acquire permit first (for backpressure)
        let _permit = self.semaphore.acquire().await?;
        
        // Get connection with read lock, upgrade if needed
        let connections = self.connections.read().await;
        if let Some(conn) = connections.iter().find(|c| c.is_available()) {
            return Ok(conn.clone());
        }
        drop(connections);
        
        // Need write lock to create new connection
        let mut connections = self.connections.write().await;
        let conn = Connection::new().await?;
        connections.push(conn.clone());
        Ok(conn)
    }
}
```

### Builder Pattern

```rust
#[derive(Debug, Clone)]
pub struct BotConfig {
    model: String,
    temperature: f32,
    max_tokens: usize,
    timeout: Duration,
}

impl BotConfig {
    pub fn builder() -> BotConfigBuilder {
        BotConfigBuilder::default()
    }
}

#[derive(Default)]
pub struct BotConfigBuilder {
    model: Option<String>,
    temperature: Option<f32>,
    max_tokens: Option<usize>,
    timeout: Option<Duration>,
}

impl BotConfigBuilder {
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
    
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }
    
    pub fn build(self) -> Result<BotConfig> {
        Ok(BotConfig {
            model: self.model.context("model is required")?,
            temperature: self.temperature.unwrap_or(0.1),
            max_tokens: self.max_tokens.unwrap_or(2048),
            timeout: self.timeout.unwrap_or(Duration::from_secs(30)),
        })
    }
}
```

## Performance Considerations

### Optimization Guidelines

1. **Use `Arc` for shared immutable data**: Avoid unnecessary cloning
2. **Prefer `&str` over `String` in APIs**: Reduce allocations
3. **Use `SmallVec` for small collections**: Stack allocation for small arrays
4. **Implement `Drop` for resources**: Ensure cleanup
5. **Use `tokio::spawn` judiciously**: Don't spawn for trivial tasks

### Benchmarking

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_template_rendering(c: &mut Criterion) {
    let engine = TemplateEngine::new();
    let template = "Hello {{name}}!";
    let data = json!({"name": "World"});
    
    c.bench_function("render_template", |b| {
        b.iter(|| {
            engine.render(black_box(template), black_box(&data))
        })
    });
}

criterion_group!(benches, bench_template_rendering);
criterion_main!(benches);
```

## Security Guidelines

### Input Validation

```rust
use validator::{Validate, ValidationError};

#[derive(Debug, Validate)]
pub struct GenerateRequest {
    #[validate(length(min = 1, max = 10000))]
    pub input: String,
    
    #[validate(range(min = 0.0, max = 1.0))]
    pub temperature: f32,
    
    #[validate(custom = "validate_model")]
    pub model: String,
}

fn validate_model(model: &str) -> Result<(), ValidationError> {
    const ALLOWED_MODELS: &[&str] = &[
        "anthropic.claude-opus-4-1",
        "anthropic.claude-sonnet-4",
    ];
    
    if ALLOWED_MODELS.contains(&model) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_model"))
    }
}
```

### Secrets Management

```rust
use secrecy::{ExposeSecret, Secret};

pub struct Credentials {
    api_key: Secret<String>,
}

impl Credentials {
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("API_KEY")
            .context("API_KEY not set")?;
        
        Ok(Self {
            api_key: Secret::new(api_key),
        })
    }
    
    pub fn use_key(&self, f: impl FnOnce(&str)) {
        f(self.api_key.expose_secret())
    }
}
```

## Dependency Management

### Approved Dependencies

```toml
[dependencies]
# Async runtime
tokio = { version = "1.35", features = ["full"] }

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# AWS SDK
aws-sdk-bedrockruntime = "1.13"
aws-config = "1.1"

# Testing
proptest = "1.4"
criterion = "0.5"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Validation
validator = { version = "0.16", features = ["derive"] }
```

## CI/CD Pipeline

### GitHub Actions Workflow

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  quality:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      
      - name: Run quality checks
        run: |
          make lint
          make test
          make test-doc
          make test-property
          
      - name: Check coverage
        run: |
          make coverage
          # Fail if coverage < 85%
          
      - name: Security audit
        run: make security
```

## Troubleshooting Guide

### Common Issues

1. **AWS Credentials Error**
   - Check `AWS_PROFILE` and `AWS_REGION` environment variables
   - Ensure credentials have Bedrock permissions
   - Use `aws configure` to set up credentials

2. **Rate Limiting**
   - Implement exponential backoff
   - Use connection pooling
   - Consider caching responses

3. **Memory Issues**
   - Profile with `cargo-flamegraph`
   - Check for unbounded collections
   - Implement streaming for large responses

4. **Test Flakiness**
   - Use `tokio::test` for async tests
   - Mock external dependencies
   - Set deterministic seeds for property tests

## Review Checklist

Before submitting PR:

- [ ] All tests pass (`make test`)
- [ ] Coverage >= 85% (`make coverage`)
- [ ] No clippy warnings (`make lint`)
- [ ] Documentation updated
- [ ] Property tests added for critical paths
- [ ] Benchmarks show no regression
- [ ] Security audit passes
- [ ] CHANGELOG.md updated
- [ ] Examples still work

## Contact and Support

- **Documentation**: See `docs/` directory
- **Issues**: GitHub Issues
- **Discussions**: GitHub Discussions
- **Security**: Report to security@universal-bot.io

## Version

Last updated: 2025-01-17
Universal Bot version: 1.0.0
PMAT compliance: Full