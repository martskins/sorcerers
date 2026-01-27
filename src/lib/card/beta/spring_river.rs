use crate::{
    card::{Card, CardBase, Cost, Edition, Rarity, Region, Site, SiteBase, SiteType, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds, take_action},
    state::State,
};

#[derive(Debug, Clone)]
pub struct SpringRiver {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl SpringRiver {
    pub const NAME: &'static str = "Spring River";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("W"),
                types: vec![SiteType::River],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Atlasbook,
                cost: Cost::zero(),
                region: Region::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Site for SpringRiver {}

#[async_trait::async_trait]
impl Card for SpringRiver {
    fn get_name(&self) -> &str {
        Self::NAME
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

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let deck = state.get_player_deck(&controller_id)?;
        if let Some(spell_id) = deck.peek_spell() {
            let prompt = "Viewing the top card of your spellbook";
            let action = "Put into the bottom of your spellbook?";
            let action = take_action(&controller_id, &[spell_id.clone()], state, prompt, action).await?;
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
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (SpringRiver::NAME, |owner_id: PlayerId| {
    Box::new(SpringRiver::new(owner_id))
});
