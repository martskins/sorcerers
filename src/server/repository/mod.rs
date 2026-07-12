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
        users::migrate(&self.pool).await?;
        cards::migrate(&self.pool).await?;
        decks::migrate(&self.pool).await?;
        booster_packs::migrate(&self.pool).await?;
        Ok(())
    }
}

impl UserRepositoryError {
    pub fn user_message(&self) -> &str {
        match self {
            Self::InvalidUsername => "username must be 3-32 letters, digits, or underscores",
            Self::InvalidPassword => "password must be at least 8 characters",
            Self::UsernameTaken => "a user with that username already exists",
            Self::InvalidCredentials => "invalid username or password",
            Self::StarterDeckAlreadySelected => "a starter deck has already been selected",
            Self::Database(_) | Self::Password | Self::Serialization => {
                "authentication service is unavailable"
            }
        }
    }
}
