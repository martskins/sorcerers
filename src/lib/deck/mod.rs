pub mod precon;

use crate::{card::Zone, effect::Effect};

#[derive(Debug)]
pub struct Deck {
    pub sites: Vec<uuid::Uuid>,
    pub spells: Vec<uuid::Uuid>,
    pub avatar: uuid::Uuid,
}

impl Deck {
    pub fn new(sites: Vec<uuid::Uuid>, spells: Vec<uuid::Uuid>, avatar: uuid::Uuid) -> Self {
        Deck { sites, spells, avatar }
    }

    pub fn draw_site(&mut self) -> Vec<Effect> {
        let card_id = self.sites.pop();
        vec![Effect::MoveCard {
            card_id: card_id.unwrap(),
            to: Zone::Hand,
        }]
    }

    pub fn draw_spell(&mut self) -> Vec<Effect> {
        let card_id = self.spells.pop();
        vec![Effect::MoveCard {
            card_id: card_id.unwrap(),
            to: Zone::Hand,
        }]
    }
}
