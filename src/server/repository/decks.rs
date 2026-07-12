use sorcerers::deck::DeckList;
use sqlx::PgPool;

use super::{UserRepository, UserRepositoryError};

pub(super) async fn migrate(pool: &PgPool) -> Result<(), sqlx::Error> {
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
    .execute(pool)
    .await?;
    Ok(())
}

impl UserRepository {
    pub async fn load_decks(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<DeckList>, UserRepositoryError> {
        let decks: Vec<String> = sqlx::query_scalar(
            "SELECT deck::text FROM user_decks WHERE user_id = $1 ORDER BY created_at",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        decks
            .into_iter()
            .map(|deck| serde_json::from_str(&deck).map_err(|_| UserRepositoryError::Serialization))
            .collect()
    }
}
