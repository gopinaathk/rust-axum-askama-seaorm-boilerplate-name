//! Flash messages: one-shot notices that survive a redirect.
//!
//! Stored in the session, read exactly once, then removed.

use tower_sessions::Session;

use crate::{error::AppResult, web::templates::Flash};

const KEY: &str = "flash";

/// Queues a message for the next rendered page.
pub async fn push(session: &Session, flash: Flash) -> AppResult<()> {
    let mut queued: Vec<Flash> = session.get(KEY).await?.unwrap_or_default();
    queued.push(flash);
    session.insert(KEY, queued).await?;

    Ok(())
}

/// Reads and clears the queued messages.
pub async fn take(session: &Session) -> AppResult<Vec<Flash>> {
    let queued: Vec<Flash> = session.remove(KEY).await?.unwrap_or_default();

    Ok(queued)
}
