//! Register, sign in, sign out.
//!
//! Every POST verifies the CSRF token first. Failed submissions re-render the
//! form with `422` and keep the typed values (except passwords).

use std::net::SocketAddr;

use axum::{
    Form,
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use tower_sessions::Session;

use crate::{
    error::{AppError, AppResult},
    services::{AuthError, LoginInput, RegisterInput},
    state::AppState,
    web::{
        csrf,
        extractors::MaybeUser,
        flash,
        session::{self as session_helper, ClientInfo},
        templates::{Flash, LoginPage, Nav, RegisterPage},
    },
};

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub csrf_token: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterForm {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub password_confirmation: String,
    #[serde(default)]
    pub csrf_token: String,
}

#[derive(Debug, Deserialize)]
pub struct SignOutForm {
    #[serde(default)]
    pub csrf_token: String,
}

pub async fn show_login(
    State(state): State<AppState>,
    session: Session,
    user: MaybeUser,
) -> AppResult<Response> {
    if user.is_authenticated() {
        return Ok(Redirect::to("/dashboard").into_response());
    }

    let page = LoginPage {
        nav: Nav::guest_of(&state.config).with_csrf(csrf::token(&session).await?),
        flash: flash::take(&session).await?,
        email: String::new(),
        errors: Vec::new(),
    };

    Ok(page.into_response())
}

pub async fn login(
    State(state): State<AppState>,
    session: Session,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Form(form): Form<LoginForm>,
) -> AppResult<Response> {
    csrf::verify(&session, &form.csrf_token).await?;

    let input = LoginInput {
        email: form.email.clone(),
        password: form.password,
    };

    match state.auth().login(input).await {
        Ok(user) => {
            let client = ClientInfo::resolve(&headers, Some(peer), state.config.trust_proxy);
            session_helper::start(&session, &user, &client).await?;
            flash::push(
                &session,
                Flash::success(format!("Welcome back, {}.", user.name)),
            )
            .await?;

            Ok(Redirect::to("/dashboard").into_response())
        }
        Err(error) => {
            let errors = login_errors(error)?;

            let page = LoginPage {
                nav: Nav::guest_of(&state.config).with_csrf(csrf::token(&session).await?),
                flash: Vec::new(),
                email: form.email,
                errors,
            };

            Ok((StatusCode::UNPROCESSABLE_ENTITY, page).into_response())
        }
    }
}

pub async fn show_register(
    State(state): State<AppState>,
    session: Session,
    user: MaybeUser,
) -> AppResult<Response> {
    if user.is_authenticated() {
        return Ok(Redirect::to("/dashboard").into_response());
    }

    let page = RegisterPage {
        nav: Nav::guest_of(&state.config).with_csrf(csrf::token(&session).await?),
        flash: flash::take(&session).await?,
        name: String::new(),
        email: String::new(),
        errors: Vec::new(),
    };

    Ok(page.into_response())
}

pub async fn register(
    State(state): State<AppState>,
    session: Session,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Form(form): Form<RegisterForm>,
) -> AppResult<Response> {
    csrf::verify(&session, &form.csrf_token).await?;

    let input = RegisterInput {
        name: form.name.clone(),
        email: form.email.clone(),
        password: form.password,
        password_confirmation: form.password_confirmation,
    };

    match state.auth().register(input).await {
        Ok(user) => {
            let client = ClientInfo::resolve(&headers, Some(peer), state.config.trust_proxy);
            session_helper::start(&session, &user, &client).await?;
            flash::push(
                &session,
                Flash::success(format!("Your account is ready, {}.", user.name)),
            )
            .await?;

            Ok(Redirect::to("/dashboard").into_response())
        }
        Err(error) => {
            let errors = register_errors(error)?;

            let page = RegisterPage {
                nav: Nav::guest_of(&state.config).with_csrf(csrf::token(&session).await?),
                flash: Vec::new(),
                name: form.name,
                email: form.email,
                errors,
            };

            Ok((StatusCode::UNPROCESSABLE_ENTITY, page).into_response())
        }
    }
}

pub async fn sign_out(session: Session, Form(form): Form<SignOutForm>) -> AppResult<Redirect> {
    csrf::verify(&session, &form.csrf_token).await?;

    // Deletes the session row and the cookie's session id.
    session_helper::destroy(&session).await?;

    // Runs against the fresh session created for the redirect.
    flash::push(&session, Flash::success("You have been signed out.")).await?;

    Ok(Redirect::to("/login"))
}

/// Turns an auth failure into form messages, or bubbles up real errors.
fn login_errors(error: AuthError) -> AppResult<Vec<String>> {
    match error {
        AuthError::Validation(messages) => Ok(messages),
        AuthError::InvalidCredentials => Ok(vec![
            "Those credentials do not match our records.".to_owned(),
        ]),
        AuthError::Database(error) => Err(AppError::Database(error)),
        AuthError::Password => Err(AppError::Internal("password hashing failed".to_owned())),
    }
}

fn register_errors(error: AuthError) -> AppResult<Vec<String>> {
    match error {
        AuthError::Validation(messages) => Ok(messages),
        AuthError::InvalidCredentials => Ok(vec!["Registration failed.".to_owned()]),
        AuthError::Database(error) => Err(AppError::Database(error)),
        AuthError::Password => Err(AppError::Internal("password hashing failed".to_owned())),
    }
}
