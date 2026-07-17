use sorcerers::deck::DeckList;

use super::{UserRepository, UserRepositoryError};

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
