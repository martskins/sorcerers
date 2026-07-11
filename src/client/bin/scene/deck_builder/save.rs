use sorcerers::deck::{CardNameWithCount, DeckList};

use super::*;

impl DeckBuilder {
    pub(super) fn back_to_menu(&self) -> Scene {
        Scene::Menu(crate::scene::menu::Menu::restore(
            self.client.clone(),
            self.player_id,
            self.player_name.clone(),
            self.prev_available_decks.clone(),
            self.prev_saved_decks.clone(),
            self.collection
                .iter()
                .map(|(name, &count)| CardNameWithCount {
                    name: name.clone(),
                    count,
                })
                .collect(),
        ))
    }

    pub(super) fn try_save_deck(&mut self) -> Result<Scene, String> {
        let avatar = self.selected_avatar.clone().unwrap_or_default();
        let name = self.deck_name.trim().to_string();

        let spells = self
            .deck_spells
            .iter()
            .map(|(card_name, &count)| CardNameWithCount {
                count,
                name: card_name.clone(),
            })
            .collect();
        let sites = self
            .deck_sites
            .iter()
            .map(|(card_name, &count)| CardNameWithCount {
                count,
                name: card_name.clone(),
            })
            .collect();

        let deck_list = DeckList {
            name,
            avatar,
            spells,
            sites,
        };

        deck_list.validate()?;
        deck_list
            .save()
            .map_err(|e| format!("Failed to save: {e}"))?;

        Ok(self.back_to_menu())
    }
}
