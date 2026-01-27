use macroquad::rand::ChooseRandom;

use crate::{
    card::{Ability, Card, CardBase, Cost, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    game::PlayerId,
};

#[derive(Debug, Clone)]
pub struct Frog {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
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
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Atlasbook,
                cost: Cost::zero(),
                region: Region::Surface,
                rarity: Rarity::Token,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
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
        let arts = vec![
            "https://d27a44hjr9gen3.cloudfront.net/cards/got-frog-bt-s.png",
            "https://d27a44hjr9gen3.cloudfront.net/cards/bet-frog_blue-bt-s.png",
            "https://d27a44hjr9gen3.cloudfront.net/cards/bet-frog_green-bt-s.png",
            "https://d27a44hjr9gen3.cloudfront.net/cards/bet-frog_red-bt-s.png",
        ];

        match arts.choose() {
            Some(art) => art.to_string(),
            None => "".to_string(),
        }
    }
}
