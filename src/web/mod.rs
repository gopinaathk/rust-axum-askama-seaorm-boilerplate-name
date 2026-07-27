//! HTTP layer: router assembly, guards, templates, session helpers.

pub mod csrf;
pub mod dev;
pub mod extractors;
pub mod flash;
pub mod routes;
pub mod session;
pub mod templates;

use axum::{
    Router,
    http::{HeaderName, HeaderValue},
};
use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer, trace::TraceLayer};
use tower_sessions::SessionManagerLayer;

use crate::{error::AppError, sessions::AppSessionStore, state::AppState};

/// Builds the application router: routes, static files, session and tracing.
pub fn router(state: AppState, sessions: SessionManagerLayer<AppSessionStore>) -> Router {
    let static_dir = state.config.server.static_dir.clone();

    routes::router()
        .fallback(not_found)
        .nest_service("/static", ServeDir::new(static_dir))
        .layer(sessions)
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Unknown paths render the styled 404 page.
async fn not_found() -> AppError {
    AppError::NotFound
}
