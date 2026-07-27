//! Registration and login use cases.

use sea_orm::DbErr;
use thiserror::Error;

use crate::{
    entities::users,
    repositories::UserRepository,
    security::password,
    services::validation::{self, Errors},
};

#[derive(Debug, Clone)]
pub struct RegisterInput {
    pub name: String,
    pub email: String,
    pub password: String,
    pub password_confirmation: String,
}

#[derive(Debug, Clone)]
pub struct LoginInput {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Error)]
pub enum AuthError {
    /// One or more inputs were rejected; messages are safe to show.
    #[error("validation failed")]
    Validation(Vec<String>),

    /// Wrong email or wrong password: never say which one.
    #[error("invalid credentials")]
    InvalidCredentials,

    #[error(transparent)]
    Database(#[from] DbErr),

    #[error("password hashing failed")]
    Password,
}

#[derive(Clone, Debug)]
pub struct AuthService {
    users: UserRepository,
}

impl AuthService {
    pub fn new(users: UserRepository) -> Self {
        Self { users }
    }

    /// Validates the input, hashes the password and stores the new user.
    pub async fn register(&self, input: RegisterInput) -> Result<users::Model, AuthError> {
        let mut errors = Errors::new();
        validation::validate_name(&input.name, &mut errors);
        validation::validate_email(&input.email, &mut errors);
        validation::validate_password(
            &input.password,
            Some(&input.password_confirmation),
            &mut errors,
        );

        if !errors.is_empty() {
            return Err(AuthError::Validation(errors.into_messages()));
        }

        if self.users.email_taken(&input.email).await? {
            return Err(AuthError::Validation(vec![
                "That email is already registered. Try signing in instead.".to_owned(),
            ]));
        }

        let password_hash = hash_off_runtime(input.password).await?;

        match self
            .users
            .create(&input.name, &input.email, password_hash)
            .await
        {
            Ok(user) => {
                tracing::info!(user_id = user.id, "user registered");
                Ok(user)
            }
            // Unique index catches the race between `email_taken` and `create`.
            Err(error) if is_unique_violation(&error) => Err(AuthError::Validation(vec![
                "That email is already registered. Try signing in instead.".to_owned(),
            ])),
            Err(error) => Err(AuthError::Database(error)),
        }
    }

    /// Returns the user when the credentials match.
    pub async fn login(&self, input: LoginInput) -> Result<users::Model, AuthError> {
        let mut errors = Errors::new();
        validation::validate_email(&input.email, &mut errors);

        if input.password.is_empty() {
            errors.push("Password is required.");
        }

        if !errors.is_empty() {
            return Err(AuthError::Validation(errors.into_messages()));
        }

        let Some(user) = self.users.find_by_email(&input.email).await? else {
            // Spend the same time as a real verification to avoid leaking
            // which addresses exist.
            let password = input.password.clone();
            let _ = tokio::task::spawn_blocking(move || password::verify_dummy(&password)).await;
            return Err(AuthError::InvalidCredentials);
        };

        let stored_hash = user.password_hash.clone();
        let password = input.password.clone();
        let matches =
            tokio::task::spawn_blocking(move || password::verify(&password, &stored_hash))
                .await
                .map_err(|error| {
                    tracing::error!(%error, "password verification task panicked");
                    AuthError::Password
                })?;

        if matches {
            tracing::info!(user_id = user.id, "user signed in");
            Ok(user)
        } else {
            Err(AuthError::InvalidCredentials)
        }
    }

    /// Looks a user up for the session guard.
    pub async fn find_by_id(&self, id: i32) -> Result<Option<users::Model>, AuthError> {
        Ok(self.users.find_by_id(id).await?)
    }
}

/// Argon2 is CPU bound, so it runs on the blocking pool.
async fn hash_off_runtime(password: String) -> Result<String, AuthError> {
    tokio::task::spawn_blocking(move || password::hash(&password))
        .await
        .map_err(|error| {
            tracing::error!(%error, "password hashing task panicked");
            AuthError::Password
        })?
        .map_err(|_| AuthError::Password)
}

fn is_unique_violation(error: &DbErr) -> bool {
    let message = error.to_string().to_lowercase();
    message.contains("duplicate key") || message.contains("unique constraint")
}
