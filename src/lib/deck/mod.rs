pub mod precon;

use crate::{effect::Effect, game::PlayerId};

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

    pub fn peek_site(&self) -> Option<&uuid::Uuid> {
        self.sites.last()
    }

    pub fn draw_site(&mut self) -> Vec<Effect> {
        vec![Effect::DrawSite {
            player_id: self.player_id.clone(),
            count: 1,
        }]
    }

    pub fn peek_spell(&self) -> Option<&uuid::Uuid> {
        self.spells.last()
    }

    pub fn draw_spell(&mut self) -> Vec<Effect> {
        vec![Effect::DrawSpell {
            player_id: self.player_id.clone(),
            count: 1,
        }]
    }

    pub fn shuffle(&mut self) {
        use rand::rng;
        use rand::seq::SliceRandom;

        let mut rng = rng();
        self.sites.shuffle(&mut rng);
        self.spells.shuffle(&mut rng);
    }

    pub fn rotate_sites(&mut self, count: usize) {
        for _ in 0..count {
            if let Some(site) = self.sites.pop() {
                self.sites.insert(0, site);
            }
        }
    }

    pub fn rotate_spells(&mut self, count: usize) {
        for _ in 0..count {
            if let Some(spell) = self.spells.pop() {
                self.spells.insert(0, spell);
            }
        }
    }
}
