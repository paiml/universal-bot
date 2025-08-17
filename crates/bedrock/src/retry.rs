//! Retry logic and policies for Bedrock operations

use std::time::Duration;

use backoff::{ExponentialBackoff, ExponentialBackoffBuilder};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::error::{BedrockError, ErrorCategory};

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Initial retry interval
    pub initial_interval: Duration,
    /// Maximum retry interval
    pub max_interval: Duration,
    /// Maximum total retry time
    pub max_elapsed_time: Duration,
    /// Backoff multiplier
    pub multiplier: f64,
    /// Maximum number of retries
    pub max_retries: usize,
    /// Jitter to add to retry intervals
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            initial_interval: Duration::from_millis(500),
            max_interval: Duration::from_secs(30),
            max_elapsed_time: Duration::from_secs(300),
            multiplier: 2.0,
            max_retries: 5,
            jitter: true,
        }
    }
}

impl RetryPolicy {
    /// Create a conservative retry policy
    pub fn conservative() -> Self {
        Self {
            initial_interval: Duration::from_secs(1),
            max_interval: Duration::from_secs(60),
            max_elapsed_time: Duration::from_secs(600),
            multiplier: 3.0,
            max_retries: 3,
            jitter: true,
        }
    }

    /// Create an aggressive retry policy
    pub fn aggressive() -> Self {
        Self {
            initial_interval: Duration::from_millis(100),
            max_interval: Duration::from_secs(10),
            max_elapsed_time: Duration::from_secs(120),
            multiplier: 1.5,
            max_retries: 10,
            jitter: true,
        }
    }

    /// Create a no-retry policy
    pub fn no_retry() -> Self {
        Self {
            initial_interval: Duration::from_millis(0),
            max_interval: Duration::from_millis(0),
            max_elapsed_time: Duration::from_millis(0),
            multiplier: 1.0,
            max_retries: 0,
            jitter: false,
        }
    }

    /// Convert to exponential backoff
    pub fn to_exponential_backoff(&self) -> ExponentialBackoff {
        ExponentialBackoffBuilder::new()
            .with_initial_interval(self.initial_interval)
            .with_max_interval(self.max_interval)
            .with_multiplier(self.multiplier)
            .with_max_elapsed_time(Some(self.max_elapsed_time))
            .with_randomization_factor(if self.jitter { 0.1 } else { 0.0 })
            .build()
    }
}

/// Retry strategy for different error types
#[derive(Debug, Clone)]
pub struct RetryStrategy {
    policies: std::collections::HashMap<ErrorCategory, RetryPolicy>,
    default_policy: RetryPolicy,
}

impl RetryStrategy {
    /// Create a new retry strategy
    pub fn new() -> Self {
        let mut policies = std::collections::HashMap::new();

        // Rate limiting gets more conservative retry
        policies.insert(ErrorCategory::RateLimit, RetryPolicy::conservative());

        // Network errors get aggressive retry
        policies.insert(ErrorCategory::Network, RetryPolicy::aggressive());

        // Server errors get default retry
        policies.insert(ErrorCategory::Server, RetryPolicy::default());

        // Resource exhaustion gets conservative retry
        policies.insert(ErrorCategory::Resource, RetryPolicy::conservative());

        // Client errors, auth errors, etc. don't get retried
        policies.insert(ErrorCategory::Client, RetryPolicy::no_retry());
        policies.insert(ErrorCategory::Authentication, RetryPolicy::no_retry());
        policies.insert(ErrorCategory::Authorization, RetryPolicy::no_retry());
        policies.insert(ErrorCategory::Configuration, RetryPolicy::no_retry());
        policies.insert(ErrorCategory::Content, RetryPolicy::no_retry());

        Self {
            policies,
            default_policy: RetryPolicy::default(),
        }
    }

    /// Get retry policy for an error
    pub fn policy_for_error(&self, error: &BedrockError) -> &RetryPolicy {
        let category = error.category();
        self.policies.get(&category).unwrap_or(&self.default_policy)
    }

    /// Check if an error should be retried
    pub fn should_retry(&self, error: &BedrockError, attempt: usize) -> bool {
        let policy = self.policy_for_error(error);

        // Don't retry if we've exceeded max retries
        if attempt >= policy.max_retries {
            return false;
        }

        // Check if the error is retryable
        error.is_retryable()
    }

    /// Calculate retry delay for an error and attempt number
    pub fn retry_delay(&self, error: &BedrockError, attempt: usize) -> Duration {
        let policy = self.policy_for_error(error);

        if attempt == 0 {
            return policy.initial_interval;
        }

        let mut delay = policy.initial_interval.as_millis() as f64;

        // Apply exponential backoff
        for _ in 0..attempt {
            delay *= policy.multiplier;
        }

        // Cap at max interval
        delay = delay.min(policy.max_interval.as_millis() as f64);

        // Add jitter if enabled
        if policy.jitter {
            let jitter_factor = fastrand::f64() * 0.1; // 10% jitter
            delay *= 1.0 + jitter_factor;
        }

        Duration::from_millis(delay as u64)
    }

    /// Set custom policy for an error category
    pub fn set_policy(&mut self, category: ErrorCategory, policy: RetryPolicy) {
        self.policies.insert(category, policy);
    }
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self::new()
    }
}

