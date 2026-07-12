use sorcerers::collection::CollectedCard;
use sqlx::PgPool;

use super::{UserRepository, UserRepositoryError};

pub(super) async fn migrate(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS user_cards (
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            card_name TEXT NOT NULL,
            is_foil BOOLEAN NOT NULL DEFAULT FALSE,
            quantity SMALLINT NOT NULL CHECK (quantity > 0),
            PRIMARY KEY (user_id, card_name, is_foil)
        )",
    )
    .execute(pool)
    .await?;
    sqlx::query("ALTER TABLE user_cards ADD COLUMN IF NOT EXISTS is_foil BOOLEAN NOT NULL DEFAULT FALSE")
        .execute(pool)
        .await?;
    // The first version of the collection schema used `(user_id, card_name)` as
    // the key. Split foil and non-foil copies into distinct collection entries
    // without needlessly rebuilding the primary key on every startup.
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
        .execute(pool)
        .await?;
    Ok(())
}

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
