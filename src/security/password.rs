//! Argon2id password hashing.
//!
//! Hashing is deliberately slow, so callers should run these functions on a
//! blocking thread (see `AuthService`) instead of the async runtime.

use std::sync::OnceLock;

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("password hashing failed")]
pub struct PasswordError;

/// Hashes a plaintext password into a PHC string (`$argon2id$v=19$...`).
pub fn hash(password: &str) -> Result<String, PasswordError> {
    let salt = SaltString::generate(&mut OsRng);

    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| {
            tracing::error!(%error, "could not hash password");
            PasswordError
        })
}

/// Verifies a plaintext password against a stored PHC hash.
pub fn verify(password: &str, phc_hash: &str) -> bool {
    match PasswordHash::new(phc_hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(error) => {
            tracing::error!(%error, "stored password hash is malformed");
            false
        }
    }
}

/// Burns roughly the same CPU time as a real verification.
///
/// Called when no user matches the submitted email so that response times do
/// not reveal which addresses are registered.
pub fn verify_dummy(password: &str) {
    static DUMMY: OnceLock<Option<String>> = OnceLock::new();

    let dummy = DUMMY.get_or_init(|| hash("timing-equalisation-probe").ok());

    if let Some(dummy) = dummy {
        let _ = verify(password, dummy);
    }
}

#[cfg(test)]
mod tests {
    use super::{hash, verify};

    #[test]
    fn hashes_and_verifies() {
        let phc = hash("correct horse battery staple").expect("hashing works");

        assert!(phc.starts_with("$argon2id$"));
        assert!(verify("correct horse battery staple", &phc));
        assert!(!verify("wrong password", &phc));
    }

    #[test]
    fn rejects_malformed_hash() {
        assert!(!verify("anything", "not-a-phc-string"));
    }
}
