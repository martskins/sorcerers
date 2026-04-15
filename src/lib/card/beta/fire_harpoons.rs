use crate::{
    card::{Card, CardBase, Cost, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{PlayerId, pick_card},
    query::ZoneQuery,
    state::{CardQuery, State},
};

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
                controller_id: owner_id.clone(),
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

    async fn on_cast(
        &mut self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let caster = state.get_card(caster_id);
        let caster_zone = caster.get_zone().clone();
        let controller_id = caster.get_controller_id(state);

        // Find adjacent Water sites and collect the zones above/below them.
        let adjacent_water_sites: Vec<Zone> = CardQuery::new()
            .water_sites()
            .adjacent_to(&caster_zone)
            .all(state)
            .into_iter()
            .map(|cid| state.get_card(&cid).get_zone().clone())
            .collect();

        // Find enemy minions at those zones.
        let targets = CardQuery::new()
            .minions()
            .in_regions(vec![
                Region::Underwater,
                Region::Surface,
                Region::Underground,
            ])
            .id_not_in(vec![caster_id.clone()])
            .in_zones(&adjacent_water_sites)
            .all(state);

        if targets.is_empty() {
            return Ok(vec![Effect::DrawCard {
                player_id: controller_id,
                count: 1,
            }]);
        }

        let target_id = pick_card(
            &controller_id,
            &targets,
            state,
            "Fire Harpoons!: Pick a target minion",
        )
        .await?;
        let target = state.get_card(&target_id);

        Ok(vec![
            Effect::TakeDamage {
                card_id: target_id.clone(),
                from: caster_id.clone(),
                damage: 1,
                is_strike: false,
            },
            Effect::MoveCard {
                player_id: controller_id.clone(),
                card_id: target_id,
                from: target.get_zone().clone(),
                to: ZoneQuery::from_zone(caster_zone),
                tap: false,
                region: target.get_region(state).clone(),
                through_path: None,
            },
            Effect::DrawCard {
                player_id: controller_id,
                count: 1,
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (FireHarpoons::NAME, |owner_id: PlayerId| {
        Box::new(FireHarpoons::new(owner_id))
    });
