use crate::{
    card::{Card, CardBase, Cost, Edition, MinionType, Plane, Rarity, Zone},
    effect::Effect,
    game::{PlayerId, pick_zone},
    state::State,
};

#[derive(Debug, Clone)]
pub struct Incinerate {
    pub card_base: CardBase,
}

impl Incinerate {
    pub const NAME: &'static str = "Incinerate";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(2, "F"),
                plane: Plane::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Incinerate {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    async fn on_cast(&mut self, state: &State, caster_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        let caster = state.get_card(caster_id);
        let mut zones: Vec<Zone> = state
            .cards
            .iter()
            .filter(|c| c.get_zone().is_in_play())
            .filter(|c| c.get_owner_id() == self.get_owner_id())
            .filter(|c| c.is_unit())
            .filter(|c| c.get_unit_base().unwrap().types.contains(&MinionType::Dragon))
            .flat_map(|c| c.get_zone().get_nearby())
            .collect();
        zones.push(caster.get_zone().clone());

        let prompt = "Incinerate: Pick a zone to deal 4 damage to all other units in that zone";
        let picked_zone = pick_zone(self.get_owner_id(), &zones, state, prompt).await?;
        let units = state.get_units_in_zone(&picked_zone);
        let mut effects = vec![];
        for unit in units {
            if unit.get_id() == self.get_id() {
                continue;
            }

            effects.push(Effect::take_damage(unit.get_id(), self.get_id(), 4));
        }
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (Incinerate::NAME, |owner_id: PlayerId| {
    Box::new(Incinerate::new(owner_id))
});
