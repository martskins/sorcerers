pub mod precon;

use crate::{
    card::{Plane, Zone},
    effect::Effect,
    game::PlayerId,
    query::ZoneQuery,
};

#[derive(Debug, Clone)]
pub struct Deck {
    pub player_id: uuid::Uuid,
    pub sites: Vec<uuid::Uuid>,
    pub spells: Vec<uuid::Uuid>,
    pub avatar: uuid::Uuid,
}

impl Deck {
    pub fn new(player_id: &PlayerId, sites: Vec<uuid::Uuid>, spells: Vec<uuid::Uuid>, avatar: uuid::Uuid) -> Self {
        Deck {
            player_id: player_id.clone(),
            sites,
            spells,
            avatar,
        }
    }

    pub fn draw_site(&mut self) -> Vec<Effect> {
        let card_id = self.sites.pop();
        vec![Effect::MoveCard {
            player_id: self.player_id.clone(),
            card_id: card_id.unwrap(),
            from: Zone::Atlasbook,
            to: ZoneQuery::Specific {
                id: uuid::Uuid::new_v4(),
                zone: Zone::Hand,
            },
            tap: false,
            plane: Plane::None,
            through_path: None,
        }]
    }

    pub fn draw_spell(&mut self) -> Vec<Effect> {
        let card_id = self.spells.pop();
        vec![Effect::MoveCard {
            player_id: self.player_id.clone(),
            card_id: card_id.unwrap(),
            from: Zone::Spellbook,
            to: ZoneQuery::Specific {
                id: uuid::Uuid::new_v4(),
                zone: Zone::Hand,
            },
            tap: false,
            plane: Plane::None,
            through_path: None,
        }]
    }

    pub fn shuffle(&mut self) {
        use rand::rng;
        use rand::seq::SliceRandom;

        let mut rng = rng();
        self.sites.shuffle(&mut rng);
        self.spells.shuffle(&mut rng);
    }
}
