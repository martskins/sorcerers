use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use sorcerers::deck::{CardNameWithCount, DeckList, precon::PreconDeck};
use sqlx::PgPool;

use super::{UserRepository, UserRepositoryError};

const MIN_USERNAME_LENGTH: usize = 3;
const MAX_USERNAME_LENGTH: usize = 32;
const MIN_PASSWORD_LENGTH: usize = 8;

#[derive(Clone)]
pub struct User {
    pub id: uuid::Uuid,
    pub username: String,
}

pub(super) async fn migrate(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await?;
    sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS starter_deck TEXT")
        .execute(pool)
        .await?;
    sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS last_booster_week DATE")
        .execute(pool)
        .await?;
    Ok(())
}

impl UserRepository {
    pub async fn register(
        &self,
        username: &str,
        password: &str,
    ) -> Result<User, UserRepositoryError> {
        validate_username(username)?;
        if password.len() < MIN_PASSWORD_LENGTH {
            return Err(UserRepositoryError::InvalidPassword);
        }

        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
            .map_err(|_| UserRepositoryError::Password)?
            .to_string();
        let user = User {
            id: uuid::Uuid::new_v4(),
            username: username.to_owned(),
        };
        let result = sqlx::query("INSERT INTO users (id, username, password_hash) VALUES ($1, $2, $3)")
            .bind(user.id)
            .bind(&user.username)
            .bind(password_hash)
            .execute(&self.pool)
            .await;
        match result {
            Ok(_) => Ok(user),
            Err(sqlx::Error::Database(error)) if error.code().as_deref() == Some("23505") => {
                Err(UserRepositoryError::UsernameTaken)
            }
            Err(error) => Err(error.into()),
        }
    }

    pub async fn verify_login(
        &self,
        username: &str,
        password: &str,
    ) -> Result<User, UserRepositoryError> {
        let row: Option<(uuid::Uuid, String)> =
            sqlx::query_as("SELECT id, password_hash FROM users WHERE username = $1")
                .bind(username)
                .fetch_optional(&self.pool)
                .await?;
        let Some((id, password_hash)) = row else {
            return Err(UserRepositoryError::InvalidCredentials);
        };
        let parsed_hash = PasswordHash::new(&password_hash).map_err(|_| UserRepositoryError::Password)?;
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| UserRepositoryError::InvalidCredentials)?;
        Ok(User {
            id,
            username: username.to_owned(),
        })
    }

    pub async fn selected_starter_deck(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Option<PreconDeck>, UserRepositoryError> {
        let deck: Option<String> = sqlx::query_scalar("SELECT starter_deck FROM users WHERE id = $1")
            .bind(user_id)
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
            "UPDATE users SET starter_deck = $1 WHERE id = $2 AND starter_deck IS NULL",
        )
        .bind(starter_deck.name())
        .bind(user_id)
        .execute(&mut *transaction)
        .await?;
        if result.rows_affected() != 1 {
            return Err(UserRepositoryError::StarterDeckAlreadySelected);
        }

        for card in cards {
            sqlx::query(
                "INSERT INTO user_cards (user_id, card_name, is_foil, quantity) VALUES ($1, $2, FALSE, $3)
                 ON CONFLICT (user_id, card_name, is_foil)
                 DO UPDATE SET quantity = user_cards.quantity + EXCLUDED.quantity",
            )
            .bind(user_id)
            .bind(&card.name)
            .bind(i16::from(card.count))
            .execute(&mut *transaction)
            .await?;
        }

        let deck_json = serde_json::to_string(deck).map_err(|_| UserRepositoryError::Serialization)?;
        sqlx::query("INSERT INTO user_decks (id, user_id, name, deck) VALUES ($1, $2, $3, $4::jsonb)")
            .bind(uuid::Uuid::new_v4())
            .bind(user_id)
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

#[cfg(test)]
mod tests {
    use super::validate_username;

    #[test]
    fn username_validation_rejects_unsafe_names() {
        assert!(validate_username("mage_7").is_ok());
        assert!(validate_username("no").is_err());
        assert!(validate_username("mage name").is_err());
    }
}
