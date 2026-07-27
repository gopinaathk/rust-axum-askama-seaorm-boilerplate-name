//! Central error type.
//!
//! Handlers return [`AppResult<T>`]; anything that fails is converted into an
//! HTML error page. Internal details are logged, never shown to the visitor.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

use crate::web::templates::{ErrorPage, Nav};

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    #[error("session error: {0}")]
    Session(#[from] tower_sessions::session::Error),

    #[error("template error: {0}")]
    Template(#[from] askama::Error),

    #[error("resource not found")]
    NotFound,

    #[error("the form security token was missing or expired")]
    InvalidCsrfToken,

    #[error("{0}")]
    BadRequest(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl AppError {
    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::InvalidCsrfToken => StatusCode::FORBIDDEN,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Database(_) | Self::Session(_) | Self::Template(_) | Self::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    /// Visitor facing copy. Internal failures stay vague on purpose.
    fn public_message(&self) -> String {
        match self {
            Self::NotFound => "We could not find the page you were looking for.".to_owned(),
            Self::InvalidCsrfToken => {
                "Your form session expired. Please reload the page and try again.".to_owned()
            }
            Self::BadRequest(message) => message.clone(),
            Self::Database(_) | Self::Session(_) | Self::Template(_) | Self::Internal(_) => {
                "Something went wrong on our side. Please try again in a moment.".to_owned()
            }
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();

        if status.is_server_error() {
            tracing::error!(error = %self, "request failed");
        } else {
            tracing::debug!(error = %self, "request rejected");
        }

        let page = ErrorPage {
            nav: Nav::guest(),
            flash: Vec::new(),
            status: status.as_u16(),
            title: status
                .canonical_reason()
                .unwrap_or("Unexpected error")
                .to_owned(),
            message: self.public_message(),
        };

        (status, page).into_response()
    }
}
