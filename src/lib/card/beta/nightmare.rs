use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone,
    },
    effect::Effect,
    game::{PlayerId, pick_zone, yes_or_no},
    query::ZoneQuery,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Nightmare {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl Nightmare {
    pub const NAME: &'static str = "Nightmare";
    pub const DESCRIPTION: &'static str = "At the end of your turn, for each enemy minion here, you may push it to an adjacent location or void.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 6,
                toughness: 6,
                types: vec![MinionType::Undead],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(7, "AA"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Nightmare {
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

    fn get_unit_base(&self) -> Option<&UnitBase> {
        Some(&self.unit_base)
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        Some(&mut self.unit_base)
    }

    async fn on_turn_end(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        if state.current_player != controller_id {
            return Ok(vec![]);
        }
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        let my_zone = self.get_zone().clone();
        let enemy_minions = CardQuery::new()
            .minions()
            .in_zone(&my_zone)
            .all(state)
            .into_iter()
            .filter(|id| state.get_card(id).get_controller_id(state) != controller_id)
            .collect::<Vec<_>>();

        let adjacent_zones = my_zone.get_adjacent();
        if adjacent_zones.is_empty() {
            return Ok(vec![]);
        }

        let mut effects = vec![];
        for minion_id in enemy_minions {
            let minion_zone = state.get_card(&minion_id).get_zone().clone();
            let push = yes_or_no(
                &controller_id,
                state,
                &format!(
                    "Nightmare: Push {} to an adjacent location?",
                    state.get_card(&minion_id).get_name()
                ),
            )
            .await?;

            if !push {
                continue;
            }

            let target_zone = pick_zone(
                &controller_id,
                &adjacent_zones,
                state,
                false,
                "Nightmare: Choose adjacent location to push enemy minion",
            )
            .await?;

            let region = state.get_card(&minion_id).get_region(state).clone();
            effects.push(Effect::MoveCard {
                player_id: controller_id,
                card_id: minion_id,
                from: minion_zone,
                to: ZoneQuery::from_zone(target_zone),
                tap: false,
                region,
                through_path: None,
            });
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Nightmare::NAME, |owner_id: PlayerId| {
    Box::new(Nightmare::new(owner_id))
});