/// Retry executor for running operations with retry logic
pub struct RetryExecutor {
    strategy: RetryStrategy,
}

impl RetryExecutor {
    /// Create a new retry executor
    pub fn new(strategy: RetryStrategy) -> Self {
        Self { strategy }
    }

    /// Execute an operation with retry logic
    pub async fn execute<F, Fut, T>(&self, operation: F) -> Result<T, BedrockError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, BedrockError>>,
    {
        let mut attempt = 0;
        let start_time = std::time::Instant::now();

        loop {
            debug!("Executing operation (attempt {})", attempt + 1);

            match operation().await {
                Ok(result) => {
                    if attempt > 0 {
                        debug!("Operation succeeded after {} retries", attempt);
                    }
                    return Ok(result);
                }
                Err(error) => {
                    if !self.strategy.should_retry(&error, attempt) {
                        warn!("Operation failed after {} attempts: {}", attempt + 1, error);
                        return Err(error);
                    }

                    let delay = self.strategy.retry_delay(&error, attempt);

                    // Check if we've exceeded max elapsed time
                    let policy = self.strategy.policy_for_error(&error);
                    if start_time.elapsed() + delay > policy.max_elapsed_time {
                        warn!("Operation failed due to max elapsed time: {}", error);
                        return Err(error);
                    }

                    debug!(
                        "Operation failed (attempt {}), retrying in {:?}: {}",
                        attempt + 1,
                        delay,
                        error
                    );

                    tokio::time::sleep(delay).await;
                    attempt += 1;
                }
            }
        }
    }
}

/// Circuit breaker for preventing cascading failures
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    failure_threshold: usize,
    success_threshold: usize,
    timeout: Duration,
    state: CircuitBreakerState,
    failure_count: usize,
    success_count: usize,
    last_failure_time: Option<std::time::Instant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(failure_threshold: usize, success_threshold: usize, timeout: Duration) -> Self {
        Self {
            failure_threshold,
            success_threshold,
            timeout,
            state: CircuitBreakerState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
        }
    }

    /// Check if the circuit breaker allows the operation
    pub fn can_execute(&mut self) -> bool {
        match self.state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => {
                if let Some(last_failure) = self.last_failure_time {
                    if last_failure.elapsed() >= self.timeout {
                        self.state = CircuitBreakerState::HalfOpen;
                        self.success_count = 0;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitBreakerState::HalfOpen => true,
        }
    }

    /// Record a successful operation
    pub fn record_success(&mut self) {
        match self.state {
            CircuitBreakerState::Closed => {
                self.failure_count = 0;
            }
            CircuitBreakerState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.success_threshold {
                    self.state = CircuitBreakerState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                }
            }
            CircuitBreakerState::Open => {}
        }
    }

    /// Record a failed operation
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_time = Some(std::time::Instant::now());

        match self.state {
            CircuitBreakerState::Closed => {
                if self.failure_count >= self.failure_threshold {
                    self.state = CircuitBreakerState::Open;
                }
            }
            CircuitBreakerState::HalfOpen => {
                self.state = CircuitBreakerState::Open;
                self.success_count = 0;
            }
            CircuitBreakerState::Open => {}
        }
    }

    /// Get the current state
    pub fn state(&self) -> &str {
        match self.state {
            CircuitBreakerState::Closed => "closed",
            CircuitBreakerState::Open => "open",
            CircuitBreakerState::HalfOpen => "half-open",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy_creation() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_retries, 5);
        assert!(policy.jitter);

        let conservative = RetryPolicy::conservative();
        assert_eq!(conservative.max_retries, 3);

        let aggressive = RetryPolicy::aggressive();
        assert_eq!(aggressive.max_retries, 10);

        let no_retry = RetryPolicy::no_retry();
        assert_eq!(no_retry.max_retries, 0);
    }

    #[test]
    fn test_retry_strategy() {
        let strategy = RetryStrategy::new();

        let rate_limit_error = BedrockError::RateLimited("test".to_string());
        let policy = strategy.policy_for_error(&rate_limit_error);
        assert_eq!(policy.max_retries, 3); // Conservative policy

        let auth_error = BedrockError::Authentication("test".to_string());
        assert!(!strategy.should_retry(&auth_error, 0));
    }

    #[test]
    fn test_circuit_breaker() {
        let mut breaker = CircuitBreaker::new(3, 2, Duration::from_secs(10));

        // Initially closed
        assert!(breaker.can_execute());
        assert_eq!(breaker.state(), "closed");

        // Record failures to open the circuit
        breaker.record_failure();
        breaker.record_failure();
        breaker.record_failure();

        assert_eq!(breaker.state(), "open");
        assert!(!breaker.can_execute());
    }

    #[tokio::test]
    async fn test_retry_executor() {
        let strategy = RetryStrategy::new();
        let executor = RetryExecutor::new(strategy);

        let mut call_count = 0;
        let result = executor
            .execute(|| {
                call_count += 1;
                async move {
                    if call_count < 3 {
                        Err(BedrockError::ServiceError("temporary error".to_string()))
                    } else {
                        Ok("success")
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(call_count, 3);
    }
}
