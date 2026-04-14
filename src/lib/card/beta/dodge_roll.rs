use crate::{
    card::{Card, CardBase, Cost, Costs, Edition, Rarity, Zone},
    effect::Effect,
    game::PlayerId,
    state::State,
};

#[derive(Debug, Clone)]
pub struct DodgeRoll {
    pub card_base: CardBase,
}

impl DodgeRoll {
    pub const NAME: &'static str = "Dodge Roll";
    pub const DESCRIPTION: &'static str = "May be cast when an ally is attacked.\r \r An attacked ally may move to another adjacent location to evade the attack.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(0, "WW"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for DodgeRoll {
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

    async fn on_cast(
        &mut self,
        _state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        // Dodge Roll effect is implement on State::replace_effect, so as to ask the player wether
        // to play it only once, even if they have multiple Dodge Roll cards in hand.
        Ok(vec![])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (DodgeRoll::NAME, |owner_id: PlayerId| {
        Box::new(DodgeRoll::new(owner_id))
    });
