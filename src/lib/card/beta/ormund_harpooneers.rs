use crate::{
    card::{
        AdditionalCost, Card, CardBase, CardConstructor, Cost, Costs, Edition, MinionType, Rarity,
        Region, UnitBase, Zone,
    },
    effect::Effect,
    game::{ActivatedAbility, PlayerId},
    query::ZoneQuery,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
struct HarpoonPull;

#[async_trait::async_trait]
impl ActivatedAbility for HarpoonPull {
    fn get_name(&self) -> String {
        "Harpoon Pull".to_string()
    }

    fn get_cost(&self, card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::tap(card_id)))
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let harpooneers = state.get_card(card_id);
        let my_zone = harpooneers.get_zone().clone();

        // Find minions in adjacent water sites (surface or underwater)
        let adjacent_water_zones: Vec<Zone> = my_zone
            .get_adjacent()
            .into_iter()
            .filter(|z| {
                z.get_site(state)
                    .and_then(|s| s.is_water_site(state).ok())
                    .unwrap_or(false)
            })
            .collect();

        let Some(target_id) = CardQuery::new()
            .minions()
            .in_zones(&adjacent_water_zones)
            .in_regions(vec![Region::Surface, Region::Underwater])
            .with_prompt("Ormund Harpooneers: Pick a minion to harpoon")
            .pick(player_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };

        let target = state.get_card(&target_id);
        let target_zone = target.get_zone().clone();
        let target_region = target.get_region(state).clone();

        Ok(vec![
            Effect::TakeDamage {
                card_id: target_id,
                from: *card_id,
                damage: 1,
                is_strike: false,
                is_ranged: false,
            },
            Effect::MoveCard {
                player_id: *player_id,
                card_id: target_id,
                from: target_zone,
                to: ZoneQuery::from_zone(my_zone),
                tap: false,
                region: target_region,
                through_path: None,
            },
        ])
    }
}

#[derive(Debug, Clone)]
pub struct OrmundHarpooneers {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl OrmundHarpooneers {
    pub const NAME: &'static str = "Ormund Harpooneers";
    pub const DESCRIPTION: &'static str = "Tap → Deal 1 damage to target minion above or below an adjacent water site and pull that minion to this location.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "W"),
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
impl Card for OrmundHarpooneers {
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

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(HarpoonPull)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (OrmundHarpooneers::NAME, |owner_id: PlayerId| {
        Box::new(OrmundHarpooneers::new(owner_id))
    });
