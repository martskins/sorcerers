use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use sorcerers::{deck::{CardNameWithCount, DeckList, precon::PreconDeck}};
use sqlx::{PgPool, postgres::PgPoolOptions};

const MIN_USERNAME_LENGTH: usize = 3;
const MAX_USERNAME_LENGTH: usize = 32;
const MIN_PASSWORD_LENGTH: usize = 8;

#[derive(Clone)]
pub struct User {
    pub id: uuid::Uuid,
    pub username: String,
}

#[derive(Debug, thiserror::Error)]
pub enum UserRepositoryError {
    #[error("username must be {MIN_USERNAME_LENGTH}-{MAX_USERNAME_LENGTH} characters and use only letters, digits, or underscores")]
    InvalidUsername,
    #[error("password must be at least {MIN_PASSWORD_LENGTH} characters")]
    InvalidPassword,
    #[error("a user with that username already exists")]
    UsernameTaken,
    #[error("invalid username or password")]
    InvalidCredentials,
    #[error("a starter deck has already been selected")]
    StarterDeckAlreadySelected,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("password processing failed")]
    Password,
    #[error("deck serialization failed")]
    DeckSerialization,
}

#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub async fn connect(database_url: &str) -> Result<Self, UserRepositoryError> {
        let pool = PgPoolOptions::new().max_connections(5).connect(database_url).await?;
        let repository = Self { pool };
        repository.migrate().await?;
        Ok(repository)
    }

    async fn migrate(&self) -> Result<(), sqlx::Error> {
        sqlx::query("CREATE TABLE IF NOT EXISTS users (id UUID PRIMARY KEY, username TEXT NOT NULL UNIQUE, password_hash TEXT NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW())").execute(&self.pool).await?;
        sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS starter_deck TEXT").execute(&self.pool).await?;
        sqlx::query("CREATE TABLE IF NOT EXISTS user_cards (user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE, card_name TEXT NOT NULL, quantity SMALLINT NOT NULL CHECK (quantity > 0), PRIMARY KEY (user_id, card_name))").execute(&self.pool).await?;
        sqlx::query("CREATE TABLE IF NOT EXISTS user_decks (id UUID PRIMARY KEY, user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE, name TEXT NOT NULL, deck JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), UNIQUE (user_id, name))").execute(&self.pool).await?;
        Ok(())
    }

    pub async fn register(&self, username: &str, password: &str) -> Result<User, UserRepositoryError> {
        validate_username(username)?;
        if password.len() < MIN_PASSWORD_LENGTH { return Err(UserRepositoryError::InvalidPassword); }
        let password_hash = Argon2::default().hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng)).map_err(|_| UserRepositoryError::Password)?.to_string();
        let user = User { id: uuid::Uuid::new_v4(), username: username.to_owned() };
        let result = sqlx::query("INSERT INTO users (id, username, password_hash) VALUES ($1, $2, $3)").bind(user.id).bind(&user.username).bind(password_hash).execute(&self.pool).await;
        match result {
            Ok(_) => Ok(user),
            Err(sqlx::Error::Database(error)) if error.code().as_deref() == Some("23505") => Err(UserRepositoryError::UsernameTaken),
            Err(error) => Err(error.into()),
        }
    }

    pub async fn verify_login(&self, username: &str, password: &str) -> Result<User, UserRepositoryError> {
        let row: Option<(uuid::Uuid, String)> = sqlx::query_as("SELECT id, password_hash FROM users WHERE username = $1").bind(username).fetch_optional(&self.pool).await?;
        let Some((id, password_hash)) = row else { return Err(UserRepositoryError::InvalidCredentials); };
        let parsed_hash = PasswordHash::new(&password_hash).map_err(|_| UserRepositoryError::Password)?;
        Argon2::default().verify_password(password.as_bytes(), &parsed_hash).map_err(|_| UserRepositoryError::InvalidCredentials)?;
        Ok(User { id, username: username.to_owned() })
    }

    pub async fn selected_starter_deck(&self, user_id: uuid::Uuid) -> Result<Option<PreconDeck>, UserRepositoryError> {
        let deck: Option<String> = sqlx::query_scalar("SELECT starter_deck FROM users WHERE id = $1").bind(user_id).fetch_one(&self.pool).await?;
        Ok(match deck.as_deref() {
            Some("Beta - Fire") => Some(PreconDeck::BetaFire), Some("Beta - Air") => Some(PreconDeck::BetaAir),
            Some("Beta - Earth") => Some(PreconDeck::BetaEarth), Some("Beta - Water") => Some(PreconDeck::BetaWater), _ => None,
        })
    }

    pub async fn complete_starter_selection(&self, user_id: uuid::Uuid, starter_deck: &PreconDeck, deck: &DeckList, cards: &[CardNameWithCount]) -> Result<(), UserRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let result = sqlx::query("UPDATE users SET starter_deck = $1 WHERE id = $2 AND starter_deck IS NULL").bind(starter_deck.name()).bind(user_id).execute(&mut *transaction).await?;
        if result.rows_affected() != 1 { return Err(UserRepositoryError::StarterDeckAlreadySelected); }
        for card in cards {
            sqlx::query("INSERT INTO user_cards (user_id, card_name, quantity) VALUES ($1, $2, $3) ON CONFLICT (user_id, card_name) DO UPDATE SET quantity = user_cards.quantity + EXCLUDED.quantity")
                .bind(user_id).bind(&card.name).bind(i16::from(card.count)).execute(&mut *transaction).await?;
        }
        let deck_json = serde_json::to_string(deck).map_err(|_| UserRepositoryError::DeckSerialization)?;
        sqlx::query("INSERT INTO user_decks (id, user_id, name, deck) VALUES ($1, $2, $3, $4::jsonb)").bind(uuid::Uuid::new_v4()).bind(user_id).bind(&deck.name).bind(deck_json).execute(&mut *transaction).await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn load_decks(&self, user_id: uuid::Uuid) -> Result<Vec<DeckList>, UserRepositoryError> {
        let decks: Vec<String> = sqlx::query_scalar("SELECT deck::text FROM user_decks WHERE user_id = $1 ORDER BY created_at").bind(user_id).fetch_all(&self.pool).await?;
        decks.into_iter().map(|deck| serde_json::from_str(&deck).map_err(|_| UserRepositoryError::DeckSerialization)).collect()
    }

    pub async fn load_collection(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<CardNameWithCount>, UserRepositoryError> {
        let cards: Vec<(String, i16)> = sqlx::query_as(
            "SELECT card_name, quantity FROM user_cards WHERE user_id = $1 ORDER BY card_name",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(cards
            .into_iter()
            .filter_map(|(name, quantity)| {
                u8::try_from(quantity)
                    .ok()
                    .map(|count| CardNameWithCount { name, count })
            })
            .collect())
    }
}

impl UserRepositoryError {
    pub fn user_message(&self) -> &str {
        match self {
            Self::InvalidUsername => "username must be 3-32 letters, digits, or underscores", Self::InvalidPassword => "password must be at least 8 characters", Self::UsernameTaken => "a user with that username already exists", Self::InvalidCredentials => "invalid username or password", Self::StarterDeckAlreadySelected => "a starter deck has already been selected", Self::Database(_) | Self::Password | Self::DeckSerialization => "authentication service is unavailable",
        }
    }
}

fn validate_username(username: &str) -> Result<(), UserRepositoryError> {
    if !(MIN_USERNAME_LENGTH..=MAX_USERNAME_LENGTH).contains(&username.len()) || !username.chars().all(|character| character.is_ascii_alphanumeric() || character == '_') { return Err(UserRepositoryError::InvalidUsername); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_username;
    #[test]
    fn username_validation_rejects_unsafe_names() {
        assert!(validate_username("mage_7").is_ok()); assert!(validate_username("no").is_err()); assert!(validate_username("mage name").is_err());
    }
}
