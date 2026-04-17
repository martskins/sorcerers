use crate::{
    card::{Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    game::PlayerId,
};
use rand::seq::IndexedRandom;

#[derive(Debug, Clone)]
pub struct FootSoldier {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl FootSoldier {
    pub const NAME: &'static str = "Foot Soldier";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: true,
                ..Default::default()
            },
        }
    }
}

impl Card for FootSoldier {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn get_unit_base(&self) -> Option<&UnitBase> {
        Some(&self.unit_base)
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        Some(&mut self.unit_base)
    }

    fn get_image_path(&self) -> String {
        let arts = [
            "https://d27a44hjr9gen3.cloudfront.net/cards/pro-foot_soldier_english-d-s.png",
            "https://d27a44hjr9gen3.cloudfront.net/cards/pro-foot_soldier_saracen-d-s.png",
            "https://d27a44hjr9gen3.cloudfront.net/cards/bet-foot_soldier_1-bt-s.png",
            "https://d27a44hjr9gen3.cloudfront.net/cards/bet-foot_soldier_2-bt-s.png",
            "https://d27a44hjr9gen3.cloudfront.net/cards/bet-foot_soldier_3-bt-s.png",
            "https://d27a44hjr9gen3.cloudfront.net/cards/art-foot_soldiers-bt-s.png",
        ];
        match arts.choose(&mut rand::rng()) {
            Some(art) => art.to_string(),
            None => "".to_string(),
        }
    }
}
