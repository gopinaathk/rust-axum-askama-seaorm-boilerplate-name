//! Session cookie, lifetime and backend configuration.

use std::{fmt, time::Duration};

use crate::config::{
    Environment,
    env_vars::{ConfigError, optional, parse, string},
};

/// Where session records are kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionBackend {
    /// `sessions` table in Postgres, via SeaORM.
    Postgres,
    /// Redis keys with a native TTL.
    Redis,
}

impl SessionBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Postgres => "postgres",
            Self::Redis => "redis",
        }
    }

    pub fn is_redis(self) -> bool {
        matches!(self, Self::Redis)
    }
}

impl fmt::Display for SessionBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub backend: SessionBackend,
    pub cookie_name: String,
    /// `Secure` attribute: cookies are then only sent over HTTPS.
    pub cookie_secure: bool,
    /// Sliding inactivity window.
    pub ttl: Duration,
    /// How often expired Postgres rows are purged (Redis expires keys itself).
    pub cleanup_interval: Duration,
}

impl SessionConfig {
    pub fn from_env(environment: Environment) -> Result<Self, ConfigError> {
        // Secure cookies are the default in production, opt-in locally where
        // there is usually no TLS.
        let cookie_secure = match optional("SESSION_COOKIE_SECURE") {
            Some(_) => parse("SESSION_COOKIE_SECURE", environment.is_production())?,
            None => environment.is_production(),
        };

        Ok(Self {
            backend: backend_from_env()?,
            cookie_name: string("SESSION_COOKIE_NAME", "rust_askama_sid"),
            cookie_secure,
            ttl: Duration::from_secs(parse::<u64>("SESSION_TTL_MINUTES", 1440)? * 60),
            cleanup_interval: Duration::from_secs(parse("SESSION_CLEANUP_INTERVAL_SECS", 600)?),
        })
    }

    pub fn ttl_seconds(&self) -> u64 {
        self.ttl.as_secs()
    }
}

/// `SESSION_STORE=postgres|redis`, defaults to postgres.
fn backend_from_env() -> Result<SessionBackend, ConfigError> {
    let raw = string("SESSION_STORE", "postgres").to_lowercase();

    match raw.as_str() {
        "postgres" | "postgresql" | "pg" | "database" | "db" => Ok(SessionBackend::Postgres),
        "redis" | "valkey" => Ok(SessionBackend::Redis),
        _ => Err(ConfigError::Invalid {
            key: "SESSION_STORE",
            value: raw,
        }),
    }
}
