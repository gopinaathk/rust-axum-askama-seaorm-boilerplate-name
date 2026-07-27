//! Browser live reload for development.
//!
//! The page opens an `EventSource` on `/dev/live-reload`. Two things trigger a
//! refresh:
//!
//! * **Asset change** – a file watcher on `static/` and `templates/` pushes a
//!   `reload` event, so CSS and JS edits show up without touching Rust.
//! * **Server restart** – `cargo watch` rebuilds and the stream drops. The
//!   browser reconnects on its own, sees a different `boot` id, and reloads.
//!
//! Disabled entirely when `APP_ENV=production` or `DEV_LIVE_RELOAD=false`.

use std::{convert::Infallible, path::Path, time::Duration};

use axum::{
    extract::State,
    http::StatusCode,
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
};
use notify::{RecursiveMode, Watcher, recommended_watcher};
use tokio::sync::{broadcast, mpsc};
use tokio_stream::{StreamExt, wrappers::BroadcastStream};
use uuid::Uuid;

use crate::state::AppState;

/// Coalesce window for editors that write a file several times in a row.
const DEBOUNCE: Duration = Duration::from_millis(200);

/// Handle shared by the watcher task and the SSE endpoint.
#[derive(Clone, Debug)]
pub struct LiveReload {
    /// Changes every time the process starts.
    boot_id: String,
    sender: broadcast::Sender<()>,
}

impl Default for LiveReload {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveReload {
    pub fn new() -> Self {
        Self {
            boot_id: Uuid::new_v4().simple().to_string(),
            sender: broadcast::channel(16).0,
        }
    }

    pub fn boot_id(&self) -> &str {
        &self.boot_id
    }

    /// Tells every connected browser to refresh.
    pub fn trigger(&self) {
        let _ = self.sender.send(());
    }

    /// Watches `paths` for changes.
    ///
    /// The returned watcher must be kept alive; dropping it stops the watch.
    pub fn watch<P: AsRef<Path>>(
        &self,
        paths: &[P],
    ) -> notify::Result<notify::RecommendedWatcher> {
        let (raw_sender, mut raw_receiver) = mpsc::unbounded_channel::<()>();

        let mut watcher = recommended_watcher(move |event: notify::Result<notify::Event>| {
            match event {
                // Access events fire on plain reads: ignore them.
                Ok(event) if !event.kind.is_access() => {
                    let _ = raw_sender.send(());
                }
                Ok(_) => {}
                Err(error) => tracing::debug!(%error, "file watcher error"),
            }
        })?;

        for path in paths {
            let path = path.as_ref();

            if !path.exists() {
                tracing::debug!(path = %path.display(), "skipping watch path, not found");
                continue;
            }

            watcher.watch(path, RecursiveMode::Recursive)?;
            tracing::debug!(path = %path.display(), "watching for changes");
        }

        // Debounce bursts into a single reload.
        let reload = self.clone();
        tokio::spawn(async move {
            while raw_receiver.recv().await.is_some() {
                tokio::time::sleep(DEBOUNCE).await;
                while raw_receiver.try_recv().is_ok() {}

                tracing::debug!("assets changed, asking browsers to reload");
                reload.trigger();
            }
        });

        Ok(watcher)
    }
}

/// `GET /dev/live-reload`: server-sent events stream.
pub async fn stream(State(state): State<AppState>) -> Response {
    let Some(reload) = state.live_reload.clone() else {
        return (StatusCode::NOT_FOUND, "live reload is disabled").into_response();
    };

    let boot = tokio_stream::once(Ok::<_, Infallible>(
        Event::default().event("boot").data(reload.boot_id().to_owned()),
    ));

    let changes = BroadcastStream::new(reload.sender.subscribe())
        .filter_map(|result| result.ok())
        .map(|()| Ok::<_, Infallible>(Event::default().event("reload").data("assets")));

    Sse::new(boot.chain(changes))
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response()
}
