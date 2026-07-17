mod booster_packs;
mod cards;
mod decks;
mod users;

use sqlx::{PgPool, postgres::PgPoolOptions};

pub use users::User;

#[derive(Clone)]
pub struct UserRepository {
    pub(super) pool: PgPool,
}

#[derive(Debug, thiserror::Error)]
pub enum UserRepositoryError {
    #[error("username must be 3-32 characters and use only letters, digits, or underscores")]
    InvalidUsername,
    #[error("password must be at least 8 characters")]
    InvalidPassword,
    #[error("enter a valid email address")]
    InvalidEmail,
    #[error("a user with that username already exists")]
    UsernameTaken,
    #[error("an account already uses that email address")]
    EmailTaken,
    #[error("invalid username or password")]
    InvalidCredentials,
    #[error("email confirmation is required")]
    EmailConfirmationRequired(String),
    #[error("the confirmation code is invalid or has expired")]
    InvalidConfirmationCode,
    #[error("too many confirmation attempts; request a new code")]
    ConfirmationAttemptsExceeded,
    #[error("that email address has already been confirmed")]
    EmailAlreadyConfirmed,
    #[error("a starter deck has already been selected")]
    StarterDeckAlreadySelected,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("password processing failed")]
    Password,
    #[error("data serialization failed")]
    Serialization,
}

impl UserRepository {
    pub async fn connect(database_url: &str) -> Result<Self, UserRepositoryError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        let repository = Self { pool };
        repository.migrate().await?;
        Ok(repository)
    }

    async fn migrate(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS users (
                id UUID PRIMARY KEY,
                username TEXT NOT NULL UNIQUE,
                email TEXT UNIQUE,
                password_hash TEXT NOT NULL,
                email_confirmed_at TIMESTAMPTZ,
                confirmation_code_hash TEXT,
                confirmation_code_expires_at TIMESTAMPTZ,
                confirmation_attempts SMALLINT NOT NULL DEFAULT 0,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS email TEXT")
            .execute(&self.pool)
            .await?;
        sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS email_confirmed_at TIMESTAMPTZ")
            .execute(&self.pool)
            .await?;
        sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS confirmation_code_hash TEXT")
            .execute(&self.pool)
            .await?;
        sqlx::query(
            "ALTER TABLE users ADD COLUMN IF NOT EXISTS confirmation_code_expires_at TIMESTAMPTZ",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "ALTER TABLE users ADD COLUMN IF NOT EXISTS confirmation_attempts SMALLINT NOT NULL DEFAULT 0",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS users_email_unique_idx ON users (email) WHERE email IS NOT NULL",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS starter_deck TEXT")
            .execute(&self.pool)
            .await?;
        sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS last_booster_week DATE")
            .execute(&self.pool)
            .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS user_cards (
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                card_name TEXT NOT NULL,
                is_foil BOOLEAN NOT NULL DEFAULT FALSE,
                quantity SMALLINT NOT NULL CHECK (quantity > 0),
                PRIMARY KEY (user_id, card_name, is_foil)
            )",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "ALTER TABLE user_cards ADD COLUMN IF NOT EXISTS is_foil BOOLEAN NOT NULL DEFAULT FALSE",
        )
        .execute(&self.pool)
        .await?;
        // The first version of the collection schema used `(user_id, card_name)` as
        // the key. Split foil and non-foil copies without rebuilding the primary
        // key on every startup.
        sqlx::query(
            "DO $$
            DECLARE previous_key TEXT;
            BEGIN
                SELECT conname INTO previous_key
                FROM pg_constraint
                WHERE conrelid = 'user_cards'::regclass
                  AND contype = 'p'
                  AND pg_get_constraintdef(oid) = 'PRIMARY KEY (user_id, card_name)';
                IF previous_key IS NOT NULL THEN
                    EXECUTE format('ALTER TABLE user_cards DROP CONSTRAINT %I', previous_key);
                    ALTER TABLE user_cards ADD PRIMARY KEY (user_id, card_name, is_foil);
                END IF;
            END $$",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS user_decks (
                id UUID PRIMARY KEY,
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                deck JSONB NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                UNIQUE (user_id, name)
            )",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS booster_packs (
                id UUID PRIMARY KEY,
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                set_name TEXT NOT NULL,
                cards JSONB NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                opened_at TIMESTAMPTZ
            )",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

impl UserRepositoryError {
    pub fn user_message(&self) -> &str {
        match self {
            Self::InvalidUsername => "username must be 3-32 letters, digits, or underscores",
            Self::InvalidPassword => "password must be at least 8 characters",
            Self::InvalidEmail => "enter a valid email address",
            Self::UsernameTaken => "a user with that username already exists",
            Self::EmailTaken => "an account already uses that email address",
            Self::InvalidCredentials => "invalid username or password",
            Self::EmailConfirmationRequired(_) => "confirm your email address to continue",
            Self::InvalidConfirmationCode => "that confirmation code is invalid or has expired",
            Self::ConfirmationAttemptsExceeded => {
                "too many confirmation attempts; request a new code"
            }
            Self::EmailAlreadyConfirmed => "that email address has already been confirmed",
            Self::StarterDeckAlreadySelected => "a starter deck has already been selected",
            Self::Database(_) | Self::Password | Self::Serialization => {
                "authentication service is unavailable"
            }
        }
    }
}
