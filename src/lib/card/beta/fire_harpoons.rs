use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct FireHarpoons {
    card_base: CardBase,
}

impl FireHarpoons {
    pub const NAME: &'static str = "Fire Harpoons!";
    pub const DESCRIPTION: &'static str = "Deal 1 damage to target minion above or below an adjacent Water site and pull it to the caster's location. Draw a card.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "W"),
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
impl Card for FireHarpoons {
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
impl Magic for FireHarpoons {
    async fn resolve_magic(
        &self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let caster = state.get_card(caster_id);
        let caster_location = caster.get_location().clone();
        let controller_id = caster.get_controller_id(state);

        // Find adjacent Water sites and collect the zones above/below them.
        let adjacent_water_sites: Vec<Location> = CardQuery::new()
            .water_sites()
            .adjacent_to(&caster_location)
            .all(state)
            .into_iter()
            .map(|cid| state.get_card(&cid).get_location().clone())
            .collect();

        // Find enemy minions at those zones.
        let targets = CardQuery::new()
            .minions()
            .id_not(*caster_id)
            .with_source_card(*self.get_id())
            .with_prompt("Pick a minion to fire at")
            .occupying_sites_at_locations(adjacent_water_sites);

        if targets.is_empty(state) {
            return Ok(vec![Effect::DrawCard {
                player_id: controller_id,
                count: 1,
                kind: DrawKind::Choice,
            }]);
        }

        let Some(target_id) = targets.pick(&controller_id, state).await? else {
            return Ok(vec![]);
        };
        let target = state.get_card(&target_id);

        Ok(vec![
            Effect::TakeDamage {
                card_id: target_id,
                from: *self.get_id(),
                damage: Damage::basic(1),
            },
            Effect::MoveCard {
                player_id: controller_id,
                card_id: target_id,
                from: target.get_location().clone(),
                to: caster_location.into(),
                tap: false,
                through_path: None,
            },
            Effect::DrawCard {
                player_id: controller_id,
                count: 1,
                kind: DrawKind::Choice,
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (FireHarpoons::NAME, |owner_id: PlayerId| {
    Box::new(FireHarpoons::new(owner_id))
});
