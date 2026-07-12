use crate::card::{Edition, Rarity, ALL_CARDS};
use rand::seq::IndexedRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};

pub const BETA_ORDINARY_CARDS: usize = 11;
pub const BETA_EXCEPTIONAL_CARDS: usize = 3;
pub const BETA_ELITE_OR_UNIQUE_CARDS: usize = 1;
pub const BETA_FOIL_CHANCE: f64 = 0.25;

#[derive(Debug, Clone, Serialize)]
pub struct BoosterCard {
    pub name: String,
    pub is_foil: bool,
}

impl<'de> Deserialize<'de> for BoosterCard {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct CardRepr {
            name: String,
            #[serde(default)]
            is_foil: bool,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            LegacyName(String),
            Card(CardRepr),
        }

        match Repr::deserialize(deserializer)? {
            Repr::LegacyName(name) => Ok(Self { name, is_foil: false }),
            Repr::Card(CardRepr { name, is_foil }) => Ok(Self { name, is_foil }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoosterPack {
    pub set_name: String,
    pub cards: Vec<BoosterCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnopenedBoosterPack {
    pub id: uuid::Uuid,
    pub pack: BoosterPack,
}

impl BoosterPack {
    pub fn beta() -> Self {
        let mut ordinary = Vec::new();
        let mut exceptional = Vec::new();
        let mut elite_or_unique = Vec::new();

        for (_, constructor) in ALL_CARDS {
            let card = constructor(uuid::Uuid::nil());
            let base = card.get_base();
            if base.edition != Edition::Beta || base.is_token || card.is_avatar() {
                continue;
            }

            match base.rarity {
                Rarity::Ordinary => ordinary.push(card.get_name().to_string()),
                Rarity::Exceptional => exceptional.push(card.get_name().to_string()),
                Rarity::Elite | Rarity::Unique => elite_or_unique.push(card.get_name().to_string()),
            }
        }

        let mut rng = rand::rng();
        let mut cards = draw(&ordinary, BETA_ORDINARY_CARDS, &mut rng);
        cards.extend(draw(&exceptional, BETA_EXCEPTIONAL_CARDS, &mut rng));
        cards.extend(draw(
            &elite_or_unique,
            BETA_ELITE_OR_UNIQUE_CARDS,
            &mut rng,
        ));

        if rng.random_bool(BETA_FOIL_CHANCE) {
            let mut foil_pool = ordinary;
            foil_pool.extend(exceptional);
            foil_pool.extend(elite_or_unique);
            cards[0] = BoosterCard {
                name: foil_pool
                    .choose(&mut rng)
                    .expect("Beta must contain foil candidates")
                    .clone(),
                is_foil: true,
            };
        }

        Self {
            set_name: "Beta".to_string(),
            cards,
        }
    }
}

fn draw(pool: &[String], count: usize, rng: &mut impl rand::Rng) -> Vec<BoosterCard> {
    (0..count)
        .map(|_| {
            BoosterCard {
                name: pool
                    .choose(rng)
                    .expect("Beta must contain cards for every booster rarity slot")
                    .clone(),
                is_foil: false,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::from_name;

    #[test]
    fn beta_booster_uses_beta_rarity_composition() {
        let pack = BoosterPack::beta();
        assert_eq!(pack.cards.len(), 15);

        let mut ordinary = 0;
        let mut exceptional = 0;
        let mut elite_or_unique = 0;
        let foil_count = pack.cards.iter().filter(|card| card.is_foil).count();
        assert!(foil_count <= 1);
        for booster_card in pack.cards {
            let card = from_name(&booster_card.name, &uuid::Uuid::nil());
            assert_eq!(card.get_base().edition, Edition::Beta);
            if booster_card.is_foil {
                continue;
            }
            match card.get_base().rarity {
                Rarity::Ordinary => ordinary += 1,
                Rarity::Exceptional => exceptional += 1,
                Rarity::Elite | Rarity::Unique => elite_or_unique += 1,
            }
        }
        assert_eq!(ordinary, BETA_ORDINARY_CARDS - foil_count);
        assert_eq!(exceptional, BETA_EXCEPTIONAL_CARDS);
        assert_eq!(elite_or_unique, BETA_ELITE_OR_UNIQUE_CARDS);
    }

    #[test]
    fn legacy_pack_cards_are_treated_as_nonfoil() {
        let cards: Vec<BoosterCard> = serde_json::from_str(r#"["Aqueduct"]"#).unwrap();

        assert_eq!(cards[0].name, "Aqueduct");
        assert!(!cards[0].is_foil);
    }

    #[test]
    fn booster_cards_round_trip_over_messagepack() {
        let cards = vec![BoosterCard {
            name: "Aqueduct".to_string(),
            is_foil: true,
        }];
        let bytes = rmp_serde::to_vec(&cards).unwrap();
        let received: Vec<BoosterCard> = rmp_serde::from_slice(&bytes).unwrap();

        assert_eq!(received[0].name, "Aqueduct");
        assert!(received[0].is_foil);
    }
}
