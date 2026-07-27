//! Request guards.
//!
//! [`CurrentUser`] protects private pages, [`MaybeUser`] lets public pages
//! adapt their navigation without forcing a sign-in.

use axum::{
    extract::FromRequestParts,
    http::request::Parts,
    response::{IntoResponse, Redirect, Response},
};
use tower_sessions::Session;

use crate::{
    entities::users,
    error::AppResult,
    state::AppState,
    web::{flash, session as session_helper, templates::Flash},
};

/// Signed-in user; unauthenticated requests are redirected to `/login`.
#[derive(Debug, Clone)]
pub struct CurrentUser(pub users::Model);

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let session = extract_session(parts, state).await?;

        match resolve_user(&session, state).await {
            Ok(Some(user)) => Ok(Self(user)),
            Ok(None) => {
                let _ = flash::push(&session, Flash::info("Please sign in to continue.")).await;
                Err(Redirect::to("/login").into_response())
            }
            Err(error) => Err(error.into_response()),
        }
    }
}

/// Optional user, for pages that render for guests and members alike.
#[derive(Debug, Clone)]
pub struct MaybeUser(pub Option<users::Model>);

impl MaybeUser {
    pub fn as_ref(&self) -> Option<&users::Model> {
        self.0.as_ref()
    }

    pub fn is_authenticated(&self) -> bool {
        self.0.is_some()
    }
}

impl FromRequestParts<AppState> for MaybeUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let session = extract_session(parts, state).await?;

        resolve_user(&session, state)
            .await
            .map(Self)
            .map_err(IntoResponse::into_response)
    }
}

async fn extract_session(parts: &mut Parts, state: &AppState) -> Result<Session, Response> {
    Session::from_request_parts(parts, state)
        .await
        .map_err(IntoResponse::into_response)
}

/// Loads the session's user. A session pointing at a deleted user is dropped.
async fn resolve_user(session: &Session, state: &AppState) -> AppResult<Option<users::Model>> {
    let Some(user_id) = session_helper::user_id(session).await? else {
        return Ok(None);
    };

    match state.users().find_by_id(user_id).await? {
        Some(user) => Ok(Some(user)),
        None => {
            tracing::warn!(user_id, "session referenced a missing user, clearing it");
            session_helper::destroy(session).await?;
            Ok(None)
        }
    }
}
