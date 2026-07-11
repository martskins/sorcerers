use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use sqlx::{PgPool, postgres::PgPoolOptions};

const MIN_USERNAME_LENGTH: usize = 3;
const MAX_USERNAME_LENGTH: usize = 32;
const MIN_PASSWORD_LENGTH: usize = 8;

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
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("password processing failed")]
    Password,
}

#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
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
                password_hash TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn register(&self, username: &str, password: &str) -> Result<(), UserRepositoryError> {
        validate_username(username)?;
        if password.len() < MIN_PASSWORD_LENGTH {
            return Err(UserRepositoryError::InvalidPassword);
        }

        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|_| UserRepositoryError::Password)?
            .to_string();
        let result = sqlx::query("INSERT INTO users (id, username, password_hash) VALUES ($1, $2, $3)")
            .bind(uuid::Uuid::new_v4())
            .bind(username)
            .bind(password_hash)
            .execute(&self.pool)
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(error)) if error.code().as_deref() == Some("23505") => {
                Err(UserRepositoryError::UsernameTaken)
            }
            Err(error) => Err(error.into()),
        }
    }

    pub async fn verify_login(&self, username: &str, password: &str) -> Result<(), UserRepositoryError> {
        let password_hash: Option<String> =
            sqlx::query_scalar("SELECT password_hash FROM users WHERE username = $1")
                .bind(username)
                .fetch_optional(&self.pool)
                .await?;
        let Some(password_hash) = password_hash else {
            return Err(UserRepositoryError::InvalidCredentials);
        };
        let parsed_hash = PasswordHash::new(&password_hash).map_err(|_| UserRepositoryError::Password)?;
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| UserRepositoryError::InvalidCredentials)
    }
}

impl UserRepositoryError {
    pub fn user_message(&self) -> &str {
        match self {
            Self::InvalidUsername => "username must be 3-32 letters, digits, or underscores",
            Self::InvalidPassword => "password must be at least 8 characters",
            Self::UsernameTaken => "a user with that username already exists",
            Self::InvalidCredentials => "invalid username or password",
            Self::Database(_) | Self::Password => "authentication service is unavailable",
        }
    }
}

fn validate_username(username: &str) -> Result<(), UserRepositoryError> {
    if !(MIN_USERNAME_LENGTH..=MAX_USERNAME_LENGTH).contains(&username.len())
        || !username.chars().all(|character| character.is_ascii_alphanumeric() || character == '_')
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
