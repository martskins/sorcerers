use crate::{
    card::{Ability, Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    game::PlayerId,
};
use rand::seq::IndexedRandom;

#[derive(Debug, Clone)]
pub struct Frog {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl Frog {
    pub const NAME: &'static str = "Frog";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 0,
                toughness: 0,
                types: vec![MinionType::Beast],
                abilities: vec![Ability::Submerge],
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

impl Card for Frog {
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
            "https://d27a44hjr9gen3.cloudfront.net/cards/got-frog-bt-s.png",
            "https://d27a44hjr9gen3.cloudfront.net/cards/bet-frog_blue-bt-s.png",
            "https://d27a44hjr9gen3.cloudfront.net/cards/bet-frog_green-bt-s.png",
            "https://d27a44hjr9gen3.cloudfront.net/cards/bet-frog_red-bt-s.png",
        ];

        match arts.choose(&mut rand::rng()) {
            Some(art) => art.to_string(),
            None => "".to_string(),
        }
    }
}
