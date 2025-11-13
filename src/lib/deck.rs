use crate::card::{Avatar, CardType, CardZone, Site, Spell};
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
        let spells = vec![
            "Black Knight",
            "Bailey",
            "Cast Into Exile",
            "Castle Haunt",
            "Arc Lightning",
            "Castle Servants",
            "Caerleon-Upon-Usk",
            "Ball Lightning",
            "Treasures of Britain",
            "Arcane Barrage",
            "Bonfire",
            "Burning Hands",
            "Amulet of Niniane",
            "Attack by Night",
            "A Midsummer Night's Dream",
            "Black Knight",
            "Bailey",
            "Cast Into Exile",
            "Castle Haunt",
            "Arc Lightning",
            "Castle Servants",
            "Caerleon-Upon-Usk",
            "Ball Lightning",
            "Treasures of Britain",
            "Arcane Barrage",
            "Bonfire",
            "Burning Hands",
            "Amulet of Niniane",
            "Attack by Night",
            "A Midsummer Night's Dream",
            "Black Knight",
            "Bailey",
            "Cast Into Exile",
            "Castle Haunt",
            "Arc Lightning",
            "Castle Servants",
            "Caerleon-Upon-Usk",
            "Ball Lightning",
            "Treasures of Britain",
            "Arcane Barrage",
            "Bonfire",
            "Burning Hands",
            "Amulet of Niniane",
            "Attack by Night",
            "A Midsummer Night's Dream",
            "Black Knight",
            "Bailey",
            "Cast Into Exile",
            "Castle Haunt",
            "Arc Lightning",
            "Castle Servants",
            "Caerleon-Upon-Usk",
            "Ball Lightning",
            "Treasures of Britain",
            "Arcane Barrage",
            "Bonfire",
            "Burning Hands",
            "Amulet of Niniane",
            "Attack by Night",
            "A Midsummer Night's Dream",
        ];
        let spells: Vec<Spell> = spells
            .into_iter()
            .map(|s| Spell {
                id: uuid::Uuid::new_v4(),
                name: s.to_string(),
                owner_id: player_id,
                zone: CardZone::Spellbook,
                card_type: CardType::Spell,
                mana_cost: 1,
                description: None,
                tapped: false,
            })
            .collect();

        let sites = vec![
            "Beacon",
            "Bog",
            "Annual Fair",
            "Beacon",
            "Bog",
            "Annual Fair",
            "Beacon",
            "Bog",
            "Annual Fair",
            "Beacon",
            "Bog",
            "Annual Fair",
            "Beacon",
            "Bog",
            "Annual Fair",
            "Beacon",
            "Bog",
            "Annual Fair",
            "Beacon",
            "Bog",
            "Annual Fair",
            "Beacon",
            "Bog",
            "Annual Fair",
            "Beacon",
            "Bog",
            "Annual Fair",
            "Beacon",
            "Bog",
            "Annual Fair",
        ];
        let sites: Vec<Site> = sites
            .into_iter()
            .map(|s| Site {
                id: uuid::Uuid::new_v4(),
                name: s.to_string(),
                owner_id: player_id,
                zone: CardZone::Atlasbook,
            })
            .collect();

        Deck {
            id: 0,
            name: "Test Deck".to_string(),
            sites,
            spells,
            avatar: Avatar {
                id: uuid::Uuid::new_v4(),
                name: "Battlemage".to_string(),
                owner_id: player_id,
                zone: CardZone::Avatar,
            },
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
