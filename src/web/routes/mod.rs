//! Route table. One module per area of the app.

pub mod auth;
pub mod dashboard;
pub mod health;
pub mod home;

use axum::{
    Router,
    routing::{get, post},
};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(home::show))
        .route("/register", get(auth::show_register).post(auth::register))
        .route("/login", get(auth::show_login).post(auth::login))
        .route("/sign-out", post(auth::sign_out))
        .route("/dashboard", get(dashboard::show))
        // Human readable status page plus machine probes.
        .route("/health", get(health::page))
        .route("/healthz", get(health::probe))
        .route("/healthz.json", get(health::probe_json))
        // Development only: returns 404 when live reload is disabled.
        .route("/dev/live-reload", get(crate::web::dev::stream))
}
