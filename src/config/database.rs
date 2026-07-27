//! Database configuration.
//!
//! The connection is described by separate parts (`DB_HOST`, `DB_PORT`,
//! `DB_USERNAME`, `DB_PASSWORD`, `DB_NAME`) which are assembled into a URL.
//! `DATABASE_URL` still wins when it is set, which is what managed platforms
//! (Railway, Fly, Heroku, ...) inject.

use std::time::Duration;

use crate::config::env_vars::{ConfigError, optional, parse, string};

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    /// Application database.
    pub name: String,
    /// Maintenance database used to issue `CREATE DATABASE`.
    pub admin_name: String,
    /// `sslmode` and friends, e.g. `sslmode=require`. Empty when unused.
    pub options: String,

    pub auto_create: bool,
    pub run_migrations: bool,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: Duration,

    /// Set when `DATABASE_URL` was provided; used verbatim.
    url_override: Option<String>,
}

impl DatabaseConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            host: string("DB_HOST", "localhost"),
            port: parse("DB_PORT", 5432)?,
            username: string("DB_USERNAME", "postgres"),
            password: string("DB_PASSWORD", ""),
            name: string("DB_NAME", "rust-axum-askama"),
            admin_name: string("DB_ADMIN_NAME", "postgres"),
            options: string("DB_OPTIONS", ""),

            auto_create: parse("DB_AUTO_CREATE", true)?,
            run_migrations: parse("DB_RUN_MIGRATIONS", true)?,
            max_connections: parse("DB_MAX_CONNECTIONS", 10)?,
            min_connections: parse("DB_MIN_CONNECTIONS", 1)?,
            connect_timeout: Duration::from_secs(parse("DB_CONNECT_TIMEOUT_SECS", 8)?),

            url_override: optional("DATABASE_URL"),
        })
    }

    /// Connection string for the application database.
    pub fn url(&self) -> String {
        match &self.url_override {
            Some(url) => url.clone(),
            None => self.build_url(&self.name),
        }
    }

    /// Connection string for the maintenance database.
    pub fn admin_url(&self) -> String {
        match &self.url_override {
            // Swap the trailing database name, keep credentials and options.
            Some(url) => swap_database(url, &self.admin_name),
            None => self.build_url(&self.admin_name),
        }
    }

    /// Name of the application database, whichever source it came from.
    pub fn database_name(&self) -> String {
        match &self.url_override {
            Some(url) => database_of(url).unwrap_or_else(|| self.name.clone()),
            None => self.name.clone(),
        }
    }

    /// Host:port pair for log lines (never includes credentials).
    pub fn endpoint(&self) -> String {
        match &self.url_override {
            Some(url) => authority_of(url).unwrap_or_else(|| "unknown".to_owned()),
            None => format!("{}:{}", self.host, self.port),
        }
    }

    fn build_url(&self, database: &str) -> String {
        let credentials = if self.password.is_empty() {
            encode(&self.username)
        } else {
            format!("{}:{}", encode(&self.username), encode(&self.password))
        };

        let query = if self.options.is_empty() {
            String::new()
        } else {
            format!("?{}", self.options.trim_start_matches('?'))
        };

        format!(
            "postgres://{credentials}@{}:{}/{}{query}",
            self.host,
            self.port,
            encode(database)
        )
    }
}

/// Percent-encodes the characters that would break a connection URL.
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

/// `postgres://u:p@h:5432/app?x=1` -> `Some("app")`
fn database_of(url: &str) -> Option<String> {
    let (_, rest) = url.split_once("://")?;
    let (_, path) = rest.split_once('/')?;
    let name = path.split(['?', '#']).next()?.trim();

    (!name.is_empty()).then(|| name.to_owned())
}

/// `postgres://u:p@h:5432/app` -> `Some("h:5432")`
fn authority_of(url: &str) -> Option<String> {
    let (_, rest) = url.split_once("://")?;
    let authority = rest.split('/').next()?;
    let host = authority.rsplit('@').next()?;

    (!host.is_empty()).then(|| host.to_owned())
}

/// Replaces the database segment of a URL, keeping everything else.
fn swap_database(url: &str, database: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_owned();
    };

    let Some((authority, path)) = rest.split_once('/') else {
        return format!("{url}/{}", encode(database));
    };

    let query = path
        .split_once('?')
        .map(|(_, query)| format!("?{query}"))
        .unwrap_or_default();

    format!("{scheme}://{authority}/{}{query}", encode(database))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> DatabaseConfig {
        DatabaseConfig {
            host: "localhost".into(),
            port: 5432,
            username: "postgres".into(),
            password: "p@ss word".into(),
            name: "rust-axum-askama".into(),
            admin_name: "postgres".into(),
            options: String::new(),
            auto_create: true,
            run_migrations: true,
            max_connections: 10,
            min_connections: 1,
            connect_timeout: Duration::from_secs(8),
            url_override: None,
        }
    }

    #[test]
    fn builds_url_from_parts_and_encodes_credentials() {
        assert_eq!(
            config().url(),
            "postgres://postgres:p%40ss%20word@localhost:5432/rust-axum-askama"
        );
    }

    #[test]
    fn admin_url_targets_the_maintenance_database() {
        assert_eq!(
            config().admin_url(),
            "postgres://postgres:p%40ss%20word@localhost:5432/postgres"
        );
    }

    #[test]
    fn database_url_override_wins() {
        let mut config = config();
        config.url_override =
            Some("postgres://user:pw@db.internal:6543/live?sslmode=require".into());

        assert_eq!(config.database_name(), "live");
        assert_eq!(config.endpoint(), "db.internal:6543");
        assert_eq!(
            config.admin_url(),
            "postgres://user:pw@db.internal:6543/postgres?sslmode=require"
        );
    }

    #[test]
    fn appends_options_when_present() {
        let mut config = config();
        config.options = "sslmode=require".into();

        assert!(config.url().ends_with("/rust-axum-askama?sslmode=require"));
    }
}
