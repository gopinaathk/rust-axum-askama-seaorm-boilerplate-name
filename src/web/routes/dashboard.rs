//! Private area: shows the signed-in user and their live session.

use axum::extract::State;
use chrono::Utc;
use tower_sessions::Session;

use crate::{
    config::Config,
    entities::users,
    error::AppResult,
    state::AppState,
    web::{
        csrf,
        extractors::CurrentUser,
        flash, session as session_helper,
        templates::{DashboardPage, Nav, UserView},
    },
};

pub async fn show(
    State(state): State<AppState>,
    session: Session,
    CurrentUser(user): CurrentUser,
) -> AppResult<DashboardPage> {
    let nav = Nav::of(&state.config, &user).with_csrf(csrf::token(&session).await?);

    Ok(DashboardPage {
        nav,
        flash: flash::take(&session).await?,
        session: session_helper::view(&session, &state.config).await?,
        user: user_view(&user, &state.config),
    })
}

fn user_view(user: &users::Model, config: &Config) -> UserView {
    let age_seconds = (Utc::now() - user.created_at.to_utc()).num_seconds().max(0);

    UserView {
        id: user.id,
        name: user.name.clone(),
        email: user.email.clone(),
        created_at: config.format_datetime(user.created_at.to_utc()),
        updated_at: config.format_datetime(user.updated_at.to_utc()),
        member_since: session_helper::humanize(age_seconds),
    }
}
