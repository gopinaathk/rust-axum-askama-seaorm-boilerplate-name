//! Session keys and the helpers that read/write them.
//!
//! `tower-sessions` stores an opaque map; these helpers keep the key names and
//! value types in one place instead of scattering string literals in handlers.

use std::net::SocketAddr;

use axum::http::HeaderMap;
use chrono::{DateTime, Utc};
use tower_sessions::Session;

use crate::{
    config::{Config, SessionBackend},
    entities::users,
    error::AppResult,
    web::templates::SessionView,
};

pub const USER_ID: &str = "user_id";
pub const SIGNED_IN_AT: &str = "signed_in_at";
pub const IP_ADDRESS: &str = "ip_address";
pub const USER_AGENT: &str = "user_agent";

/// Client details captured at sign-in time, shown back on the dashboard.
#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub ip_address: String,
    pub user_agent: String,
}

impl ClientInfo {
    /// Resolves the client address and user agent.
    ///
    /// The socket address is the source of truth. Forwarding headers are only
    /// consulted when `TRUST_PROXY=true`, because any client can set them.
    pub fn resolve(headers: &HeaderMap, peer: Option<SocketAddr>, trust_proxy: bool) -> Self {
        let forwarded = if trust_proxy {
            forwarded_for(headers)
        } else {
            None
        };

        let ip_address = forwarded
            .or_else(|| peer.map(|address| address.ip().to_string()))
            .unwrap_or_else(|| "unknown".to_owned());

        let user_agent = headers
            .get(axum::http::header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .map(|value| truncate(value, 180))
            .unwrap_or_else(|| "unknown".to_owned());

        Self {
            ip_address,
            user_agent,
        }
    }
}

/// First hop of `X-Forwarded-For`, falling back to `X-Real-IP`.
fn forwarded_for(headers: &HeaderMap) -> Option<String> {
    let from_chain = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);

    from_chain.or_else(|| {
        headers
            .get("x-real-ip")
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    })
}

/// Signs a user in: rotates the session id, then records who they are.
///
/// Rotating first defends against session fixation, where an attacker plants a
/// known session id before the victim authenticates.
pub async fn start(session: &Session, user: &users::Model, client: &ClientInfo) -> AppResult<()> {
    session.cycle_id().await?;
    session.insert(USER_ID, user.id).await?;
    session.insert(SIGNED_IN_AT, Utc::now().timestamp()).await?;
    session
        .insert(IP_ADDRESS, client.ip_address.clone())
        .await?;
    session
        .insert(USER_AGENT, client.user_agent.clone())
        .await?;

    Ok(())
}

/// Signs a user out and deletes the session row.
pub async fn destroy(session: &Session) -> AppResult<()> {
    session.flush().await?;

    Ok(())
}

pub async fn user_id(session: &Session) -> AppResult<Option<i32>> {
    Ok(session.get::<i32>(USER_ID).await?)
}

/// Builds the "current session" card for the dashboard.
///
/// Timestamps are rendered in `APP_TIMEZONE`.
pub async fn view(session: &Session, config: &Config) -> AppResult<SessionView> {
    let id = session
        .id()
        .map(|id| id.to_string())
        .unwrap_or_else(|| "pending".to_owned());

    let signed_in_at = session
        .get::<i64>(SIGNED_IN_AT)
        .await?
        .and_then(|seconds| DateTime::<Utc>::from_timestamp(seconds, 0))
        .map(|value| config.format_datetime(value))
        .unwrap_or_else(|| "unknown".to_owned());

    let expiry = session.expiry_date();
    let ttl = session.expiry_age();

    Ok(SessionView {
        short_id: truncate(&id, 12),
        id,
        signed_in_at,
        expires_at: format_offset(config, expiry),
        expires_in: humanize(ttl.whole_seconds().max(0)),
        ip_address: session
            .get::<String>(IP_ADDRESS)
            .await?
            .unwrap_or_else(|| "unknown".to_owned()),
        user_agent: session
            .get::<String>(USER_AGENT)
            .await?
            .unwrap_or_else(|| "unknown".to_owned()),
        cookie_name: config.session.cookie_name.clone(),
        secure_cookie: config.session.cookie_secure,
        timezone: config.timezone_name().to_owned(),
        backend: config.session.backend.as_str().to_owned(),
        backend_detail: match config.session.backend {
            SessionBackend::Postgres => "Postgres · `sessions` table".to_owned(),
            SessionBackend::Redis => format!(
                "Redis · {} · keys `{}*`",
                config.redis.endpoint(),
                config.redis.key_prefix
            ),
        },
    })
}

/// Converts the session's `time::OffsetDateTime` expiry into display copy.
fn format_offset(config: &Config, value: time::OffsetDateTime) -> String {
    DateTime::<Utc>::from_timestamp(value.unix_timestamp(), 0)
        .map(|value| config.format_datetime(value))
        .unwrap_or_else(|| "unknown".to_owned())
}

/// `90 minutes` / `2 days` style copy for durations.
pub fn humanize(seconds: i64) -> String {
    const MINUTE: i64 = 60;
    const HOUR: i64 = 60 * MINUTE;
    const DAY: i64 = 24 * HOUR;

    let (value, unit) = match seconds {
        s if s < MINUTE => (s, "second"),
        s if s < HOUR => (s / MINUTE, "minute"),
        s if s < DAY => (s / HOUR, "hour"),
        s => (s / DAY, "day"),
    };

    if value == 1 {
        format!("1 {unit}")
    } else {
        format!("{value} {unit}s")
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }

    let head: String = value.chars().take(max_chars).collect();
    format!("{head}…")
}

#[cfg(test)]
mod tests {
    use super::{humanize, truncate};

    #[test]
    fn humanizes_durations() {
        assert_eq!(humanize(30), "30 seconds");
        assert_eq!(humanize(60), "1 minute");
        assert_eq!(humanize(5400), "1 hour");
        assert_eq!(humanize(172_800), "2 days");
    }

    #[test]
    fn truncates_with_ellipsis() {
        assert_eq!(truncate("abcdef", 10), "abcdef");
        assert_eq!(truncate("abcdef", 3), "abc…");
    }
}
