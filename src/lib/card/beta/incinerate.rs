use crate::prelude::*;

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

    fn get_magic(&self) -> Option<&dyn Magic> {
        Some(self)
    }
}

#[async_trait::async_trait]
impl Magic for Incinerate {
    async fn resolve_magic(
        &self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let caster = state.get_card(caster_id);
        let mut locations = CardQuery::new()
            .units()
            .minion_type(&MinionType::Dragon)
            .controlled_by(&self.get_controller_id(state))
            .all_map(state, |card| card.get_location().clone());
        locations.push(caster.get_location().clone());

        let prompt = "Pick a zone to deal 4 damage to all other units in that zone";
        let picked_location = pick_location_source(
            self.get_owner_id(),
            &locations,
            state,
            false,
            prompt,
            Some(*self.get_id()),
        )
        .await?;
        Ok(CardQuery::new()
            .units()
            .id_not(*self.get_id())
            .in_location(picked_location)
            .all(state)
            .into_iter()
            .map(|id| Effect::TakeDamage {
                card_id: id,
                from: *self.get_id(),
                damage: Damage::basic(4),
            })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Incinerate::NAME, |owner_id: PlayerId| {
    Box::new(Incinerate::new(owner_id))
});
