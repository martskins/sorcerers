use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Dispel {
    card_base: CardBase,
}

impl Dispel {
    pub const NAME: &'static str = "Dispel";
    pub const DESCRIPTION: &'static str =
        "Destroy all auras and artifacts at target location up to two steps away.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "E"),
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
impl Card for Dispel {
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

    fn get_magic(&self) -> Option<&dyn Magic> {
        Some(self)
    }
}

#[async_trait::async_trait]
impl Magic for Dispel {
    async fn resolve_magic(
        &self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let caster = state.get_card(caster_id);
        let locs_within_two_steps = caster.get_locations_within_steps(state, 2);
        let target_zone = LocationQuery::from_locations(locs_within_two_steps)
            .with_source_card(*self.get_id())
            .with_prompt("Pick a zone to disenchant")
            .pick(&controller_id, state)
            .await?;

        let artifacts_and_auras = CardQuery::new()
            .card_types(vec![CardType::Aura, CardType::Artifact])
            .in_zone(&target_zone)
            .all(state);

        // TODO: Do we need a destroy effect for non-minion cards? Or even a generic one that does
        // what KillMinion does?
        Ok(artifacts_and_auras
            .into_iter()
            .map(|id| Effect::BuryCard { card_id: id })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Dispel::NAME, |owner_id: PlayerId| {
    Box::new(Dispel::new(owner_id))
});
