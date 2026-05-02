use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    effect::Effect,
    game::{PlayerId, pick_zone},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct UltimateHorror {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl UltimateHorror {
    pub const NAME: &'static str = "Ultimate Horror";
    pub const DESCRIPTION: &'static str = "Airborne, Voidwalk Genesis → Summon each other dead Voidwalk minion to a nearby site or void.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 6,
                toughness: 6,
                abilities: vec![Ability::Airborne, Ability::Voidwalk],
                types: vec![MinionType::Spirit],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(8, "AA"),
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for UltimateHorror {
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

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let nearby_zones = self.get_zone().get_nearby();
        let dead_voidwalkers = CardQuery::new()
            .minions()
            .dead()
            .with_ability(&Ability::Voidwalk)
            .all(state);
        let mut effects = Vec::new();
        for card_id in dead_voidwalkers {
            let zone = pick_zone(
                &controller_id,
                &nearby_zones,
                state,
                false,
                &format!(
                    "Ultimate Horror: Pick a nearby site or void for {}",
                    state.get_card(&card_id).get_name()
                ),
            )
            .await?;
            effects.push(Effect::SummonCard {
                player_id: controller_id,
                card_id,
                zone,
            });
        }
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (UltimateHorror::NAME, |owner_id: PlayerId| {
        Box::new(UltimateHorror::new(owner_id))
    });
