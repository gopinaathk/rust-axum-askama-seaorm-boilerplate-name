//! Public landing page.

use axum::extract::State;
use tower_sessions::Session;

use crate::{
    error::AppResult,
    state::AppState,
    web::{
        csrf,
        extractors::MaybeUser,
        flash,
        templates::{HomePage, Nav},
    },
};

pub async fn show(
    State(state): State<AppState>,
    session: Session,
    MaybeUser(user): MaybeUser,
) -> AppResult<HomePage> {
    let nav =
        Nav::maybe(&state.config, user.as_ref()).with_csrf(csrf::token(&session).await?);

    Ok(HomePage {
        nav,
        flash: flash::take(&session).await?,
        user_count: state.users().count().await.unwrap_or_default(),
    })
}
