use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::Effect,
    game::PlayerId,
    query::ZoneQuery,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Telekinesis {
    card_base: CardBase,
}

impl Telekinesis {
    pub const NAME: &'static str = "Telekinesis";
    pub const DESCRIPTION: &'static str =
        "Move a nearby artifact to your caster's location.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(1, "A"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Telekinesis {
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
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let caster = state.get_card(caster_id);
        let caster_zone = caster.get_zone().clone();

        let Some(artifact_id) = CardQuery::new()
            .artifacts()
            .near_to(&caster_zone)
            .with_prompt("Telekinesis: Pick a nearby artifact to move")
            .pick(&controller_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };

        let artifact = state.get_card(&artifact_id);
        let artifact_zone = artifact.get_zone().clone();

        Ok(vec![Effect::MoveCard {
            player_id: controller_id,
            card_id: artifact_id,
            from: artifact_zone,
            to: ZoneQuery::from_zone(caster_zone),
            tap: false,
            region: crate::card::Region::Surface,
            through_path: None,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (Telekinesis::NAME, |owner_id: PlayerId| {
        Box::new(Telekinesis::new(owner_id))
    });
