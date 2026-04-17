use crate::{
    card::{Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::{BaseOption, PlayerId, pick_option, pick_zone},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct SkirmishersOfMu {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl SkirmishersOfMu {
    pub const NAME: &'static str = "Skirmishers of Mu";
    pub const DESCRIPTION: &'static str = "Ranged\r \r During basic movement, Skirmishers of Mu may perform a ranged strike from any location along their path.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![Ability::Ranged(1)],
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "AA"),
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
impl Card for SkirmishersOfMu {
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

    async fn on_move(&self, state: &State, path: &[Zone]) -> anyhow::Result<Vec<Effect>> {
        let options = [BaseOption::Yes, BaseOption::No];
        let option_labels = options.iter().map(|o| o.to_string()).collect::<Vec<_>>();
        let picked_option = pick_option(
            self.get_controller_id(state),
            &option_labels,
            state,
            "Ranged strike?",
            false,
        )
        .await?;
        if options[picked_option] == BaseOption::No {
            return Ok(vec![]);
        }

        let picked_zone = pick_zone(
            self.get_controller_id(state),
            path,
            state,
            false,
            "Skirmishers of Mu: Pick a zone to perform a ranged strike from",
        )
        .await?;

        let controller_id = self.get_controller_id(state);
        let Some(picked_unit_id) = CardQuery::new()
            .units()
            .near_to(&picked_zone)
            .with_prompt("Skirmishers of Mu: Pick a target for Ranged Strike")
            .pick(&controller_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };

        Ok(vec![Effect::RangedStrike {
            striker_id: *self.get_id(),
            target_id: picked_unit_id,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (SkirmishersOfMu::NAME, |owner_id: PlayerId| {
        Box::new(SkirmishersOfMu::new(owner_id))
    });
