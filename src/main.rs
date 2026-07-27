//! Binary entry point: configuration, database, sessions, HTTP server.

use std::sync::Arc;

use std::net::SocketAddr;

use rust_askama::{
    Config, db, sessions::AppSessionStore, state::AppState, web, web::dev::LiveReload,
};
use tokio::{net::TcpListener, signal};
use tower_sessions::{Expiry, SessionManagerLayer, cookie::SameSite};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // `.env` is optional: real deployments usually inject env vars directly.
    let dotenv = dotenvy::dotenv();
    init_tracing();

    match &dotenv {
        Ok(path) => tracing::debug!(path = %path.display(), "loaded .env"),
        // A parse error means later keys were skipped, which silently falls back
        // to defaults: worth a warning rather than a debug line.
        Err(error) if error.not_found() => {
            tracing::debug!("no .env file found, using the process environment");
        }
        Err(error) => tracing::warn!(%error, "could not read .env fully"),
    }

    let config = Arc::new(Config::from_env()?);
    let db = db::connect(&config.database).await?;

    // Postgres or Redis, selected by `SESSION_STORE`.
    let store = AppSessionStore::build(&config, db.clone()).await?;
    let cleanup = store.spawn_cleanup_task(config.session.cleanup_interval);

    let session_ttl = tower_sessions::cookie::time::Duration::seconds(
        config.session.ttl.as_secs().min(i64::MAX as u64) as i64,
    );

    let sessions = SessionManagerLayer::new(store.clone())
        .with_name(config.session.cookie_name.clone())
        // Cookies are only sent over HTTPS when this is true: keep it on in
        // production, off for plain-http local development.
        .with_secure(config.session.cookie_secure)
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_path("/".to_owned())
        // Sliding window: activity refreshes the deadline.
        .with_expiry(Expiry::OnInactivity(session_ttl));

    // Development live reload: watches assets, tells browsers to refresh.
    // `_watcher` must stay alive for the watch to keep running.
    let (live_reload, _watcher) = if config.dev.live_reload {
        let reload = LiveReload::new();

        match reload.watch(&config.dev.watch_paths) {
            Ok(watcher) => {
                tracing::info!("live reload enabled at /dev/live-reload");
                (Some(reload), Some(watcher))
            }
            Err(error) => {
                tracing::warn!(%error, "could not start the asset watcher");
                (Some(reload), None)
            }
        }
    } else {
        (None, None)
    };

    let state = AppState::new(db, Arc::clone(&config), store, live_reload);
    let app = web::router(state, sessions);

    let listener = TcpListener::bind(config.bind_address()).await?;
    tracing::info!(
        address = %listener.local_addr()?,
        app = %config.app_name,
        environment = config.environment.as_str(),
        timezone = config.timezone_name(),
        session_store = config.session.backend.as_str(),
        secure_cookies = config.session.cookie_secure,
        "server started"
    );

    if config.environment.is_production() && !config.session.cookie_secure {
        tracing::warn!(
            "running in production with SESSION_COOKIE_SECURE=false: session \
             cookies will be sent over plain HTTP"
        );
    }

    // `into_make_service_with_connect_info` exposes the peer address so the
    // dashboard can show where the session was created from.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    if let Some(cleanup) = cleanup {
        cleanup.abort();
    }
    tracing::info!("server stopped");

    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("rust_askama=debug,tower_http=info,sea_orm=warn,info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true).compact())
        .init();
}

/// Resolves on Ctrl+C (all platforms) or SIGTERM (unix), so in-flight requests
/// get a chance to finish.
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received");
}
