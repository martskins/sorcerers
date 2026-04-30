use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, Rarity, ResourceProvider, Site, SiteBase,
        Zone,
    },
    effect::Effect,
    game::{PlayerId, Thresholds},
    state::{CardQuery, State},
};

/// **Pillar of Zeiros** — Unique Site (Earth threshold)
///
/// Genesis → Banish all dead minions, and you heal 1 life for each.
#[derive(Debug, Clone)]
pub struct PillarOfZeiros {
    site_base: SiteBase,
    card_base: CardBase,
}

impl PillarOfZeiros {
    pub const NAME: &'static str = "Pillar of Zeiros";
    pub const DESCRIPTION: &'static str =
        "Genesis → Banish all dead minions, and you heal 1 life for each.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("E"),
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Site for PillarOfZeiros {}

#[async_trait::async_trait]
impl Card for PillarOfZeiros {
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

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let avatar_id = state.get_player_avatar_id(&controller_id)?;

        let dead_minions = CardQuery::new().minions().dead().all(state);
        let count = dead_minions.len() as u32;

        let mut effects: Vec<Effect> = dead_minions
            .into_iter()
            .map(|card_id| Effect::BanishCard { card_id })
            .collect();

        for _ in 0..count {
            effects.push(Effect::Heal {
                card_id: avatar_id,
                amount: 1,
            });
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (PillarOfZeiros::NAME, |owner_id: PlayerId| {
        Box::new(PillarOfZeiros::new(owner_id))
    });
