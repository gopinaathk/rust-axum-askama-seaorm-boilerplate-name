//! Health endpoints.
//!
//! * `GET /health`  – HTML status page for humans
//! * `GET /healthz` – plain text probe for containers and uptime checks
//! * `GET /healthz.json` – same checks as JSON

use std::time::Instant;

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sea_orm::ConnectionTrait;
use serde_json::json;
use tower_sessions::Session;

use crate::{
    error::AppResult,
    state::AppState,
    web::{
        csrf,
        extractors::MaybeUser,
        flash, session as session_helper,
        templates::{Check, HealthPage, Nav},
    },
};

/// One probe result: name, ok/failed, latency and a short detail line.
struct Probe {
    name: &'static str,
    healthy: bool,
    latency_ms: u128,
    detail: String,
}

async fn probe_database(state: &AppState) -> Probe {
    let started = Instant::now();

    match state.db.execute_unprepared("SELECT 1").await {
        Ok(_) => Probe {
            name: "Database",
            healthy: true,
            latency_ms: started.elapsed().as_millis(),
            detail: format!(
                "Postgres · {} · {}",
                state.config.database.endpoint(),
                state.config.database.database_name()
            ),
        },
        Err(error) => Probe {
            name: "Database",
            healthy: false,
            latency_ms: started.elapsed().as_millis(),
            detail: error.to_string(),
        },
    }
}

async fn probe_sessions(state: &AppState) -> Probe {
    let started = Instant::now();
    let backend = state.sessions.backend();

    match state.sessions.health_check().await {
        Ok(()) => Probe {
            name: "Session store",
            healthy: true,
            latency_ms: started.elapsed().as_millis(),
            detail: match backend.is_redis() {
                true => format!("Redis · {}", state.config.redis.endpoint()),
                false => "Postgres · `sessions` table".to_owned(),
            },
        },
        Err(error) => Probe {
            name: "Session store",
            healthy: false,
            latency_ms: started.elapsed().as_millis(),
            detail: error,
        },
    }
}

/// HTML status page.
pub async fn page(
    State(state): State<AppState>,
    session: Session,
    MaybeUser(user): MaybeUser,
) -> AppResult<HealthPage> {
    let database = probe_database(&state).await;
    let sessions = probe_sessions(&state).await;
    let healthy = database.healthy && sessions.healthy;

    let nav =
        Nav::maybe(&state.config, user.as_ref()).with_csrf(csrf::token(&session).await?);

    Ok(HealthPage {
        nav,
        flash: flash::take(&session).await?,
        healthy,
        status_label: if healthy {
            "All systems operational".to_owned()
        } else {
            "Degraded".to_owned()
        },
        checks: vec![
            Check {
                name: database.name.to_owned(),
                healthy: database.healthy,
                detail: database.detail,
                latency: format!("{} ms", database.latency_ms),
            },
            Check {
                name: sessions.name.to_owned(),
                healthy: sessions.healthy,
                detail: sessions.detail,
                latency: format!("{} ms", sessions.latency_ms),
            },
        ],
        environment: state.config.environment.as_str().to_owned(),
        version: state.config.version().to_owned(),
        uptime: session_helper::humanize(state.uptime_seconds() as i64),
        server_time: state.config.format_datetime(chrono::Utc::now()),
        timezone: state.config.timezone_name().to_owned(),
        session_backend: state.config.session.backend.as_str().to_owned(),
        session_ttl: session_helper::humanize(state.config.session.ttl_seconds() as i64),
        active_sessions: state
            .sessions
            .active_sessions()
            .await
            .map(|count| count.to_string())
            .unwrap_or_else(|| "unavailable".to_owned()),
        registered_users: state
            .users()
            .count()
            .await
            .map(|count| count.to_string())
            .unwrap_or_else(|_| "unavailable".to_owned()),
    })
}

/// Plain text probe: `ok` with 200, or the failing component with 503.
pub async fn probe(State(state): State<AppState>) -> Response {
    let database = probe_database(&state).await;
    let sessions = probe_sessions(&state).await;

    if database.healthy && sessions.healthy {
        (StatusCode::OK, "ok").into_response()
    } else {
        let failing = [&database, &sessions]
            .iter()
            .filter(|probe| !probe.healthy)
            .map(|probe| probe.name)
            .collect::<Vec<_>>()
            .join(", ");

        tracing::error!(failing = %failing, "health probe failed");

        (
            StatusCode::SERVICE_UNAVAILABLE,
            format!("unhealthy: {failing}"),
        )
            .into_response()
    }
}

/// JSON probe for dashboards and alerting.
pub async fn probe_json(State(state): State<AppState>) -> Response {
    let database = probe_database(&state).await;
    let sessions = probe_sessions(&state).await;
    let healthy = database.healthy && sessions.healthy;

    let body = json!({
        "status": if healthy { "ok" } else { "unhealthy" },
        "version": state.config.version(),
        "environment": state.config.environment.as_str(),
        "timezone": state.config.timezone_name(),
        "uptime_seconds": state.uptime_seconds(),
        "checks": {
            "database": {
                "healthy": database.healthy,
                "latency_ms": database.latency_ms,
                "detail": database.detail,
            },
            "session_store": {
                "healthy": sessions.healthy,
                "backend": state.config.session.backend.as_str(),
                "latency_ms": sessions.latency_ms,
                "detail": sessions.detail,
            },
        },
    });

    let status = if healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status, Json(body)).into_response()
}
