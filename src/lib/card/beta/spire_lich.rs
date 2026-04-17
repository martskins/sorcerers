use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, SiteType, UnitBase,
        Zone,
    },
    game::PlayerId,
    state::State,
};

#[derive(Debug, Clone)]
pub struct SpireLich {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl SpireLich {
    pub const NAME: &'static str = "Spire Lich";
    pub const DESCRIPTION: &'static str =
        "If Spire Lich is atop a Tower, it has +2 power, Ranged, and Spellcaster.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                abilities: vec![],
                types: vec![MinionType::Undead],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "A"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }

    fn is_atop_tower(&self, state: &State) -> anyhow::Result<bool> {
        if !self.get_zone().is_in_play() {
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
                .ok_or(anyhow::anyhow!(
                    "{} does not have site base",
                    site.get_name()
                ))?
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

    fn get_abilities(&self, state: &State) -> anyhow::Result<Vec<Ability>> {
        let mut modifiers = self.base_get_abilities(state);
        if self.is_atop_tower(state)? {
            modifiers.push(Ability::Ranged(1));
            modifiers.push(Ability::Spellcaster(None));
        }

        Ok(modifiers)
    }

    fn get_power(&self, state: &State) -> anyhow::Result<Option<u16>> {
        let mut power = self.base_get_power(state);
        if self.is_atop_tower(state)? {
            power = power.map(|p| p + 2);
        }
        Ok(power)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (SpireLich::NAME, |owner_id: PlayerId| {
    Box::new(SpireLich::new(owner_id))
});
