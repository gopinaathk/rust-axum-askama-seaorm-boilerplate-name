//! Environment variable readers shared by the config sections.

use std::env;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("environment variable `{0}` is required")]
    Missing(&'static str),
    #[error("environment variable `{key}` has an invalid value: `{value}`")]
    Invalid { key: &'static str, value: String },
}

/// Required value; empty strings count as missing.
pub fn required(key: &'static str) -> Result<String, ConfigError> {
    match env::var(key) {
        Ok(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(ConfigError::Missing(key)),
    }
}

/// Optional value with a fallback.
pub fn string(key: &str, default: &str) -> String {
    optional(key).unwrap_or_else(|| default.to_owned())
}

/// Optional value, `None` when unset or blank.
pub fn optional(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

/// Parses a value of any `FromStr` type, falling back to `default`.
pub fn parse<T: std::str::FromStr>(key: &'static str, default: T) -> Result<T, ConfigError> {
    match optional(key) {
        Some(value) => value
            .parse()
            .map_err(|_| ConfigError::Invalid { key, value }),
        None => Ok(default),
    }
}
