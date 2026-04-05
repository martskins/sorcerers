pub mod precon;

use crate::{card::Card, effect::Effect, game::PlayerId};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeckList {
    pub name: String,
    pub sites: Vec<String>,
    pub spells: Vec<String>,
    pub avatar: String,
}

impl DeckList {
    pub fn save(&self) -> anyhow::Result<()> {
        let filepath = format!("decks/{}.json", self.name);
        std::fs::create_dir_all("decks")?;
        let file = std::fs::File::create(filepath)?;
        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Deck {
    pub name: String,
    pub player_id: uuid::Uuid,
    pub sites: Vec<uuid::Uuid>,
    pub spells: Vec<uuid::Uuid>,
    pub avatar: uuid::Uuid,
}

impl Deck {
    pub fn new(
        player_id: &PlayerId,
        name: String,
        sites: Vec<uuid::Uuid>,
        spells: Vec<uuid::Uuid>,
        avatar: uuid::Uuid,
    ) -> Self {
        Deck {
            name,
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

    pub fn from_file(filepath: &str, player_id: &PlayerId) -> anyhow::Result<(Deck, Vec<Box<dyn Card>>)> {
        use crate::card::from_name;

        let file = std::fs::File::open(filepath)?;
        let decklist: DeckList = serde_json::from_reader(file)?;
        let avatar_card = from_name(&decklist.avatar, player_id);
        let spell_cards: Vec<Box<dyn Card>> = decklist.spells.iter().map(|name| from_name(name, player_id)).collect();
        let site_cards: Vec<Box<dyn Card>> = decklist.sites.iter().map(|name| from_name(name, player_id)).collect();

        let mut deck = Deck {
            name: decklist.name,
            player_id: player_id.clone(),
            spells: spell_cards.iter().map(|c| c.get_id().clone()).collect(),
            sites: site_cards.iter().map(|c| c.get_id().clone()).collect(),
            avatar: avatar_card.get_id().clone(),
        };
        deck.shuffle();

        let all_cards = std::iter::once(avatar_card)
            .chain(spell_cards)
            .chain(site_cards)
            .collect();
        Ok((deck, all_cards))
    }
}
