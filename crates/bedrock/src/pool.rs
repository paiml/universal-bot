//! Connection pool management for Bedrock clients

use std::sync::Arc;
use std::time::Duration;

use aws_sdk_bedrockruntime::Client as BedrockClient;
use parking_lot::RwLock;
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};

use crate::config::BedrockConfig;
use crate::error::{BedrockError, Result};

/// Connection pool for managing Bedrock clients
#[derive(Clone)]
pub struct ClientPool {
    inner: Arc<PoolInner>,
}

struct PoolInner {
    clients: Vec<BedrockClient>,
    semaphore: Arc<Semaphore>,
    config: BedrockConfig,
    stats: RwLock<PoolStats>,
}

/// Pool statistics
#[derive(Debug, Default, Clone)]
pub struct PoolStats {
    /// Total number of clients in the pool
    pub total_clients: usize,
    /// Number of clients currently in use
    pub active_clients: usize,
    /// Number of clients available for use
    pub available_clients: usize,
    /// Total number of client acquisitions
    pub total_acquisitions: u64,
    /// Total number of client releases
    pub total_releases: u64,
    /// Total time spent waiting for clients (ms)
    pub total_wait_time_ms: u64,
    /// Number of timeouts waiting for clients
    pub acquisition_timeouts: u64,
}

impl ClientPool {
    /// Create a new client pool
    pub async fn new(config: BedrockConfig) -> Result<Self> {
        info!("Creating client pool with {} connections", config.pool_size);

        let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(config.region.clone())
            .load()
            .await;

        let mut clients = Vec::with_capacity(config.pool_size);
        for i in 0..config.pool_size {
            debug!("Creating client {}/{}", i + 1, config.pool_size);

            let client_config = aws_sdk_bedrockruntime::Config::builder()
                .region(config.region.clone())
                .timeout_config(
                    aws_sdk_bedrockruntime::config::timeout::TimeoutConfig::builder()
                        .operation_timeout(Duration::from_secs(config.timeout_seconds))
                        .build(),
                )
                .build();

            let client = BedrockClient::from_conf(client_config);
            clients.push(client);
        }

        let stats = PoolStats {
            total_clients: config.pool_size,
            available_clients: config.pool_size,
            ..Default::default()
        };

        let inner = PoolInner {
            clients,
            semaphore: Arc::new(Semaphore::new(config.pool_size)),
            config,
            stats: RwLock::new(stats),
        };

        info!("Client pool created successfully");
        Ok(Self {
            inner: Arc::new(inner),
        })
    }

    /// Acquire a client from the pool
    pub async fn acquire(&self) -> Result<PooledClient> {
        let start = std::time::Instant::now();

        debug!("Acquiring client from pool");

        // Update stats
        {
            let mut stats = self.inner.stats.write();
            stats.total_acquisitions += 1;
        }

        // Acquire a permit from the semaphore
        let permit = self
            .inner
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| BedrockError::PoolExhausted(format!("Semaphore error: {}", e)))?;

        // Select a client (simple round-robin based on acquisition count)
        let client_index = {
            let stats = self.inner.stats.read();
            (stats.total_acquisitions - 1) as usize % self.inner.clients.len()
        };

        let client = &self.inner.clients[client_index];

        // Update stats
        {
            let mut stats = self.inner.stats.write();
            stats.active_clients += 1;
            stats.available_clients = stats.available_clients.saturating_sub(1);
            stats.total_wait_time_ms += start.elapsed().as_millis() as u64;
        }

        debug!("Client acquired from pool (index: {})", client_index);

