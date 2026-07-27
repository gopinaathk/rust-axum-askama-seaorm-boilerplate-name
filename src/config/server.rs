//! HTTP server configuration.

use crate::config::env_vars::{ConfigError, parse, string};

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    /// Directory served under `/static`.
    pub static_dir: String,
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            host: string("HOST", "127.0.0.1"),
            port: parse("PORT", 3000)?,
            static_dir: string("STATIC_DIR", "static"),
        })
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
