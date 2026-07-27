//! Shared application state handed to every handler.

use std::{sync::Arc, time::Instant};

use axum::extract::FromRef;
use sea_orm::DatabaseConnection;

use crate::{
    config::Config, repositories::UserRepository, services::AuthService, sessions::AppSessionStore,
    web::dev::LiveReload,
};

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: Arc<Config>,
    /// Active session backend, also probed by the health page.
    pub sessions: AppSessionStore,
    /// Process start, used to report uptime.
    pub started_at: Instant,
    /// Present only when development live reload is enabled.
    pub live_reload: Option<LiveReload>,
}

impl AppState {
    pub fn new(
        db: DatabaseConnection,
        config: Arc<Config>,
        sessions: AppSessionStore,
        live_reload: Option<LiveReload>,
    ) -> Self {
        Self {
            db,
            config,
            sessions,
            started_at: Instant::now(),
            live_reload,
        }
    }

    /// Auth use cases wired to this state's connection.
    pub fn auth(&self) -> AuthService {
        AuthService::new(UserRepository::new(self.db.clone()))
    }

    pub fn users(&self) -> UserRepository {
        UserRepository::new(self.db.clone())
    }

    pub fn uptime_seconds(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }
}

impl FromRef<AppState> for DatabaseConnection {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl FromRef<AppState> for Arc<Config> {
    fn from_ref(state: &AppState) -> Self {
        Arc::clone(&state.config)
    }
}
