use crate::{
    card::{Card, CardBase, Cost, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::PlayerId,
    query::ZoneQuery,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Blink {
    pub card_base: CardBase,
}

impl Blink {
    pub const NAME: &'static str = "Blink";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "A"),
                region: Region::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Blink {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    async fn on_cast(
        &mut self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let caster = state.get_card(caster_id);
        let controller_id = caster.get_controller_id(state);
        let card_id = CardQuery::new()
            .units()
            .controlled_by(&controller_id)
            .with_prompt("Blink: Pick an ally to teleport")
            .id_not_in(vec![caster_id.clone()])
            .pick(&controller_id, state, false)
            .await?;
        let card_id = card_id.expect("value not to be None");
        let card = state.get_card(&card_id);
        let zone = ZoneQuery::new()
            .near(card.get_zone())
            .with_prompt("Blink: Pick a zone to teleport to")
            .pick(&controller_id, state)
            .await?;

        Ok(vec![
            Effect::TeleportCard {
                player_id: controller_id.clone(),
                card_id,
                to_zone: zone,
            },
            Effect::DrawCard {
                player_id: self.get_controller_id(state).clone(),
                count: 1,
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Blink::NAME, |owner_id: PlayerId| Box::new(Blink::new(owner_id)));
