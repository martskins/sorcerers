use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct HoundsOfOndaros {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl HoundsOfOndaros {
    pub const NAME: &'static str = "Hounds of Ondaros";
    pub const DESCRIPTION: &'static str = "Airborne, Burrowing, Submerge, Voidwalk\r \r Removes Stealth from all nearby enemies.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                abilities: vec![
                    Ability::Airborne,
                    Ability::Burrowing,
                    Ability::Submerge,
                    Ability::Voidwalk,
                ],
                types: vec![MinionType::Beast],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "AA"),
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
impl Card for HoundsOfOndaros {
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

    fn area_effects(&self, state: &State) -> anyhow::Result<Vec<crate::effect::Effect>> {
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }
        let controller_id = self.get_controller_id(state);

        let effects = CardQuery::new()
            .minions()
            .near_to(self.get_zone())
            .with_abilities(vec![Ability::Stealth])
            .all(state)
            .into_iter()
            .filter(|id| state.get_card(id).get_controller_id(state) != controller_id)
            .map(|id| crate::effect::Effect::RemoveAbility {
                card_id: id,
                modifier: Ability::Stealth,
            })
            .collect();

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (HoundsOfOndaros::NAME, |owner_id: PlayerId| {
        Box::new(HoundsOfOndaros::new(owner_id))
    });