        Ok(PooledClient {
            client: client.clone(),
            pool: self.clone(),
            _permit: permit,
        })
    }

    /// Try to acquire a client without waiting
    pub fn try_acquire(&self) -> Option<PooledClient> {
        if let Ok(permit) = self.inner.semaphore.clone().try_acquire_owned() {
            let client_index = {
                let stats = self.inner.stats.read();
                stats.total_acquisitions as usize % self.inner.clients.len()
            };

            let client = &self.inner.clients[client_index];

            // Update stats
            {
                let mut stats = self.inner.stats.write();
                stats.total_acquisitions += 1;
                stats.active_clients += 1;
                stats.available_clients = stats.available_clients.saturating_sub(1);
            }

            Some(PooledClient {
                client: client.clone(),
                pool: self.clone(),
                _permit: permit,
            })
        } else {
            None
        }
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        self.inner.stats.read().clone()
    }

    /// Get pool configuration
    pub fn config(&self) -> &BedrockConfig {
        &self.inner.config
    }

    /// Get the number of available clients
    pub fn available(&self) -> usize {
        self.inner.semaphore.available_permits()
    }

    /// Check if the pool is healthy
    pub fn is_healthy(&self) -> bool {
        self.available() > 0
    }

    /// Close the pool and release all resources
    pub async fn close(&self) {
        info!("Closing client pool");

        // Wait for all clients to be released
        while self.inner.semaphore.available_permits() < self.inner.config.pool_size {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        info!("Client pool closed");
    }

    fn release_client(&self) {
        let mut stats = self.inner.stats.write();
        stats.total_releases += 1;
        stats.active_clients = stats.active_clients.saturating_sub(1);
        stats.available_clients += 1;

        debug!("Client released to pool");
    }
}

/// A client borrowed from the pool
pub struct PooledClient {
    client: BedrockClient,
    pool: ClientPool,
    _permit: tokio::sync::OwnedSemaphorePermit,
}

impl PooledClient {
    /// Get the underlying Bedrock client
    pub fn client(&self) -> &BedrockClient {
        &self.client
    }

    /// Get a reference to the pool this client came from
    pub fn pool(&self) -> &ClientPool {
        &self.pool
    }
}

impl Drop for PooledClient {
    fn drop(&mut self) {
        self.pool.release_client();
    }
}

/// Pool health monitor
pub struct PoolHealthMonitor {
    pool: ClientPool,
    check_interval: Duration,
}

impl PoolHealthMonitor {
    /// Create a new health monitor
    pub fn new(pool: ClientPool, check_interval: Duration) -> Self {
        Self {
            pool,
            check_interval,
        }
    }

    /// Start monitoring the pool health
    pub async fn start_monitoring(&self) {
        let mut interval = tokio::time::interval(self.check_interval);

        loop {
            interval.tick().await;

            let stats = self.pool.stats();
            let availability_ratio = stats.available_clients as f64 / stats.total_clients as f64;

            if availability_ratio < 0.1 {
                warn!(
                    "Pool health warning: Low availability ({:.1}%), active: {}, available: {}",
                    availability_ratio * 100.0,
                    stats.active_clients,
                    stats.available_clients
                );
            }

            if stats.acquisition_timeouts > 0 {
                warn!(
                    "Pool health warning: {} acquisition timeouts",
                    stats.acquisition_timeouts
                );
            }

            debug!(
                "Pool health check: total={}, active={}, available={}, acquisitions={}",
                stats.total_clients,
                stats.active_clients,
                stats.available_clients,
                stats.total_acquisitions
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BedrockConfig;

    #[tokio::test]
    async fn test_pool_creation() {
        let config = BedrockConfig {
            pool_size: 2,
            ..Default::default()
        };

        // This would require actual AWS credentials to work
        // let pool = ClientPool::new(config).await.unwrap();
        // assert_eq!(pool.available(), 2);
        // assert!(pool.is_healthy());
    }

    #[test]
    fn test_pool_stats() {
        let stats = PoolStats {
            total_clients: 5,
            active_clients: 2,
            available_clients: 3,
            total_acquisitions: 10,
            total_releases: 8,
            total_wait_time_ms: 500,
            acquisition_timeouts: 1,
        };

        assert_eq!(stats.total_clients, 5);
        assert_eq!(stats.active_clients, 2);
        assert_eq!(stats.available_clients, 3);
    }
}
