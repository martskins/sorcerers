use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use chrono::{Duration, Utc};
use email_address::EmailAddress;
use sorcerers::deck::{CardNameWithCount, DeckList, precon::PreconDeck};

use super::{Repository, RepositoryError as UserRepositoryError};

const MIN_USERNAME_LENGTH: usize = 3;
const MAX_USERNAME_LENGTH: usize = 32;
const MIN_PASSWORD_LENGTH: usize = 8;
const CONFIRMATION_CODE_LIFETIME_MINUTES: i64 = 15;
const MAX_CONFIRMATION_ATTEMPTS: i16 = 5;

#[derive(Clone)]
pub struct User {
    pub id: uuid::Uuid,
    pub username: String,
}

pub struct PendingEmailConfirmation {
    pub email: String,
    pub code: String,
}

impl Repository {
    pub async fn register(
        &self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<PendingEmailConfirmation, UserRepositoryError> {
        validate_username(username)?;
        validate_email(email)?;
        if password.len() < MIN_PASSWORD_LENGTH {
            return Err(UserRepositoryError::InvalidPassword);
        }

        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
            .map_err(|_| UserRepositoryError::Password)?
            .to_string();
        let pending = new_pending_email_confirmation(email)?;
        let id = uuid::Uuid::new_v4();
        let expires_at = Utc::now() + Duration::minutes(CONFIRMATION_CODE_LIFETIME_MINUTES);
        let result = sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, confirmation_code_hash, confirmation_code_expires_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
            .bind(id.to_string())
            .bind(username)
            .bind(&pending.email)
            .bind(password_hash)
            .bind(hash_confirmation_code(&pending.code)?)
            .bind(expires_at.to_rfc3339())
            .execute(&self.pool)
            .await;
        match result {
            Ok(_) => Ok(pending),
            Err(sqlx::Error::Database(error)) if is_unique_violation(error.as_ref()) => {
                let email_taken: Option<i64> =
                    sqlx::query_scalar("SELECT CAST(1 AS BIGINT) FROM users WHERE email = ?1")
                        .bind(&pending.email)
                        .fetch_optional(&self.pool)
                        .await?;
                Err(if email_taken.is_some() {
                    UserRepositoryError::EmailTaken
                } else {
                    UserRepositoryError::UsernameTaken
                })
            }
            Err(error) => Err(error.into()),
        }
    }

    pub async fn verify_login(
        &self,
        email: &str,
        password: &str,
    ) -> Result<User, UserRepositoryError> {
        let row: Option<(String, String, String, Option<String>)> = sqlx::query_as(
            "SELECT CAST(id AS TEXT), password_hash, username, CAST(email_confirmed_at AS TEXT) FROM users WHERE email = ?1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;
        let Some((id, password_hash, username, email_confirmed_at)) = row else {
            return Err(UserRepositoryError::InvalidCredentials);
        };
        let parsed_hash =
            PasswordHash::new(&password_hash).map_err(|_| UserRepositoryError::Password)?;
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| UserRepositoryError::InvalidCredentials)?;
        if email_confirmed_at.is_none() {
            return Err(UserRepositoryError::EmailConfirmationRequired(
                email.to_string(),
            ));
        }
        Ok(User {
            id: id.parse().map_err(|_| UserRepositoryError::Serialization)?,
            username,
        })
    }

    pub async fn resend_email_confirmation(
        &self,
        email: &str,
    ) -> Result<PendingEmailConfirmation, UserRepositoryError> {
        validate_email(email)?;
        let pending = new_pending_email_confirmation(email)?;
        let expires_at = Utc::now() + Duration::minutes(CONFIRMATION_CODE_LIFETIME_MINUTES);
        let result = sqlx::query(
            "UPDATE users SET confirmation_code_hash = ?1, confirmation_code_expires_at = ?2, confirmation_attempts = 0 WHERE email = ?3 AND email_confirmed_at IS NULL",
        )
        .bind(hash_confirmation_code(&pending.code)?)
        .bind(expires_at.to_rfc3339())
        .bind(&pending.email)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(UserRepositoryError::EmailAlreadyConfirmed);
        }
        Ok(pending)
    }

    pub async fn confirm_email(
        &self,
        email: &str,
        code: &str,
    ) -> Result<User, UserRepositoryError> {
        validate_email(email)?;
        let row: Option<(String, String, Option<String>, bool, i64)> = sqlx::query_as(
            "SELECT CAST(id AS TEXT), username, confirmation_code_hash,
                    confirmation_code_expires_at > CURRENT_TIMESTAMP,
                    CAST(confirmation_attempts AS BIGINT)
             FROM users WHERE email = ?1 AND email_confirmed_at IS NULL",
        )
        .bind(email.trim())
        .fetch_optional(&self.pool)
        .await?;
        let Some((id, username, code_hash, code_not_expired, attempts)) = row else {
            return Err(UserRepositoryError::InvalidConfirmationCode);
        };
        if attempts >= i64::from(MAX_CONFIRMATION_ATTEMPTS) {
            return Err(UserRepositoryError::ConfirmationAttemptsExceeded);
        }
        let is_valid = code_not_expired
            && code_hash
                .as_deref()
                .and_then(|hash| PasswordHash::new(hash).ok())
                .is_some_and(|hash| {
                    Argon2::default()
                        .verify_password(code.as_bytes(), &hash)
                        .is_ok()
                });
        if !is_valid {
            sqlx::query(
                "UPDATE users SET confirmation_attempts = confirmation_attempts + 1 WHERE id = ?1",
            )
            .bind(&id)
            .execute(&self.pool)
            .await?;
            return Err(UserRepositoryError::InvalidConfirmationCode);
        }
        sqlx::query(
            "UPDATE users SET email_confirmed_at = CURRENT_TIMESTAMP, confirmation_code_hash = NULL, confirmation_code_expires_at = NULL, confirmation_attempts = 0 WHERE id = ?1",
        )
        .bind(&id)
        .execute(&self.pool)
        .await?;
        Ok(User {
            id: id.parse().map_err(|_| UserRepositoryError::Serialization)?,
            username,
        })
    }

