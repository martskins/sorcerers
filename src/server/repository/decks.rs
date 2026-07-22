use sorcerers::deck::DeckList;

use super::{Repository, RepositoryError as UserRepositoryError};

impl Repository {
    pub async fn load_decks(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<DeckList>, UserRepositoryError> {
        let decks: Vec<String> = sqlx::query_scalar(
            "SELECT CAST(deck AS TEXT) FROM user_decks WHERE user_id = ?1 ORDER BY created_at",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        decks
            .into_iter()
            .map(|deck| serde_json::from_str(&deck).map_err(|_| UserRepositoryError::Serialization))
            .collect()
    }
}
