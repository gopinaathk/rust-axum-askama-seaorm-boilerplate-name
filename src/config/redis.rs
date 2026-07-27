//! Redis configuration (optional: only used when `SESSION_STORE=redis`).
//!
//! Like the database, the connection is described by parts and assembled into a
//! URL. `REDIS_URL` overrides everything when set.

use crate::config::env_vars::{ConfigError, optional, parse, string};

#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
    /// Redis 6+ ACL user. Empty for the default user.
    pub username: String,
    /// `requirepass` / ACL password. Empty when the server has no auth.
    pub password: String,
    pub database: u8,
    /// Prefix for every session key, e.g. `rust_askama:session:`.
    pub key_prefix: String,

    url_override: Option<String>,
}

impl RedisConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            host: string("REDIS_HOST", "127.0.0.1"),
            port: parse("REDIS_PORT", 6379)?,
            username: string("REDIS_USERNAME", ""),
            password: string("REDIS_PASSWORD", ""),
            database: parse("REDIS_DB", 0)?,
            key_prefix: string("REDIS_KEY_PREFIX", "rust_askama:session:"),
            url_override: optional("REDIS_URL"),
        })
    }

    /// Connection URL, credentials included.
    pub fn url(&self) -> String {
        if let Some(url) = &self.url_override {
            return url.clone();
        }

        let credentials = match (self.username.is_empty(), self.password.is_empty()) {
            (true, true) => String::new(),
            // Redis accepts `:password@` for the default user.
            (true, false) => format!(":{}@", encode(&self.password)),
            (false, true) => format!("{}@", encode(&self.username)),
            (false, false) => format!("{}:{}@", encode(&self.username), encode(&self.password)),
        };

        format!(
            "redis://{credentials}{}:{}/{}",
            self.host, self.port, self.database
        )
    }

    /// Host:port for log lines, never credentials.
    pub fn endpoint(&self) -> String {
        match &self.url_override {
            Some(url) => authority_of(url).unwrap_or_else(|| "unknown".to_owned()),
            None => format!("{}:{}", self.host, self.port),
        }
    }
}

fn encode(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '.' | '_' | '~' => c.to_string(),
            other => other
                .to_string()
                .as_bytes()
                .iter()
                .map(|byte| format!("%{byte:02X}"))
                .collect(),
        })
        .collect()
}

fn authority_of(url: &str) -> Option<String> {
    let (_, rest) = url.split_once("://")?;
    let authority = rest.split('/').next()?;
    let host = authority.rsplit('@').next()?;

    (!host.is_empty()).then(|| host.to_owned())
}

#[cfg(test)]
mod tests {
    use super::RedisConfig;

    fn config() -> RedisConfig {
        RedisConfig {
            host: "127.0.0.1".into(),
            port: 6379,
            username: String::new(),
            password: String::new(),
            database: 0,
            key_prefix: "rust_askama:session:".into(),
            url_override: None,
        }
    }

    #[test]
    fn builds_url_without_credentials() {
        assert_eq!(config().url(), "redis://127.0.0.1:6379/0");
    }

    #[test]
    fn builds_url_with_password_only() {
        let mut config = config();
        config.password = "se cret".into();

        assert_eq!(config.url(), "redis://:se%20cret@127.0.0.1:6379/0");
    }

    #[test]
    fn builds_url_with_username_and_password() {
        let mut config = config();
        config.username = "app".into();
        config.password = "pw".into();
        config.database = 3;

        assert_eq!(config.url(), "redis://app:pw@127.0.0.1:6379/3");
    }

    #[test]
    fn url_override_wins() {
        let mut config = config();
        config.url_override = Some("rediss://cache.internal:6380".into());

        assert_eq!(config.url(), "rediss://cache.internal:6380");
        assert_eq!(config.endpoint(), "cache.internal:6380");
    }
}
