mod booster_packs;
mod cards;
mod decks;
mod users;

use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

pub use users::User;

#[derive(Clone)]
pub struct Repository {
    pub(super) pool: SqlitePool,
}

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
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
    #[error("not enough reward points")]
    InsufficientRewardPoints,
    #[error("a starter deck has already been selected")]
    StarterDeckAlreadySelected,
    #[error("DATABASE_URL must use a sqlite: URL")]
    UnsupportedDatabase,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("password processing failed")]
    Password,
    #[error("data serialization failed")]
    Serialization,
}

impl Repository {
    pub async fn connect(database_url: &str) -> Result<Self, RepositoryError> {
        let connection_url = sqlite_connection_url(database_url)?;
        let pool = SqlitePoolOptions::new()
            // An in-memory SQLite database exists per connection. A single connection
            // also avoids avoidable writer contention for the embedded deployment.
            .max_connections(1)
            .connect(&connection_url)
            .await?;
        let repository = Self { pool };
        repository.migrate().await?;
        Ok(repository)
    }

    async fn migrate(&self) -> Result<(), sqlx::Error> {
        // SQLite has no `ADD COLUMN IF NOT EXISTS`, so its complete schema is created
        // atomically for a new database. SQLite support is new, so there is no prior
        // SQLite schema to upgrade.
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                username TEXT NOT NULL UNIQUE,
                email TEXT UNIQUE,
                password_hash TEXT NOT NULL,
                email_confirmed_at TEXT,
                confirmation_code_hash TEXT,
                confirmation_code_expires_at TEXT,
                confirmation_attempts INTEGER NOT NULL DEFAULT 0,
                starter_deck TEXT,
                last_booster_week TEXT,
                reward_points INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS users_email_unique_idx
             ON users (email) WHERE email IS NOT NULL",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS game_rewards (
                game_id TEXT NOT NULL,
                user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                points INTEGER NOT NULL CHECK (points > 0),
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (game_id, user_id)
            )",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS user_cards (
                user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                card_name TEXT NOT NULL,
                is_foil BOOLEAN NOT NULL DEFAULT FALSE,
                quantity INTEGER NOT NULL CHECK (quantity > 0),
                PRIMARY KEY (user_id, card_name, is_foil)
            )",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS user_decks (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                deck TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE (user_id, name)
            )",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS booster_packs (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                set_name TEXT NOT NULL,
                cards TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                opened_at TEXT
            )",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

fn sqlite_connection_url(database_url: &str) -> Result<String, RepositoryError> {
    if !database_url.starts_with("sqlite:") {
        return Err(RepositoryError::UnsupportedDatabase);
    }
    if database_url == "sqlite::memory:" || database_url.contains("mode=") {
        return Ok(database_url.to_owned());
    }

    let separator = if database_url.contains('?') { '&' } else { '?' };
    Ok(format!("{database_url}{separator}mode=rwc"))
}

impl RepositoryError {
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
            Self::InsufficientRewardPoints => "not enough reward points for that booster",
            Self::StarterDeckAlreadySelected => "a starter deck has already been selected",
            Self::Database(_)
            | Self::UnsupportedDatabase
            | Self::Password
            | Self::Serialization => "authentication service is unavailable",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Repository, RepositoryError, sqlite_connection_url};

    #[test]
    fn sqlite_file_urls_create_the_database_by_default() {
        assert_eq!(
            sqlite_connection_url("sqlite://sorcerers.db").unwrap(),
            "sqlite://sorcerers.db?mode=rwc"
        );
        assert_eq!(
            sqlite_connection_url("sqlite://existing.db?mode=ro").unwrap(),
            "sqlite://existing.db?mode=ro"
        );
    }

    #[test]
    fn non_sqlite_urls_are_rejected() {
        assert!(matches!(
            sqlite_connection_url("mysql://localhost/sorcerers"),
            Err(RepositoryError::UnsupportedDatabase)
        ));
    }

    #[tokio::test]
    async fn sqlite_repository_initializes_and_enforces_unique_emails() {
        let repository = Repository::connect("sqlite::memory:").await.unwrap();

        let pending = repository
            .register("mage_one", "mage@example.com", "very-secret-password")
            .await
            .unwrap();
        let user = repository
            .confirm_email(&pending.email, &pending.code)
            .await
            .unwrap();
        assert_eq!(
            repository
                .verify_login("mage@example.com", "very-secret-password")
                .await
                .unwrap()
                .id,
            user.id
        );

        let game_id = uuid::Uuid::new_v4();
        assert_eq!(
            repository
                .award_match_points(game_id, user.id, true)
                .await
                .unwrap()
                .points_earned,
            10
        );
        assert_eq!(
            repository
                .award_match_points(game_id, user.id, true)
                .await
                .unwrap()
                .points_earned,
            0
        );
        assert_eq!(repository.reward_points(user.id).await.unwrap(), 10);

        let duplicate = repository
            .register("mage_two", "mage@example.com", "very-secret-password")
            .await;

        assert!(matches!(duplicate, Err(RepositoryError::EmailTaken)));
    }

    #[tokio::test]
    async fn sqlite_file_url_connects_without_a_network_lookup() {
        let path = std::env::temp_dir().join(format!("sorcerers-{}.db", uuid::Uuid::new_v4()));
        let database_url = format!("sqlite://{}", path.display());

        let repository = Repository::connect(&database_url).await.unwrap();
        repository.pool.close().await;
        std::fs::remove_file(path).unwrap();
    }
}
