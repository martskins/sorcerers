use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Silence {
    aura_base: AuraBase,
    card_base: CardBase,
}

impl Silence {
    pub const NAME: &'static str = "Silence";
    pub const DESCRIPTION: &'static str = "Minions occupying affected sites are silenced.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            aura_base: AuraBase { tapped: false },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "WW"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Aura for Silence {}

#[async_trait::async_trait]
impl Card for Silence {
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
    fn get_aura_base(&self) -> Option<&AuraBase> {
        Some(&self.aura_base)
    }
    fn get_aura_base_mut(&mut self) -> Option<&mut AuraBase> {
        Some(&mut self.aura_base)
    }
    fn get_aura(&self) -> Option<&dyn Aura> {
        Some(self)
    }

    async fn get_ongoing_effects(&self, _state: &State) -> anyhow::Result<Vec<OngoingEffect>> {
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }
        Ok(vec![OngoingEffect::GrantStatus {
            status: CardStatus::Silenced,
            affected_cards: CardQuery::new()
                .units()
                .in_affected_zones_of_card(self.get_id()),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Silence::NAME, |owner_id: PlayerId| {
    Box::new(Silence::new(owner_id))
});
