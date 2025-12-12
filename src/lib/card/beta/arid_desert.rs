use crate::{
    card::{Card, CardBase, CardType, Edition, MessageHandler, SiteBase, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct AridDesert {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl AridDesert {
    pub const NAME: &'static str = "Arid Desert";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("F"),
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Atlasbook,
            },
        }
    }
}

impl Card for AridDesert {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn is_tapped(&self) -> bool {
        self.card_base.tapped
    }

    fn get_owner_id(&self) -> &PlayerId {
        &self.card_base.owner_id
    }

    fn get_edition(&self) -> Edition {
        Edition::Beta
    }

    fn get_id(&self) -> uuid::Uuid {
        self.card_base.id
    }

    fn get_card_type(&self) -> CardType {
        CardType::Site
    }

    fn genesis(&mut self, state: &State) -> Vec<Effect> {
        vec![]
    }
}

impl MessageHandler for AridDesert {}
