use sorcerers::collection::CollectedCard;

use super::{UserRepository, UserRepositoryError};

impl UserRepository {
    pub async fn load_collection(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<CollectedCard>, UserRepositoryError> {
        let cards: Vec<(String, i16, bool)> = sqlx::query_as(
            "SELECT card_name, quantity, is_foil FROM user_cards WHERE user_id = $1 ORDER BY card_name, is_foil",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(cards
            .into_iter()
            .filter_map(|(name, quantity, is_foil)| {
                u8::try_from(quantity)
                    .ok()
                    .map(|count| CollectedCard { name, count, is_foil })
            })
            .collect())
    }
}
