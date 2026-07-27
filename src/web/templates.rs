//! Askama template structs.
//!
//! Templates only ever receive plain `String`/`bool`/`Vec` fields: formatting
//! and optionality are resolved in Rust so the HTML stays trivial to read.

use askama::Template;
use askama_web::WebTemplate;
use serde::{Deserialize, Serialize};

use crate::{config::Config, entities::users};

pub const DEFAULT_APP_NAME: &str = "Rust Askama";

/// Context for the shared layout (brand, navigation state, form token).
///
/// `csrf_token` lives here because the layout renders the sign-out form.
#[derive(Debug, Clone)]
pub struct Nav {
    pub app_name: String,
    pub authenticated: bool,
    pub name: String,
    pub email: String,
    pub initials: String,
    pub csrf_token: String,
    /// Injects the development live-reload client script when true.
    pub live_reload: bool,
}

impl Default for Nav {
    fn default() -> Self {
        Self {
            app_name: DEFAULT_APP_NAME.to_owned(),
            authenticated: false,
            name: String::new(),
            email: String::new(),
            initials: String::new(),
            csrf_token: String::new(),
            live_reload: false,
        }
    }
}

impl Nav {
    /// Bare navigation, used where no config is available (error pages).
    pub fn guest() -> Self {
        Self::default()
    }

    /// Navigation for a visitor who is not signed in.
    pub fn guest_of(config: &Config) -> Self {
        Self {
            app_name: config.app_name.clone(),
            live_reload: config.dev.live_reload,
            ..Self::default()
        }
    }

    /// Navigation for a signed-in user.
    pub fn of(config: &Config, user: &users::Model) -> Self {
        Self {
            authenticated: true,
            name: user.name.clone(),
            email: user.email.clone(),
            initials: initials(&user.name),
            ..Self::guest_of(config)
        }
    }

    /// Signed in when a user is present, guest otherwise.
    pub fn maybe(config: &Config, user: Option<&users::Model>) -> Self {
        match user {
            Some(user) => Self::of(config, user),
            None => Self::guest_of(config),
        }
    }

    /// Attaches the CSRF token used by the layout and page forms.
    pub fn with_csrf(mut self, token: String) -> Self {
        self.csrf_token = token;
        self
    }
}

/// Up to two uppercase letters used by the avatar chip.
fn initials(name: &str) -> String {
    let letters: String = name
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .take(2)
        .collect();

    if letters.is_empty() {
        "?".to_owned()
    } else {
        letters.to_uppercase()
    }
}

/// One-shot message carried across a redirect in the session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flash {
    /// `success` | `error` | `info`
    pub kind: String,
    pub message: String,
}

impl Flash {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            kind: "success".to_owned(),
            message: message.into(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            kind: "error".to_owned(),
            message: message.into(),
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self {
            kind: "info".to_owned(),
            message: message.into(),
        }
    }
}

/// Pre-formatted user details shown on the dashboard.
#[derive(Debug, Clone)]
pub struct UserView {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub created_at: String,
    pub updated_at: String,
    pub member_since: String,
}

/// Pre-formatted session details shown on the dashboard.
#[derive(Debug, Clone)]
pub struct SessionView {
    pub id: String,
    pub short_id: String,
    pub signed_in_at: String,
    pub expires_at: String,
    pub expires_in: String,
    pub ip_address: String,
    pub user_agent: String,
    pub cookie_name: String,
    pub secure_cookie: bool,
    /// IANA zone the timestamps above are rendered in.
    pub timezone: String,
    /// `postgres` or `redis`.
    pub backend: String,
    /// Human readable location, e.g. `Postgres · sessions table`.
    pub backend_detail: String,
}

#[derive(Template, WebTemplate)]
#[template(path = "pages/home.html")]
pub struct HomePage {
    pub nav: Nav,
    pub flash: Vec<Flash>,
    pub user_count: u64,
}

#[derive(Template, WebTemplate)]
#[template(path = "pages/login.html")]
pub struct LoginPage {
    pub nav: Nav,
    pub flash: Vec<Flash>,
    pub email: String,
    pub errors: Vec<String>,
}

#[derive(Template, WebTemplate)]
#[template(path = "pages/register.html")]
pub struct RegisterPage {
    pub nav: Nav,
    pub flash: Vec<Flash>,
    pub name: String,
    pub email: String,
    pub errors: Vec<String>,
}

#[derive(Template, WebTemplate)]
#[template(path = "pages/dashboard.html")]
pub struct DashboardPage {
    pub nav: Nav,
    pub flash: Vec<Flash>,
    pub user: UserView,
    pub session: SessionView,
}

/// One row on the health page.
#[derive(Debug, Clone)]
pub struct Check {
    pub name: String,
    pub healthy: bool,
    pub detail: String,
    pub latency: String,
}

#[derive(Template, WebTemplate)]
#[template(path = "pages/health.html")]
pub struct HealthPage {
    pub nav: Nav,
    pub flash: Vec<Flash>,
    pub healthy: bool,
    pub status_label: String,
    pub checks: Vec<Check>,
    pub environment: String,
    pub version: String,
    pub uptime: String,
    pub server_time: String,
    pub timezone: String,
    pub session_backend: String,
    pub session_ttl: String,
    pub active_sessions: String,
    pub registered_users: String,
}

#[derive(Template, WebTemplate)]
#[template(path = "pages/error.html")]
pub struct ErrorPage {
    pub nav: Nav,
    pub flash: Vec<Flash>,
    pub status: u16,
    pub title: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::initials;

    #[test]
    fn builds_initials() {
        assert_eq!(initials("Ada Lovelace"), "AL");
        assert_eq!(initials("ada"), "A");
        assert_eq!(initials("  "), "?");
    }
}
