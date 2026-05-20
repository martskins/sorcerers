use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct BottomlessPit {
    site_base: SiteBase,
    card_base: CardBase,
}

impl BottomlessPit {
    pub const NAME: &'static str = "Bottomless Pit";
    pub const DESCRIPTION: &'static str =
        "Whenever a non-Airborne minion enters this site, kill it.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::new(),
                types: vec![],
                tapped: false,
                ..Default::default()
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

#[async_trait::async_trait]
impl Site for BottomlessPit {
    fn on_card_enter(&self, state: &State, card_id: &uuid::Uuid) -> Vec<Effect> {
        let card = state.get_card(card_id);
        if !card.is_minion() || card.has_ability(state, &Ability::Airborne) {
            return vec![];
        }

        vec![Effect::KillMinion {
            card_id: *card_id,
            killer_id: *self.get_id(),
            from_attack: false,
        }]
    }
}

impl ResourceProvider for BottomlessPit {}

#[async_trait::async_trait]
impl Card for BottomlessPit {
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
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (BottomlessPit::NAME, |owner_id: PlayerId| {
        Box::new(BottomlessPit::new(owner_id))
    });
