use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    effect::{AbilityCounter, Effect},
    game::PlayerId,
    query::EffectQuery,
    state::State,
};

#[derive(Debug, Clone)]
pub struct SlumberingGiantess {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl SlumberingGiantess {
    pub const NAME: &'static str = "Slumbering Giantess";
    pub const DESCRIPTION: &'static str =
        "Genesis → Fall asleep. Slumbering Giantess is disabled until hurt.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![],
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "E"),
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
impl Card for SlumberingGiantess {
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

    async fn genesis(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![Effect::AddAbilityCounter {
            card_id: *self.get_id(),
            counter: AbilityCounter {
                id: uuid::Uuid::new_v4(),
                ability: Ability::Disabled,
                expires_on_effect: Some(EffectQuery::DamageDealt {
                    source: None,
                    target: Some(self.get_id().into()),
                }),
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (SlumberingGiantess::NAME, |owner_id: PlayerId| {
        Box::new(SlumberingGiantess::new(owner_id))
    });
