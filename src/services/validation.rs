//! Small hand-rolled validators.
//!
//! Rules live here (not in the handlers) so the same checks apply no matter
//! which entry point calls the service.

pub const NAME_MIN: usize = 2;
pub const NAME_MAX: usize = 80;
pub const EMAIL_MAX: usize = 254;
pub const PASSWORD_MIN: usize = 8;
pub const PASSWORD_MAX: usize = 128;

/// Collected, visitor facing validation messages.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Errors(Vec<String>);

impl Errors {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, message: impl Into<String>) {
        self.0.push(message.into());
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn into_messages(self) -> Vec<String> {
        self.0
    }

    pub fn messages(&self) -> &[String] {
        &self.0
    }
}

pub fn validate_name(name: &str, errors: &mut Errors) {
    let length = name.trim().chars().count();

    if length < NAME_MIN {
        errors.push(format!("Name must be at least {NAME_MIN} characters."));
    } else if length > NAME_MAX {
        errors.push(format!("Name must be {NAME_MAX} characters or fewer."));
    }
}

/// Pragmatic email shape check: `local@domain.tld`, no spaces.
pub fn validate_email(email: &str, errors: &mut Errors) {
    let email = email.trim();

    if email.is_empty() {
        errors.push("Email is required.");
        return;
    }

    if email.len() > EMAIL_MAX {
        errors.push(format!("Email must be {EMAIL_MAX} characters or fewer."));
        return;
    }

    let shape_is_valid = match email.split_once('@') {
        Some((local, domain)) => {
            !local.is_empty()
                && !domain.is_empty()
                && !email.contains(char::is_whitespace)
                && !domain.contains('@')
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
                && !domain.contains("..")
        }
        None => false,
    };

    if !shape_is_valid {
        errors.push("Enter a valid email address.");
    }
}

pub fn validate_password(password: &str, confirmation: Option<&str>, errors: &mut Errors) {
    let length = password.chars().count();

    if length < PASSWORD_MIN {
        errors.push(format!(
            "Password must be at least {PASSWORD_MIN} characters."
        ));
    } else if length > PASSWORD_MAX {
        errors.push(format!(
            "Password must be {PASSWORD_MAX} characters or fewer."
        ));
    } else if !password.chars().any(char::is_alphabetic)
        || !password.chars().any(|c| c.is_ascii_digit())
    {
        errors.push("Password must contain at least one letter and one number.");
    }

    if let Some(confirmation) = confirmation
        && password != confirmation
    {
        errors.push("Passwords do not match.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn messages(email: &str) -> Vec<String> {
        let mut errors = Errors::new();
        validate_email(email, &mut errors);
        errors.into_messages()
    }

    #[test]
    fn accepts_reasonable_emails() {
        assert!(messages("ada.lovelace+dev@example.co.uk").is_empty());
    }

    #[test]
    fn rejects_broken_emails() {
        for email in [
            "",
            "ada",
            "ada@",
            "@example.com",
            "ada@example",
            "a b@c.com",
        ] {
            assert!(!messages(email).is_empty(), "{email} should be rejected");
        }
    }

    #[test]
    fn password_rules() {
        let mut errors = Errors::new();
        validate_password("short", None, &mut errors);
        assert!(!errors.is_empty());

        let mut errors = Errors::new();
        validate_password("letters-only", None, &mut errors);
        assert!(!errors.is_empty());

        let mut errors = Errors::new();
        validate_password("strong-pass-1", Some("strong-pass-2"), &mut errors);
        assert_eq!(errors.messages().len(), 1);

        let mut errors = Errors::new();
        validate_password("strong-pass-1", Some("strong-pass-1"), &mut errors);
        assert!(errors.is_empty());
    }

    #[test]
    fn name_rules() {
        let mut errors = Errors::new();
        validate_name("A", &mut errors);
        assert!(!errors.is_empty());

        let mut errors = Errors::new();
        validate_name("Ada Lovelace", &mut errors);
        assert!(errors.is_empty());
    }
}
