//! Metrics collection for Bedrock client

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Comprehensive metrics for the Bedrock client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedrockMetrics {
    /// Total number of requests made
    pub total_requests: u64,
    /// Number of successful requests
    pub successful_requests: u64,
    /// Number of failed requests
    pub failed_requests: u64,
    /// Number of currently active requests
    pub active_requests: u64,
    /// Total latency in milliseconds
    pub total_latency_ms: u64,
    /// Total input tokens processed
    pub total_input_tokens: u64,
    /// Total output tokens generated
    pub total_output_tokens: u64,
    /// Total estimated cost in USD
    pub total_cost: f64,
    /// Request counts by model
    pub requests_by_model: HashMap<String, u64>,
    /// Error counts by type
    pub errors_by_type: HashMap<String, u64>,
    /// Metrics collection start time
    pub start_time: DateTime<Utc>,
    /// Last updated time
    pub last_updated: DateTime<Utc>,
}

impl BedrockMetrics {
    /// Create new metrics instance
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            active_requests: 0,
            total_latency_ms: 0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cost: 0.0,
            requests_by_model: HashMap::new(),
            errors_by_type: HashMap::new(),
            start_time: now,
            last_updated: now,
        }
    }

    /// Calculate average latency in milliseconds
    pub fn average_latency_ms(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.total_latency_ms as f64 / self.total_requests as f64
        }
    }

    /// Calculate success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.successful_requests as f64 / self.total_requests as f64) * 100.0
        }
    }

    /// Calculate requests per second since start
    pub fn requests_per_second(&self) -> f64 {
        let duration = Utc::now().signed_duration_since(self.start_time);
        let seconds = duration.num_seconds() as f64;
        if seconds == 0.0 {
            0.0
        } else {
            self.total_requests as f64 / seconds
        }
    }

    /// Calculate total tokens processed
    pub fn total_tokens(&self) -> u64 {
        self.total_input_tokens + self.total_output_tokens
    }

    /// Calculate average tokens per request
    pub fn average_tokens_per_request(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.total_tokens() as f64 / self.total_requests as f64
        }
    }

    /// Calculate cost per token
    pub fn cost_per_token(&self) -> f64 {
        let total_tokens = self.total_tokens();
        if total_tokens == 0 {
            0.0
        } else {
            self.total_cost / total_tokens as f64
        }
    }

    /// Record a successful request
    pub fn record_success(
        &mut self,
        model: &str,
        latency_ms: u64,
        input_tokens: u64,
        output_tokens: u64,
        cost: f64,
    ) {
        self.total_requests += 1;
        self.successful_requests += 1;
        self.total_latency_ms += latency_ms;
        self.total_input_tokens += input_tokens;
        self.total_output_tokens += output_tokens;
        self.total_cost += cost;

        *self.requests_by_model.entry(model.to_string()).or_insert(0) += 1;
        self.last_updated = Utc::now();
    }

    /// Record a failed request
    pub fn record_failure(&mut self, model: &str, error_type: &str, latency_ms: u64) {
        self.total_requests += 1;
        self.failed_requests += 1;
        self.total_latency_ms += latency_ms;

        *self.requests_by_model.entry(model.to_string()).or_insert(0) += 1;
        *self
            .errors_by_type
            .entry(error_type.to_string())
            .or_insert(0) += 1;
        self.last_updated = Utc::now();
    }

    /// Get the most frequently used model
    pub fn most_used_model(&self) -> Option<(&String, &u64)> {
        self.requests_by_model
            .iter()
            .max_by_key(|&(_, count)| count)
    }

    /// Get the most common error type
    pub fn most_common_error(&self) -> Option<(&String, &u64)> {
        self.errors_by_type.iter().max_by_key(|&(_, count)| count)
    }

    /// Reset all metrics
    pub fn reset(&mut self) {
        let now = Utc::now();
        *self = Self::new();
        self.start_time = now;
        self.last_updated = now;
    }

    /// Get a summary of key metrics
    pub fn summary(&self) -> MetricsSummary {
        MetricsSummary {
            total_requests: self.total_requests,
            success_rate: self.success_rate(),
            average_latency_ms: self.average_latency_ms(),
            requests_per_second: self.requests_per_second(),
            total_tokens: self.total_tokens(),
            total_cost: self.total_cost,
            active_requests: self.active_requests,
            uptime_seconds: Utc::now()
                .signed_duration_since(self.start_time)
                .num_seconds() as u64,
        }
    }
}

impl Default for BedrockMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of key metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    /// Total requests processed
    pub total_requests: u64,
    /// Success rate percentage
    pub success_rate: f64,
    /// Average latency in milliseconds
    pub average_latency_ms: f64,
    /// Requests per second
    pub requests_per_second: f64,
    /// Total tokens processed
    pub total_tokens: u64,
    /// Total cost in USD
    pub total_cost: f64,
    /// Currently active requests
    pub active_requests: u64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
}

/// Health status for the client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Whether the client is healthy
    pub healthy: bool,
    /// Last health check latency
    pub latency_ms: u64,
    /// Error message if unhealthy
    pub error: Option<String>,
    /// Health check timestamp
    pub timestamp: DateTime<Utc>,
}

