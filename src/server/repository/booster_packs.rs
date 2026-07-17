use sorcerers::booster::{BoosterCard, BoosterPack, UnopenedBoosterPack};

use super::{UserRepository, UserRepositoryError};

impl UserRepository {
    pub async fn claim_weekly_boosters(
        &self,
        user_id: uuid::Uuid,
        week_start: chrono::NaiveDate,
        packs: &[BoosterPack],
    ) -> Result<bool, UserRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let result = sqlx::query(
            "UPDATE users
             SET last_booster_week = $1
             WHERE id = $2 AND (last_booster_week IS NULL OR last_booster_week < $1)",
        )
        .bind(week_start)
        .bind(user_id)
        .execute(&mut *transaction)
        .await?;
        if result.rows_affected() == 0 {
            transaction.commit().await?;
            return Ok(false);
        }

        for pack in packs {
            let cards = serde_json::to_string(&pack.cards)
                .map_err(|_| UserRepositoryError::Serialization)?;
            sqlx::query(
                "INSERT INTO booster_packs (id, user_id, set_name, cards) VALUES ($1, $2, $3, $4::jsonb)",
            )
            .bind(uuid::Uuid::new_v4())
            .bind(user_id)
            .bind(&pack.set_name)
            .bind(cards)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(true)
    }

    pub async fn load_unopened_booster_packs(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<UnopenedBoosterPack>, UserRepositoryError> {
        let packs: Vec<(uuid::Uuid, String, String)> = sqlx::query_as(
            "SELECT id, set_name, cards::text
             FROM booster_packs
             WHERE user_id = $1 AND opened_at IS NULL
             ORDER BY created_at",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        packs
            .into_iter()
            .map(|(id, set_name, cards)| {
                let cards = serde_json::from_str(&cards)
                    .map_err(|_| UserRepositoryError::Serialization)?;
                Ok(UnopenedBoosterPack {
                    id,
                    pack: BoosterPack { set_name, cards },
                })
            })
            .collect()
    }

    pub async fn open_booster_pack(
        &self,
        user_id: uuid::Uuid,
        pack_id: uuid::Uuid,
    ) -> Result<Option<BoosterPack>, UserRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let pack: Option<(String, String)> = sqlx::query_as(
            "UPDATE booster_packs SET opened_at = NOW()
             WHERE id = $1 AND user_id = $2 AND opened_at IS NULL
             RETURNING set_name, cards::text",
        )
        .bind(pack_id)
        .bind(user_id)
        .fetch_optional(&mut *transaction)
        .await?;
        let Some((set_name, cards)) = pack else {
            transaction.commit().await?;
            return Ok(None);
        };
        let cards: Vec<BoosterCard> = serde_json::from_str(&cards)
            .map_err(|_| UserRepositoryError::Serialization)?;
        for card in &cards {
            sqlx::query(
                "INSERT INTO user_cards (user_id, card_name, is_foil, quantity) VALUES ($1, $2, $3, 1)
                 ON CONFLICT (user_id, card_name, is_foil)
                 DO UPDATE SET quantity = user_cards.quantity + 1",
            )
            .bind(user_id)
            .bind(&card.name)
            .bind(card.is_foil)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(Some(BoosterPack { set_name, cards }))
    }
}
