use sorcerers::booster::{BoosterCard, BoosterPack, UnopenedBoosterPack};

use super::{Repository, RepositoryError as UserRepositoryError};

pub const WIN_REWARD_POINTS: i32 = 10;
pub const PLAY_REWARD_POINTS: i32 = 2;
pub const BETA_BOOSTER_COST: i32 = WIN_REWARD_POINTS * 3;

pub struct MatchReward {
    pub points_earned: u32,
    pub reward_points: u32,
}

impl Repository {
    pub async fn reward_points(&self, user_id: uuid::Uuid) -> Result<u32, UserRepositoryError> {
        let points: i64 = sqlx::query_scalar(
            "SELECT CAST(reward_points AS BIGINT) FROM users WHERE id = ?1",
        )
            .bind(user_id.to_string())
            .fetch_one(&self.pool)
            .await?;
        Ok(points.max(0) as u32)
    }

    pub async fn award_match_points(
        &self,
        game_id: uuid::Uuid,
        user_id: uuid::Uuid,
        won: bool,
    ) -> Result<MatchReward, UserRepositoryError> {
        let points = if won { WIN_REWARD_POINTS } else { PLAY_REWARD_POINTS };
        let mut transaction = self.pool.begin().await?;
        let awarded: Option<i64> = sqlx::query_scalar(
            "INSERT INTO game_rewards (game_id, user_id, points) VALUES (?1, ?2, ?3)
             ON CONFLICT (game_id, user_id) DO NOTHING
            RETURNING CAST(points AS BIGINT)",
        )
        .bind(game_id.to_string())
        .bind(user_id.to_string())
        .bind(points)
        .fetch_optional(&mut *transaction)
        .await?;
        let reward_points: i64 = if let Some(points_earned) = awarded {
            sqlx::query_scalar(
                "UPDATE users SET reward_points = reward_points + ?1 WHERE id = ?2 RETURNING CAST(reward_points AS BIGINT)",
            )
            .bind(points_earned)
            .bind(user_id.to_string())
            .fetch_one(&mut *transaction)
            .await?
        } else {
            sqlx::query_scalar("SELECT CAST(reward_points AS BIGINT) FROM users WHERE id = ?1")
                .bind(user_id.to_string())
                .fetch_one(&mut *transaction)
                .await?
        };
        transaction.commit().await?;
        Ok(MatchReward {
            points_earned: awarded.unwrap_or_default().max(0) as u32,
            reward_points: reward_points.max(0) as u32,
        })
    }

    pub async fn redeem_beta_booster(
        &self,
        user_id: uuid::Uuid,
        pack: BoosterPack,
    ) -> Result<(u32, UnopenedBoosterPack), UserRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let reward_points: Option<i64> = sqlx::query_scalar(
            "UPDATE users SET reward_points = reward_points - ?1
             WHERE id = ?2 AND reward_points >= ?1
             RETURNING CAST(reward_points AS BIGINT)",
        )
        .bind(BETA_BOOSTER_COST)
        .bind(user_id.to_string())
        .fetch_optional(&mut *transaction)
        .await?;
        let Some(reward_points) = reward_points else {
            transaction.rollback().await?;
            return Err(UserRepositoryError::InsufficientRewardPoints);
        };
        let cards = serde_json::to_string(&pack.cards)
            .map_err(|_| UserRepositoryError::Serialization)?;
        let pack_id = uuid::Uuid::new_v4();
        sqlx::query(
            "INSERT INTO booster_packs (id, user_id, set_name, cards) VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(pack_id.to_string())
        .bind(user_id.to_string())
        .bind(&pack.set_name)
        .bind(cards)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok((
            reward_points.max(0) as u32,
            UnopenedBoosterPack { id: pack_id, pack },
        ))
    }

    pub async fn claim_weekly_boosters(
        &self,
        user_id: uuid::Uuid,
        week_start: chrono::NaiveDate,
        packs: &[BoosterPack],
    ) -> Result<bool, UserRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let result = sqlx::query(
            "UPDATE users
             SET last_booster_week = ?1
             WHERE id = ?2 AND (last_booster_week IS NULL OR last_booster_week < ?1)",
        )
        .bind(week_start.to_string())
        .bind(user_id.to_string())
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
                "INSERT INTO booster_packs (id, user_id, set_name, cards) VALUES (?1, ?2, ?3, ?4)",
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(user_id.to_string())
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
        let packs: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT CAST(id AS TEXT), set_name, CAST(cards AS TEXT)
             FROM booster_packs
             WHERE user_id = ?1 AND opened_at IS NULL
             ORDER BY created_at",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        packs
            .into_iter()
            .map(|(id, set_name, cards)| {
                let cards = serde_json::from_str(&cards)
                    .map_err(|_| UserRepositoryError::Serialization)?;
                Ok(UnopenedBoosterPack {
                    id: id.parse().map_err(|_| UserRepositoryError::Serialization)?,
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
            "UPDATE booster_packs SET opened_at = CURRENT_TIMESTAMP
             WHERE id = ?1 AND user_id = ?2 AND opened_at IS NULL
             RETURNING set_name, CAST(cards AS TEXT)",
        )
        .bind(pack_id.to_string())
        .bind(user_id.to_string())
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
                "INSERT INTO user_cards (user_id, card_name, is_foil, quantity) VALUES (?1, ?2, ?3, 1)
                 ON CONFLICT (user_id, card_name, is_foil)
                 DO UPDATE SET quantity = user_cards.quantity + 1",
            )
            .bind(user_id.to_string())
            .bind(&card.name)
            .bind(card.is_foil)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(Some(BoosterPack { set_name, cards }))
    }
}

#[cfg(test)]
mod tests {
    use super::{BETA_BOOSTER_COST, WIN_REWARD_POINTS};

    #[test]
    fn beta_booster_costs_three_wins() {
        assert_eq!(BETA_BOOSTER_COST, WIN_REWARD_POINTS * 3);
    }
}