/// Thread-safe metrics collector using atomic operations
#[derive(Debug, Clone)]
pub struct AtomicMetrics {
    total_requests: Arc<AtomicU64>,
    successful_requests: Arc<AtomicU64>,
    failed_requests: Arc<AtomicU64>,
    active_requests: Arc<AtomicU64>,
    total_latency_ms: Arc<AtomicU64>,
    total_input_tokens: Arc<AtomicU64>,
    total_output_tokens: Arc<AtomicU64>,
    start_time: DateTime<Utc>,
}

impl AtomicMetrics {
    /// Create new atomic metrics
    pub fn new() -> Self {
        Self {
            total_requests: Arc::new(AtomicU64::new(0)),
            successful_requests: Arc::new(AtomicU64::new(0)),
            failed_requests: Arc::new(AtomicU64::new(0)),
            active_requests: Arc::new(AtomicU64::new(0)),
            total_latency_ms: Arc::new(AtomicU64::new(0)),
            total_input_tokens: Arc::new(AtomicU64::new(0)),
            total_output_tokens: Arc::new(AtomicU64::new(0)),
            start_time: Utc::now(),
        }
    }

    /// Increment request start
    pub fn start_request(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.active_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Record successful request completion
    pub fn complete_success(&self, latency_ms: u64, input_tokens: u64, output_tokens: u64) {
        self.successful_requests.fetch_add(1, Ordering::Relaxed);
        self.active_requests.fetch_sub(1, Ordering::Relaxed);
        self.total_latency_ms
            .fetch_add(latency_ms, Ordering::Relaxed);
        self.total_input_tokens
            .fetch_add(input_tokens, Ordering::Relaxed);
        self.total_output_tokens
            .fetch_add(output_tokens, Ordering::Relaxed);
    }

    /// Record failed request completion
    pub fn complete_failure(&self, latency_ms: u64) {
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
        self.active_requests.fetch_sub(1, Ordering::Relaxed);
        self.total_latency_ms
            .fetch_add(latency_ms, Ordering::Relaxed);
    }

    /// Get current snapshot of metrics
    pub fn snapshot(&self) -> MetricsSummary {
        let total_requests = self.total_requests.load(Ordering::Relaxed);
        let successful_requests = self.successful_requests.load(Ordering::Relaxed);
        let total_latency_ms = self.total_latency_ms.load(Ordering::Relaxed);
        let total_tokens = self.total_input_tokens.load(Ordering::Relaxed)
            + self.total_output_tokens.load(Ordering::Relaxed);

        let success_rate = if total_requests == 0 {
            0.0
        } else {
            (successful_requests as f64 / total_requests as f64) * 100.0
        };

        let average_latency_ms = if total_requests == 0 {
            0.0
        } else {
            total_latency_ms as f64 / total_requests as f64
        };

        let uptime_seconds = Utc::now()
            .signed_duration_since(self.start_time)
            .num_seconds() as u64;
        let requests_per_second = if uptime_seconds == 0 {
            0.0
        } else {
            total_requests as f64 / uptime_seconds as f64
        };

        MetricsSummary {
            total_requests,
            success_rate,
            average_latency_ms,
            requests_per_second,
            total_tokens,
            total_cost: 0.0, // Would need separate tracking for cost
            active_requests: self.active_requests.load(Ordering::Relaxed),
            uptime_seconds,
        }
    }
}

impl Default for AtomicMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = BedrockMetrics::new();
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.success_rate(), 0.0);
        assert_eq!(metrics.average_latency_ms(), 0.0);
    }

    #[test]
    fn test_metrics_recording() {
        let mut metrics = BedrockMetrics::new();

        metrics.record_success("test-model", 100, 50, 25, 0.01);
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.successful_requests, 1);
        assert_eq!(metrics.total_latency_ms, 100);
        assert_eq!(metrics.total_tokens(), 75);
        assert_eq!(metrics.success_rate(), 100.0);

        metrics.record_failure("test-model", "timeout", 200);
        assert_eq!(metrics.total_requests, 2);
        assert_eq!(metrics.failed_requests, 1);
        assert_eq!(metrics.success_rate(), 50.0);
    }

    #[test]
    fn test_atomic_metrics() {
        let metrics = AtomicMetrics::new();

        metrics.start_request();
        let snapshot1 = metrics.snapshot();
        assert_eq!(snapshot1.total_requests, 1);
        assert_eq!(snapshot1.active_requests, 1);

        metrics.complete_success(100, 50, 25);
        let snapshot2 = metrics.snapshot();
        assert_eq!(snapshot2.active_requests, 0);
        assert_eq!(snapshot2.success_rate, 100.0);
    }

    #[test]
    fn test_metrics_summary() {
        let mut metrics = BedrockMetrics::new();
        metrics.record_success("model1", 100, 50, 25, 0.01);
        metrics.record_success("model2", 150, 75, 30, 0.015);

        let summary = metrics.summary();
        assert_eq!(summary.total_requests, 2);
        assert_eq!(summary.success_rate, 100.0);
        assert_eq!(summary.total_tokens, 180);
        assert_eq!(summary.total_cost, 0.025);
    }

    #[test]
    fn test_most_used_model() {
        let mut metrics = BedrockMetrics::new();
        metrics.record_success("model1", 100, 50, 25, 0.01);
        metrics.record_success("model1", 100, 50, 25, 0.01);
        metrics.record_success("model2", 100, 50, 25, 0.01);

        let (model, count) = metrics.most_used_model().unwrap();
        assert_eq!(model, "model1");
        assert_eq!(*count, 2);
    }
}
