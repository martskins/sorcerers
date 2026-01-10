use crate::{
    card::{Ability, Card, CardBase, Cost, Edition, MinionType, Plane, Rarity, UnitBase, Zone},
    effect::Effect,
    game::{BaseOption, PlayerId, pick_card, pick_option, pick_zone},
    state::State,
};

#[derive(Debug, Clone)]
pub struct SkirmishersOfMu {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl SkirmishersOfMu {
    pub const NAME: &'static str = "Skirmishers of Mu";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![Ability::Ranged(1)],
                types: vec![MinionType::Mortal],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(4, "AA"),
                plane: Plane::Air,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for SkirmishersOfMu {
    fn get_name(&self) -> &str {
        Self::NAME
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

    async fn on_move(&self, state: &State, path: &[Zone]) -> anyhow::Result<Vec<Effect>> {
        let options = vec![BaseOption::Yes, BaseOption::No];
        let option_labels = options.iter().map(|o| o.to_string()).collect::<Vec<_>>();
        let picked_option = pick_option(self.get_controller_id(), &option_labels, state, "Ranged strike?").await?;
        if options[picked_option] == BaseOption::No {
            return Ok(vec![]);
        }

        let picked_zone = pick_zone(
            self.get_controller_id(),
            &path,
            state,
            "Skirmishers of Mu: Pick a zone to perform a ranged strike from",
        )
        .await?;

        let units: Vec<uuid::Uuid> = picked_zone
            .get_nearby_units(state, None)
            .iter()
            .map(|c| c.get_id().clone())
            .collect();
        let target_unit = pick_card(
            self.get_controller_id(),
            &units,
            state,
            "Pick a target for Ranged Strike:",
        )
        .await?;
        Ok(vec![Effect::RangedStrike {
            attacker_id: self.get_id().clone(),
            defender_id: target_unit,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (SkirmishersOfMu::NAME, |owner_id: PlayerId| {
    Box::new(SkirmishersOfMu::new(owner_id))
});
