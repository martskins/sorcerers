use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct IceLance {
    card_base: CardBase,
}

impl IceLance {
    pub const NAME: &'static str = "Ice Lance";
    pub const DESCRIPTION: &'static str = "Shoot a piercing projectile. Deal 3, then 2, then 1 damage to up to one hit unit at each of the first three locations along its path.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "W"),
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
impl Card for IceLance {
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
impl Magic for IceLance {
    async fn resolve_magic(
        &self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let prompt = "Pick a direction to shoot the lance";
        let direction = pick_direction_source(
            controller_id,
            &CARDINAL_DIRECTIONS,
            state,
            prompt,
            Some(*caster_id),
        )
        .await?;
        let caster = state.get_card(caster_id);
        let location = caster.get_location();

        let location_dmg = vec![
            (Some(location.clone()), 3),
            (location.steps_in_direction(&direction, 1, state, Some(caster_id)), 2),
            (location.steps_in_direction(&direction, 2, state, Some(caster_id)), 1),
        ];

        let mut effects = vec![];
        for (location, dmg) in location_dmg {
            if let Some(location) = location {
                let qry = CardQuery::new()
                    .in_location(location)
                    .units()
                    .id_not_in(vec![*caster_id])
                    .with_prompt("Pick a unit to damage")
                    .with_source_card(*self.get_id());

                if let Some(card_id) = qry.pick(&controller_id, state, false).await? {
                    effects.push(Effect::TakeDamage {
                        card_id,
                        from: *self.get_id(),
                        damage: Damage::basic(dmg),
                    });
                }
            }
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (IceLance::NAME, |owner_id: PlayerId| {
    Box::new(IceLance::new(owner_id))
});
