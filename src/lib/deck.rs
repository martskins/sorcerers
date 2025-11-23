use crate::card::site::Site;
use crate::card::spell::Spell;
use crate::card::CardBase;
use crate::card::{avatar::Avatar, CardZone};
use rand::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Deck {
    pub id: u32,
    pub name: String,
    pub sites: Vec<Site>,
    pub spells: Vec<Spell>,
    pub avatar: Avatar,
}

impl Deck {
    pub fn test_deck(player_id: uuid::Uuid) -> Self {
        let mut spells = vec![];
        for _i in 0..10 {
            spells.push(Spell::BurningHands(CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id: player_id,
                zone: CardZone::Spellbook,
                tapped: false,
            }));
            spells.push(Spell::BallLightning(CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id: player_id,
                zone: CardZone::Spellbook,
                tapped: false,
            }));
        }

        let mut sites = vec![];
        for _i in 0..10 {
            sites.push(Site::Beacon(CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id: player_id,
                zone: CardZone::Spellbook,
                tapped: false,
            }));
            sites.push(Site::Bog(CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id: player_id,
                zone: CardZone::Spellbook,
                tapped: false,
            }));
            sites.push(Site::AnnualFair(CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id: player_id,
                zone: CardZone::Spellbook,
                tapped: false,
            }));
        }

        Deck {
            id: 0,
            name: "Test Deck".to_string(),
            sites,
            spells,
            avatar: Avatar::Sorcerer(CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id: player_id,
                zone: CardZone::Avatar,
                tapped: false,
            }),
        }
    }

    pub fn draw_site(&mut self) -> Option<Site> {
        self.sites.pop()
    }

    pub fn draw_spell(&mut self) -> Option<Spell> {
        self.spells.pop()
    }

    pub fn shuffle(&mut self) {
        let mut rng = rand::rng();
        self.sites.shuffle(&mut rng);
        self.spells.shuffle(&mut rng);
    }
}
