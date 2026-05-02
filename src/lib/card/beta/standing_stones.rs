use crate::{
    card::{
        Ability, AreaModifiers, Card, CardBase, CardConstructor, Costs, Edition, Rarity, Site,
        SiteBase, Zone,
    },
    game::{PlayerId, Thresholds},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct StandingStones {
    site_base: SiteBase,
    card_base: CardBase,
}

impl StandingStones {
    pub const NAME: &'static str = "Standing Stones";
    pub const DESCRIPTION: &'static str = "Minions here are Spellcasters.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 0,
                provided_thresholds: Thresholds::ZERO,
                types: vec![],
                tapped: false,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Site for StandingStones {}

#[async_trait::async_trait]
impl Card for StandingStones {
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

    fn area_modifiers(&self, state: &State) -> AreaModifiers {
        let minions = CardQuery::new()
            .minions()
            .in_zone(self.get_zone())
            .all(state);

        AreaModifiers {
            grants_abilities: minions
                .into_iter()
                .map(|id| (id, vec![Ability::Spellcaster(None)]))
                .collect(),
            ..Default::default()
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (StandingStones::NAME, |owner_id: PlayerId| {
        Box::new(StandingStones::new(owner_id))
    });
