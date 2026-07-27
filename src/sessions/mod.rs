//! Session storage.
//!
//! Two interchangeable backends, picked at boot with `SESSION_STORE`:
//! Postgres (durable, queryable) or Redis (fast, TTL based). [`AppSessionStore`]
//! wraps both so the rest of the app is written against a single type.

pub mod postgres;
pub mod redis;

use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use thiserror::Error;
use tower_sessions::{
    SessionStore,
    session::{Id, Record},
    session_store,
};

pub use postgres::PostgresSessionStore;
pub use redis::RedisSessionStore;

use crate::config::{Config, SessionBackend};

#[derive(Debug, Error)]
pub enum SessionStoreError {
    #[error("could not connect to redis at {endpoint}: {source}")]
    Redis {
        endpoint: String,
        #[source]
        source: ::redis::RedisError,
    },
}

/// The active session backend.
#[derive(Clone, Debug)]
pub enum AppSessionStore {
    Postgres(PostgresSessionStore),
    Redis(RedisSessionStore),
}

impl AppSessionStore {
    /// Builds the configured backend, verifying connectivity for Redis.
    pub async fn build(config: &Config, db: DatabaseConnection) -> Result<Self, SessionStoreError> {
        match config.session.backend {
            SessionBackend::Postgres => {
                tracing::info!("session store: postgres (`sessions` table)");
                Ok(Self::Postgres(PostgresSessionStore::new(db)))
            }
            SessionBackend::Redis => {
                let endpoint = config.redis.endpoint();
                let store =
                    RedisSessionStore::connect(&config.redis.url(), &config.redis.key_prefix)
                        .await
                        .map_err(|source| SessionStoreError::Redis {
                            endpoint: endpoint.clone(),
                            source,
                        })?;

                store
                    .ping()
                    .await
                    .map_err(|source| SessionStoreError::Redis {
                        endpoint: endpoint.clone(),
                        source,
                    })?;

                tracing::info!(endpoint = %endpoint, "session store: redis");
                Ok(Self::Redis(store))
            }
        }
    }

    pub fn backend(&self) -> SessionBackend {
        match self {
            Self::Postgres(_) => SessionBackend::Postgres,
            Self::Redis(_) => SessionBackend::Redis,
        }
    }

    /// Verifies the backend is reachable. Used by the health checks.
    pub async fn health_check(&self) -> Result<(), String> {
        match self {
            Self::Postgres(store) => store
                .count_active()
                .await
                .map(|_| ())
                .map_err(|error| error.to_string()),
            Self::Redis(store) => store.ping().await.map_err(|error| error.to_string()),
        }
    }

    /// Live session count, best effort.
    pub async fn active_sessions(&self) -> Option<u64> {
        match self {
            Self::Postgres(store) => store.count_active().await.ok(),
            Self::Redis(store) => store.count_active().await.ok(),
        }
    }

    /// Starts the expiry sweeper. Redis expires keys itself, so it is a no-op
    /// there.
    pub fn spawn_cleanup_task(
        &self,
        period: std::time::Duration,
    ) -> Option<tokio::task::JoinHandle<()>> {
        match self {
            Self::Postgres(store) => Some(store.clone().spawn_cleanup_task(period)),
            Self::Redis(_) => None,
        }
    }
}

#[async_trait]
impl SessionStore for AppSessionStore {
    async fn save(&self, record: &Record) -> session_store::Result<()> {
        match self {
            Self::Postgres(store) => store.save(record).await,
            Self::Redis(store) => store.save(record).await,
        }
    }

    async fn load(&self, session_id: &Id) -> session_store::Result<Option<Record>> {
        match self {
            Self::Postgres(store) => store.load(session_id).await,
            Self::Redis(store) => store.load(session_id).await,
        }
    }

    async fn delete(&self, session_id: &Id) -> session_store::Result<()> {
        match self {
            Self::Postgres(store) => store.delete(session_id).await,
            Self::Redis(store) => store.delete(session_id).await,
        }
    }
}
