use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct LeapAttack {
    card_base: CardBase,
}

impl LeapAttack {
    pub const NAME: &'static str = "Leap Attack";
    pub const DESCRIPTION: &'static str =
        "An ally may take a step, and then it strikes each enemy at its location.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "F"),
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
impl Card for LeapAttack {
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
impl Magic for LeapAttack {
    async fn resolve_magic(
        &self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);

        let Some(leaper_id) = CardQuery::new()
            .minions()
            .controlled_by(&controller_id)
            .in_play()
            .with_prompt("Pick an ally to leap")
            .with_source_card(*self.get_id())
            .pick(&controller_id, state)
            .await?
        else {
            return Ok(vec![]);
        };

        let leaper = state.get_card(&leaper_id);
        let current_location = leaper.get_location().clone();
        let mut one_step_zones = leaper.get_locations_within_steps(state, 1);
        if !one_step_zones.contains(&current_location) {
            one_step_zones.push(current_location.clone());
        }

        let dest_zone = LocationQuery::from_locations(one_step_zones)
            .with_prompt("Pick a zone to leap to, or pick the current zone to stay put")
            .with_source_card(*self.get_id())
            .pick(&controller_id, state)
            .await?;

        let enemies_at_dest: Vec<CardId> = CardQuery::new()
            .minions()
            .in_zone(&dest_zone)
            .all(state)
            .into_iter()
            .filter(|id| state.get_card(id).get_controller_id(state) != controller_id)
            .collect();

        let mut effects = vec![];
        if dest_zone != current_location {
            effects.push(Effect::MoveCard {
                player_id: controller_id,
                card_id: leaper_id,
                from: current_location,
                to: dest_zone.into(),
                tap: false,
                through_path: None,
            });
        }

        for enemy_id in enemies_at_dest {
            effects.push(Effect::strike(state, leaper_id, enemy_id)?);
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (LeapAttack::NAME, |owner_id: PlayerId| {
    Box::new(LeapAttack::new(owner_id))
});
