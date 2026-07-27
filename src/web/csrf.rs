//! Synchroniser-token CSRF protection for the HTML forms.
//!
//! A random token is minted per session, embedded in every form as a hidden
//! field, and compared on submit. Combined with the `SameSite=Lax` cookie this
//! stops a third-party page from posting on the visitor's behalf.

use tower_sessions::Session;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

const KEY: &str = "csrf_token";

/// Returns the session token, minting one on first use.
pub async fn token(session: &Session) -> AppResult<String> {
    if let Some(existing) = session.get::<String>(KEY).await? {
        return Ok(existing);
    }

    // 256 bits of randomness from two v4 UUIDs.
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());

    session.insert(KEY, token.clone()).await?;

    Ok(token)
}

/// Rejects the request unless the submitted token matches the session token.
pub async fn verify(session: &Session, submitted: &str) -> AppResult<()> {
    let expected = session
        .get::<String>(KEY)
        .await?
        .ok_or(AppError::InvalidCsrfToken)?;

    if constant_time_eq(expected.as_bytes(), submitted.as_bytes()) {
        Ok(())
    } else {
        Err(AppError::InvalidCsrfToken)
    }
}

/// Comparison whose duration does not depend on where the bytes differ.
fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    left.iter()
        .zip(right)
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

#[cfg(test)]
mod tests {
    use super::constant_time_eq;

    #[test]
    fn compares_bytes() {
        assert!(constant_time_eq(b"token", b"token"));
        assert!(!constant_time_eq(b"token", b"tokeN"));
        assert!(!constant_time_eq(b"token", b"token-longer"));
    }
}
