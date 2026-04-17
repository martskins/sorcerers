use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::Effect,
    game::{PlayerId, get_knight_move_zones, pick_card, pick_zone},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct FlankingManeuver {
    card_base: CardBase,
}

impl FlankingManeuver {
    pub const NAME: &'static str = "Flanking Maneuver";
    pub const DESCRIPTION: &'static str = "Teleport any number of allies at one location to another location a chess knight's move away. Draw a card.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "A"),
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
impl Card for FlankingManeuver {
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
        let controller = caster.get_controller_id(state);

        // All realm zones that contain ally minions.
        let source_zones: Vec<Zone> = CardQuery::new()
            .minions()
            .in_play()
            .controlled_by(&controller)
            .all(state)
            .into_iter()
            .map(|cid| state.get_card(&cid).get_zone().clone())
            .collect();

        if source_zones.is_empty() {
            return Ok(vec![Effect::DrawCard {
                player_id: controller,
                count: 1,
            }]);
        }

        let source_zone = pick_zone(
            &controller,
            &source_zones,
            state,
            false,
            "Flanking Maneuver: Pick a site to teleport allies from",
        )
        .await?;

        let ally_minions_at_source: Vec<uuid::Uuid> = CardQuery::new()
            .units()
            .in_zone(&source_zone)
            .controlled_by(&controller)
            .all(state);

        // Build valid destination zones: knight's move from source that are either empty realm
        // squares (void = no site) or have an allied site.
        let destinations: Vec<Zone> = get_knight_move_zones(&source_zone)
            .into_iter()
            .filter(|z| {
                let cards_here = state.get_cards_in_zone(z);
                // Only units/artifacts count as "occupied" — a zone with only a site is fine.
                cards_here.iter().all(|c| c.is_site())
            })
            .collect();

        if destinations.is_empty() {
            return Ok(vec![Effect::DrawCard {
                player_id: controller,
                count: 1,
            }]);
        }

        let dest_zone = pick_zone(
            &controller,
            &destinations,
            state,
            false,
            "Flanking Maneuver: Pick a knight's move destination",
        )
        .await?;

        // Ask how many minions to move.
        let count = crate::game::pick_amount(
            &controller,
            0,
            ally_minions_at_source.len() as u8,
            state,
            "Flanking Maneuver: How many allies to teleport?",
        )
        .await?;

        let mut effects: Vec<Effect> = vec![];
        let mut remaining = ally_minions_at_source.clone();
        for _ in 0..count {
            if remaining.is_empty() {
                break;
            }
            let picked =
                pick_card(&controller, &remaining, state, "Pick an ally to teleport").await?;
            remaining.retain(|id| id != &picked);
            effects.push(Effect::TeleportCard {
                player_id: controller,
                card_id: picked,
                to_zone: dest_zone.clone(),
            });
        }

        effects.push(Effect::DrawCard {
            player_id: controller,
            count: 1,
        });
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (FlankingManeuver::NAME, |owner_id: PlayerId| {
        Box::new(FlankingManeuver::new(owner_id))
    });
