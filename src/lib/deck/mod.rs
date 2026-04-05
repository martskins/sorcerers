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

    /// Load all deck lists from the `decks/` directory.
    pub fn load_all() -> Vec<DeckList> {
        let Ok(entries) = std::fs::read_dir("decks") else {
            return vec![];
        };
        entries
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                if path.extension()?.to_str()? != "json" {
                    return None;
                }
                let file = std::fs::File::open(&path).ok()?;
                serde_json::from_reader(file).ok()
            })
            .collect()
    }

    /// Validate the deck list against official Sorcery: Contested Realm constructed rules.
    /// Rules: 1 avatar, ≥60 spellbook cards, ≥30 atlas sites,
    /// and copy limits: Ordinary ≤4, Exceptional ≤3, Elite ≤2, Unique ≤1.
    pub fn validate(&self) -> Result<(), String> {
        use crate::card::{Rarity, card_exists, from_name};
        use std::collections::HashMap;

        if self.name.is_empty() {
            return Err("Deck name cannot be empty.".to_string());
        }
        if self.avatar.is_empty() {
            return Err("Please select an avatar.".to_string());
        }
        if !card_exists(&self.avatar) {
            return Err(format!("Unknown avatar: \"{}\".", self.avatar));
        }

        // Spellbook size
        if self.spells.len() < 60 {
            return Err(format!(
                "Spellbook needs at least 60 cards (you have {}).",
                self.spells.len()
            ));
        }
        // Atlas size
        if self.sites.len() < 30 {
            return Err(format!(
                "Atlas needs at least 30 sites (you have {}).",
                self.sites.len()
            ));
        }

        let dummy_id = uuid::Uuid::nil();

        // Validate spellbook cards and copy limits
        let mut spell_counts: HashMap<&str, usize> = HashMap::new();
        for name in &self.spells {
            if !card_exists(name) {
                return Err(format!("Unknown card: \"{name}\"."));
            }
            *spell_counts.entry(name.as_str()).or_insert(0) += 1;
        }
        for (name, &count) in &spell_counts {
            let card = from_name(name, &dummy_id);
            let limit = match card.get_base().rarity {
                Rarity::Ordinary => 4,
                Rarity::Exceptional => 3,
                Rarity::Elite => 2,
                Rarity::Unique => 1,
            };
            if count > limit {
                return Err(format!(
                    "Too many copies of \"{name}\" ({count} — max {limit} for {:?}).",
                    card.get_base().rarity
                ));
            }
        }

        // Validate atlas sites and copy limits
        let mut site_counts: HashMap<&str, usize> = HashMap::new();
        for name in &self.sites {
            if !card_exists(name) {
                return Err(format!("Unknown site: \"{name}\"."));
            }
            *site_counts.entry(name.as_str()).or_insert(0) += 1;
        }
        for (name, &count) in &site_counts {
            let card = from_name(name, &dummy_id);
            let limit = match card.get_base().rarity {
                Rarity::Ordinary => 4,
                Rarity::Exceptional => 3,
                Rarity::Elite => 2,
                Rarity::Unique => 1,
            };
            if count > limit {
                return Err(format!(
                    "Too many copies of site \"{name}\" ({count} — max {limit} for {:?}).",
                    card.get_base().rarity
                ));
            }
        }

        Ok(())
    }

    /// Build a Deck and card list from this DeckList.
    pub fn build(&self, player_id: &PlayerId) -> (Deck, Vec<Box<dyn Card>>) {
        use crate::card::from_name;
        let avatar_card = from_name(&self.avatar, player_id);
        let spell_cards: Vec<Box<dyn Card>> = self.spells.iter().map(|n| from_name(n, player_id)).collect();
        let site_cards: Vec<Box<dyn Card>> = self.sites.iter().map(|n| from_name(n, player_id)).collect();
        let mut deck = Deck::new(
            player_id,
            self.name.clone(),
            site_cards.iter().map(|c| c.get_id().clone()).collect(),
            spell_cards.iter().map(|c| c.get_id().clone()).collect(),
            avatar_card.get_id().clone(),
        );
        deck.shuffle();
        let all_cards = std::iter::once(avatar_card).chain(spell_cards).chain(site_cards).collect();
        (deck, all_cards)
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
