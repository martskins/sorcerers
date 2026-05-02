use std::collections::HashMap;

use crate::{
    card::{
        Ability, AreaModifiers, Aura, AuraBase, Card, CardBase, CardConstructor, Costs, Edition,
        Rarity, Region, Zone,
    },
    game::{Element, PlayerId},
    state::State,
};

#[derive(Debug, Clone)]
pub struct Silence {
    aura_base: AuraBase,
    card_base: CardBase,
}

impl Silence {
    pub const NAME: &'static str = "Silence";
    pub const DESCRIPTION: &'static str = "Minions at affected sites are silenced.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            aura_base: AuraBase {
                tapped: false,
                region: Region::Surface,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "WW"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Aura for Silence {}

#[async_trait::async_trait]
impl Card for Silence {
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
    fn get_aura_base(&self) -> Option<&AuraBase> {
        Some(&self.aura_base)
    }
    fn get_aura_base_mut(&mut self) -> Option<&mut AuraBase> {
        Some(&mut self.aura_base)
    }
    fn get_aura(&self) -> Option<&dyn Aura> {
        Some(self)
    }

    fn area_modifiers(&self, state: &State) -> AreaModifiers {
        if !self.get_zone().is_in_play() {
            return AreaModifiers::default();
        }
        let affected_zones = self.get_affected_zones(state);
        let removes: HashMap<uuid::Uuid, Vec<Ability>> = state
            .cards
            .iter()
            .filter(|c| c.get_unit_base().is_some())
            .filter(|c| affected_zones.contains(c.get_zone()))
            .map(|c| {
                (
                    *c.get_id(),
                    vec![
                        Ability::Spellcaster(None),
                        Ability::Spellcaster(Some(Element::Fire)),
                        Ability::Spellcaster(Some(Element::Water)),
                        Ability::Spellcaster(Some(Element::Earth)),
                        Ability::Spellcaster(Some(Element::Air)),
                    ],
                )
            })
            .collect();
        AreaModifiers {
            removes_abilities: removes,
            ..Default::default()
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Silence::NAME, |owner_id: PlayerId| {
    Box::new(Silence::new(owner_id))
});
