//! Use cases. Handlers call into this layer, never into repositories directly.

pub mod auth_service;
pub mod validation;

pub use auth_service::{AuthError, AuthService, LoginInput, RegisterInput};