    pub async fn selected_starter_deck(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Option<PreconDeck>, UserRepositoryError> {
        let deck: Option<String> =
            sqlx::query_scalar("SELECT starter_deck FROM users WHERE id = ?1")
                .bind(user_id.to_string())
                .fetch_one(&self.pool)
                .await?;
        Ok(match deck.as_deref() {
            Some("Beta - Fire") => Some(PreconDeck::BetaFire),
            Some("Beta - Air") => Some(PreconDeck::BetaAir),
            Some("Beta - Earth") => Some(PreconDeck::BetaEarth),
            Some("Beta - Water") => Some(PreconDeck::BetaWater),
            _ => None,
        })
    }

    /// Coordinates the initial user, card, and deck records in one transaction.
    pub async fn complete_starter_selection(
        &self,
        user_id: uuid::Uuid,
        starter_deck: &PreconDeck,
        deck: &DeckList,
        cards: &[CardNameWithCount],
    ) -> Result<(), UserRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let result = sqlx::query(
            "UPDATE users SET starter_deck = ?1 WHERE id = ?2 AND starter_deck IS NULL",
        )
        .bind(starter_deck.name())
        .bind(user_id.to_string())
        .execute(&mut *transaction)
        .await?;
        if result.rows_affected() != 1 {
            return Err(UserRepositoryError::StarterDeckAlreadySelected);
        }

        for card in cards {
            sqlx::query(
                "INSERT INTO user_cards (user_id, card_name, is_foil, quantity) VALUES (?1, ?2, FALSE, ?3)
                 ON CONFLICT (user_id, card_name, is_foil)
                 DO UPDATE SET quantity = user_cards.quantity + EXCLUDED.quantity",
            )
            .bind(user_id.to_string())
            .bind(&card.name)
            .bind(i16::from(card.count))
            .execute(&mut *transaction)
            .await?;
        }

        let deck_json =
            serde_json::to_string(deck).map_err(|_| UserRepositoryError::Serialization)?;
        sqlx::query("INSERT INTO user_decks (id, user_id, name, deck) VALUES (?1, ?2, ?3, ?4)")
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(user_id.to_string())
            .bind(&deck.name)
            .bind(deck_json)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }
}

fn validate_username(username: &str) -> Result<(), UserRepositoryError> {
    if !(MIN_USERNAME_LENGTH..=MAX_USERNAME_LENGTH).contains(&username.len())
        || !username
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return Err(UserRepositoryError::InvalidUsername);
    }
    Ok(())
}

fn validate_email(email: &str) -> Result<(), UserRepositoryError> {
    EmailAddress::is_valid(email.trim())
        .then_some(())
        .ok_or(UserRepositoryError::InvalidEmail)
}

fn new_pending_email_confirmation(
    email: &str,
) -> Result<PendingEmailConfirmation, UserRepositoryError> {
    let email = email.trim().to_lowercase();
    validate_email(&email)?;
    let code = format!("{:06}", uuid::Uuid::new_v4().as_u128() % 1_000_000);
    Ok(PendingEmailConfirmation { email, code })
}

fn hash_confirmation_code(code: &str) -> Result<String, UserRepositoryError> {
    Argon2::default()
        .hash_password(code.as_bytes(), &SaltString::generate(&mut OsRng))
        .map_err(|_| UserRepositoryError::Password)
        .map(|hash| hash.to_string())
}

fn is_unique_violation(error: &dyn sqlx::error::DatabaseError) -> bool {
    matches!(
        error.code().as_deref(),
        Some("23505") | Some("2067") | Some("1555")
    )
}

#[cfg(test)]
mod tests {
    use super::{new_pending_email_confirmation, validate_email, validate_username};

    #[test]
    fn username_validation_rejects_unsafe_names() {
        assert!(validate_username("mage_7").is_ok());
        assert!(validate_username("no").is_err());
        assert!(validate_username("mage name").is_err());
    }

    #[test]
    fn email_validation_requires_a_deliverable_shape() {
        assert!(validate_email("player@example.com").is_ok());
        assert!(validate_email("not-an-email").is_err());
    }

    #[test]
    fn confirmation_codes_are_six_digits() {
        let pending = new_pending_email_confirmation("player@example.com").unwrap();
        assert_eq!(pending.code.len(), 6);
        assert!(
            pending
                .code
                .chars()
                .all(|character| character.is_ascii_digit())
        );
    }
}
