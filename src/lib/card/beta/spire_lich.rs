use crate::{
    card::{Card, CardBase, Edition, MinionType, Modifier, Plane, Rarity, SiteType, UnitBase, Zone},
    game::{Element, PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct SpireLich {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl SpireLich {
    pub const NAME: &'static str = "Spire Lich";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                modifiers: vec![],
                types: vec![MinionType::Undead],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 3,
                required_thresholds: Thresholds::parse("A"),
                plane: Plane::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }

    fn is_atop_tower(&self, state: &State) -> anyhow::Result<bool> {
        if !self.get_zone().is_in_realm() {
            return Ok(false);
        }

        let site = state
            .get_cards_in_zone(self.get_zone())
            .iter()
            .find(|c| c.is_site())
            .cloned();
        match site {
            Some(site) => Ok(site
                .get_site_base()
                .ok_or(anyhow::anyhow!("{} does not have site base", site.get_name()))?
                .types
                .contains(&SiteType::Tower)),
            None => Ok(false),
        }
    }
}

#[async_trait::async_trait]
impl Card for SpireLich {
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

    fn get_modifiers(&self, state: &State) -> anyhow::Result<Vec<Modifier>> {
        let mut modifiers = self.base_get_modifiers(state);
        if self.is_atop_tower(state)? {
            modifiers.push(Modifier::Ranged);
            modifiers.push(Modifier::Spellcaster(Element::Fire));
            modifiers.push(Modifier::Spellcaster(Element::Earth));
            modifiers.push(Modifier::Spellcaster(Element::Air));
            modifiers.push(Modifier::Spellcaster(Element::Water));
        }

        Ok(modifiers)
    }

    fn get_power(&self, state: &State) -> anyhow::Result<Option<u8>> {
        let mut power = self.base_get_power(state);
        if self.is_atop_tower(state)? {
            power = power.map(|p| p + 2);
        }
        Ok(power)
    }
}
