use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct SpringRiver {
    site_base: SiteBase,
    card_base: CardBase,
}

impl SpringRiver {
    pub const NAME: &'static str = "Spring River";
    pub const DESCRIPTION: &'static str =
        "Genesis → Look at your next spell. You may put it on the bottom of your spellbook.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("W"),
                types: vec![SiteType::River],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Site for SpringRiver {}

impl ResourceProvider for SpringRiver {}

#[async_trait::async_trait]
impl Card for SpringRiver {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_description(&self) -> &str {
        Self::DESCRIPTION
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn get_site_base(&self) -> Option<&SiteBase> {
        Some(&self.site_base)
    }

    fn get_site_base_mut(&mut self) -> Option<&mut SiteBase> {
        Some(&mut self.site_base)
    }

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook::genesis(self.get_id())])
    }

    async fn resolve_hook(
        &self,
        hook: HookId,
        state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            GENESIS_HOOK_ID => {
                let controller_id = self.get_controller_id(state);
                let deck = state.get_player_deck(&controller_id)?;
                if let Some(spell_id) = deck.peek_spell() {
                    let prompt = "Viewing the top card of your spellbook";
                    let action = "Put into the bottom of your spellbook?";
                    let action = take_action(
                        &controller_id,
                        &[*spell_id],
                        state,
                        prompt,
                        action,
                        *self.get_id(),
                    )
                    .await?;
                    if action {
                        let mut deck = deck.clone();
                        deck.rotate_spells(1);
                        return Ok(vec![Effect::RearrangeDeck {
                            spells: deck.spells,
                            sites: deck.sites,
                        }]);
                    }
                }

                Ok(vec![])
            }
            _ => Ok(vec![]),
        }
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (SpringRiver::NAME, |owner_id: PlayerId| {
    Box::new(SpringRiver::new(owner_id))
});
