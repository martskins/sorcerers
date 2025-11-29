use crate::card::site::{Site, ALL_SITES};
use crate::card::spell::{Spell, ALL_SPELLS};
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
            for spell in ALL_SPELLS {
                spells.push(Spell::from_name(spell, player_id).unwrap());
            }
        }

        let mut sites = vec![];
        for _i in 0..10 {
            for site in ALL_SITES {
                sites.push(Site::from_name(site, player_id).unwrap());
            }
        }

        Deck {
            id: 0,
            name: "Test Deck".to_string(),
            sites,
            spells,
            avatar: Avatar::Sorcerer(CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id: player_id,
                zone: CardZone::Realm(3),
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
