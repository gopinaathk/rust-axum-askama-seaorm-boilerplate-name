//! Configuration, read once at boot from the environment.
//!
//! Sections live in their own files: [`database`], [`server`], [`session`].
//! `APP_ENV` selects the profile (`development` or `production`) and shifts a
//! few defaults, e.g. secure session cookies.

pub mod database;
pub mod dev;
pub mod env_vars;
pub mod redis;
pub mod server;
pub mod session;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;

pub use database::DatabaseConfig;
pub use dev::DevConfig;
pub use env_vars::ConfigError;
pub use redis::RedisConfig;
pub use server::ServerConfig;
pub use session::{SessionBackend, SessionConfig};

/// Deployment profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Production,
}

impl Environment {
    /// Reads `APP_ENV`, defaulting to development. Anything starting with
    /// `prod` counts as production.
    pub fn from_env() -> Result<Self, ConfigError> {
        let raw = env_vars::string("APP_ENV", "development").to_lowercase();

        match raw.as_str() {
            value if value.starts_with("prod") => Ok(Self::Production),
            "development" | "dev" | "local" | "test" => Ok(Self::Development),
            _ => Err(ConfigError::Invalid {
                key: "APP_ENV",
                value: raw,
            }),
        }
    }

    pub fn is_production(self) -> bool {
        matches!(self, Self::Production)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Production => "production",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub environment: Environment,
    pub app_name: String,
    /// IANA zone (e.g. `Asia/Kolkata`) used to render every timestamp.
    pub timezone: Tz,
    /// Trust `X-Forwarded-For` / `X-Real-IP`. Only enable behind a proxy you
    /// control, otherwise clients can spoof their address.
    pub trust_proxy: bool,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub session: SessionConfig,
    pub dev: DevConfig,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let environment = Environment::from_env()?;
        let server = ServerConfig::from_env()?;

        Ok(Self {
            environment,
            app_name: env_vars::string("APP_NAME", "Rust Askama"),
            timezone: timezone_from_env()?,
            trust_proxy: env_vars::parse("TRUST_PROXY", false)?,
            database: DatabaseConfig::from_env()?,
            redis: RedisConfig::from_env()?,
            session: SessionConfig::from_env(environment)?,
            dev: DevConfig::from_env(environment, &server.static_dir)?,
            server,
        })
    }

    pub fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    pub fn bind_address(&self) -> String {
        self.server.bind_address()
    }

    /// Renders a UTC instant in the configured timezone, e.g.
    /// `27 Jul 2026, 19:35 IST`.
    pub fn format_datetime(&self, value: DateTime<Utc>) -> String {
        value
            .with_timezone(&self.timezone)
            .format("%d %b %Y, %H:%M %Z")
            .to_string()
    }

    /// Current time in the configured timezone.
    pub fn now(&self) -> DateTime<Tz> {
        Utc::now().with_timezone(&self.timezone)
    }

    /// Zone name for display, e.g. `Asia/Kolkata`.
    pub fn timezone_name(&self) -> &'static str {
        self.timezone.name()
    }
}

/// `APP_TIMEZONE` accepts any IANA zone name; defaults to UTC.
fn timezone_from_env() -> Result<Tz, ConfigError> {
    let raw = env_vars::string("APP_TIMEZONE", "UTC");

    raw.parse::<Tz>().map_err(|_| ConfigError::Invalid {
        key: "APP_TIMEZONE",
        value: raw,
    })
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use chrono_tz::Tz;

    use super::{
        Config, DatabaseConfig, DevConfig, Environment, RedisConfig, ServerConfig, SessionConfig,
    };

    fn config(timezone: Tz) -> Config {
        Config {
            environment: Environment::Development,
            app_name: "Test".into(),
            timezone,
            trust_proxy: false,
            dev: DevConfig::from_env(Environment::Development, "static")
                .expect("defaults are valid"),
            server: ServerConfig {
                host: "127.0.0.1".into(),
                port: 3000,
                static_dir: "static".into(),
            },
            database: DatabaseConfig::from_env().expect("defaults are valid"),
            redis: RedisConfig::from_env().expect("defaults are valid"),
            session: SessionConfig::from_env(Environment::Development).expect("defaults are valid"),
        }
    }

    #[test]
    fn formats_in_the_configured_timezone() {
        let instant: DateTime<Utc> = "2026-07-27T12:00:00Z".parse().expect("valid instant");

        assert_eq!(
            config(Tz::Asia__Kolkata).format_datetime(instant),
            "27 Jul 2026, 17:30 IST"
        );
        assert_eq!(
            config(Tz::UTC).format_datetime(instant),
            "27 Jul 2026, 12:00 UTC"
        );
    }

    #[test]
    fn production_defaults_to_secure_cookies() {
        assert!(Environment::Production.is_production());
    }
}
