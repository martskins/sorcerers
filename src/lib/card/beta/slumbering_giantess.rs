use crate::{
    card::{Ability, Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::{AbilityCounter, Effect},
    game::PlayerId,
    query::EffectQuery,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct SlumberingGiantess {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl SlumberingGiantess {
    pub const NAME: &'static str = "Slumbering Giantess";
    pub const DESCRIPTION: &'static str = "Genesis → Fall asleep. Slumbering Giantess is disabled until hurt.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![],
                types: vec![MinionType::Mortal],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "E"),
                region: Region::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
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
            card_id: self.get_id().clone(),
            counter: AbilityCounter {
                id: uuid::Uuid::new_v4(),
                ability: Ability::Disabled,
                expires_on_effect: Some(EffectQuery::DamageDealt {
                    source: None,
                    target: Some(CardQuery::from_id(self.get_id().clone())),
                }),
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (SlumberingGiantess::NAME, |owner_id: PlayerId| {
    Box::new(SlumberingGiantess::new(owner_id))
});
