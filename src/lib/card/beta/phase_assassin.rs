use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct PhaseAssassin {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl PhaseAssassin {
    pub const NAME: &'static str = "Phase Assassin";
    pub const DESCRIPTION: &'static str =
        "Voidwalk\n \nWhenever Phase Assassin enters the void, he gains Stealth.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![Ability::Voidwalk],
                types: vec![MinionType::Mortal],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "AA"),
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
impl Card for PhaseAssassin {
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

    async fn on_visit_zone(
        &self,
        state: &State,
        from: &Zone,
        to: &Zone,
    ) -> anyhow::Result<Vec<Effect>> {
        if from == to || to.get_site(state).is_some() || !to.is_in_play() {
            return Ok(vec![]);
        }

        if self.has_ability(state, &Ability::Stealth) {
            return Ok(vec![]);
        }

        Ok(vec![Effect::AddAbilityCounter {
            card_id: *self.get_id(),
            counter: AbilityCounter {
                id: uuid::Uuid::new_v4(),
                ability: Ability::Stealth,
                expires_on_effect: None,
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (PhaseAssassin::NAME, |owner_id: PlayerId| {
        Box::new(PhaseAssassin::new(owner_id))
    });
