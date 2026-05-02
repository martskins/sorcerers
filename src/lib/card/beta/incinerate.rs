use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, MinionType, Rarity, Zone},
    effect::Effect,
    game::{PlayerId, pick_zone},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Incinerate {
    card_base: CardBase,
}

impl Incinerate {
    pub const NAME: &'static str = "Incinerate";
    pub const DESCRIPTION: &'static str =
        "Deal 4 damage to each other unit at target location near the caster or an allied Dragon.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "F"),
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
impl Card for Incinerate {
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
        let caster = state.get_card(caster_id);
        let mut zones: Vec<Zone> = state
            .cards
            .iter()
            .filter(|c| c.get_zone().is_in_play())
            .filter(|c| c.get_owner_id() == self.get_owner_id())
            .filter(|c| c.is_unit())
            .filter(|c| {
                c.get_unit_base()
                    .unwrap()
                    .types
                    .contains(&MinionType::Dragon)
            })
            .flat_map(|c| c.get_zone().get_nearby())
            .collect();
        zones.push(caster.get_zone().clone());

        let prompt = "Incinerate: Pick a zone to deal 4 damage to all other units in that zone";
        let picked_zone = pick_zone(self.get_owner_id(), &zones, state, false, prompt).await?;
        Ok(CardQuery::new()
            .units()
            .id_not(self.get_id())
            .in_zone(&picked_zone)
            .all(state)
            .into_iter()
            .map(|id| Effect::TakeDamage {
                card_id: id,
                from: *self.get_id(),
                damage: 4,
                is_strike: false,
                is_ranged: false,
            })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Incinerate::NAME, |owner_id: PlayerId| {
    Box::new(Incinerate::new(owner_id))
});
