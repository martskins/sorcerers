use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::Effect,
    game::{PlayerId, pick_card, pick_zone},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct LeapAttack {
    card_base: CardBase,
}

impl LeapAttack {
    pub const NAME: &'static str = "Leap Attack";
    pub const DESCRIPTION: &'static str = "An ally takes a step, then strikes each enemy at that location.";

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

    async fn on_cast(
        &mut self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);

        let allies: Vec<uuid::Uuid> = CardQuery::new()
            .minions()
            .controlled_by(&controller_id)
            .in_play()
            .all(state);

        if allies.is_empty() {
            return Ok(vec![]);
        }

        let leaper_id = pick_card(
            &controller_id,
            &allies,
            state,
            "Leap Attack: Pick an ally to leap",
        )
        .await?;

        let leaper = state.get_card(&leaper_id);
        let one_step_zones = leaper.get_zones_within_steps(state, 1);
        if one_step_zones.is_empty() {
            return Ok(vec![]);
        }

        let dest_zone = pick_zone(
            &controller_id,
            &one_step_zones,
            state,
            false,
            "Leap Attack: Pick a zone to leap to",
        )
        .await?;

        let enemies_at_dest: Vec<uuid::Uuid> = CardQuery::new()
            .minions()
            .in_zone(&dest_zone)
            .all(state)
            .into_iter()
            .filter(|id| state.get_card(id).get_controller_id(state) != controller_id)
            .collect();

        let mut effects = vec![Effect::MoveCard {
            player_id: controller_id,
            card_id: leaper_id,
            from: leaper.get_zone().clone(),
            to: dest_zone.clone().into(),
            tap: false,
            region: leaper.get_region(state).clone(),
            through_path: None,
        }];

        for enemy_id in enemies_at_dest {
            effects.push(Effect::Strike {
                striker_id: leaper_id,
                target_id: enemy_id,
            });
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (LeapAttack::NAME, |owner_id: PlayerId| {
        Box::new(LeapAttack::new(owner_id))
    });
