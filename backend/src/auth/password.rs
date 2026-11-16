use argon2::Argon2;
use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};

use crate::api::AppError;

pub fn password_policy(password: &str) -> Result<(), AppError> {
    if (12..=128).contains(&password.len()) {
        Ok(())
    } else {
        Err(AppError::Validation(
            "Use a password between 12 and 128 characters.".into(),
        ))
    }
}

pub fn hash_password(password: &str) -> Result<String, AppError> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(|_| AppError::Internal)
}

pub fn verify_password(password: &str, encoded: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(encoded) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

pub fn normalize_email(value: &str) -> Result<String, AppError> {
    let email = value.trim().to_lowercase();
    let Some((local, domain)) = email.split_once('@') else {
        return Err(AppError::Validation("Enter a valid email address.".into()));
    };
    if local.is_empty()
        || !domain.contains('.')
        || domain.starts_with('.')
        || domain.ends_with('.')
        || email.contains(char::is_whitespace)
        || email.len() > 254
    {
        return Err(AppError::Validation("Enter a valid email address.".into()));
    }
    Ok(email)
}

pub fn display_name(value: &str) -> Result<String, AppError> {
    let name = value.trim();
    if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
        return Err(AppError::Validation(
            "Enter a name up to 80 characters.".into(),
        ));
    }
    Ok(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::{display_name, hash_password, normalize_email, password_policy, verify_password};

    #[test]
    fn a_password_round_trips_and_rejects_the_wrong_one() {
        let encoded = hash_password("correct-horse-battery").expect("hash");
        assert!(verify_password("correct-horse-battery", &encoded));
        assert!(!verify_password("correct-horse-batteries", &encoded));
        assert!(!verify_password("correct-horse-battery", "not-a-hash"));
    }

    #[test]
    fn short_passwords_are_rejected() {
        assert!(password_policy("short").is_err());
        assert!(password_policy("long-enough-password").is_ok());
    }

    #[test]
    fn emails_are_normalized() {
        assert_eq!(
            normalize_email("  Adaeze@Kamfany.com ").unwrap(),
            "adaeze@kamfany.com"
        );
        assert!(normalize_email("adaeze").is_err());
        assert!(normalize_email("adaeze@localhost").is_err());
    }

    #[test]
    fn names_cannot_be_blank() {
        assert_eq!(display_name("  Adaeze  ").unwrap(), "Adaeze");
        assert!(display_name("   ").is_err());
    }
}
