//! Developer experience settings (live reload).
//!
//! Everything here is a no-op in production: `live_reload` defaults to false
//! when `APP_ENV=production`.

use std::path::PathBuf;

use crate::config::{
    Environment,
    env_vars::{ConfigError, optional, parse, string},
};

#[derive(Debug, Clone)]
pub struct DevConfig {
    /// Serve the browser live-reload stream and inject its client script.
    pub live_reload: bool,
    /// Directories watched for asset changes.
    pub watch_paths: Vec<PathBuf>,
}

impl DevConfig {
    pub fn from_env(environment: Environment, static_dir: &str) -> Result<Self, ConfigError> {
        let default_enabled = !environment.is_production();

        let live_reload = match optional("DEV_LIVE_RELOAD") {
            Some(_) => parse("DEV_LIVE_RELOAD", default_enabled)?,
            None => default_enabled,
        };

        let default_paths = format!("{static_dir},templates");
        let watch_paths = string("DEV_WATCH_PATHS", &default_paths)
            .split(',')
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .collect();

        Ok(Self {
            live_reload,
            watch_paths,
        })
    }
}
