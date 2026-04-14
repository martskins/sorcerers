use crate::{
    card::{Ability, Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    game::PlayerId,
};

/// **Daperyll Vampire** — Exceptional Minion (5 cost, 4/4)
///
/// Airborne
/// Daperyll Vampire's strike damage against units heals you.
#[derive(Debug, Clone)]
pub struct DaperyllVampire {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl DaperyllVampire {
    pub const NAME: &'static str = "Daperyll Vampire";
    pub const DESCRIPTION: &'static str =
        "Airborne\r \r Daperyll Vampire's strike damage against units heals you.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                abilities: vec![Ability::Airborne, Ability::Lifesteal],
                types: vec![MinionType::Undead],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "A"),
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
impl Card for DaperyllVampire {
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
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (DaperyllVampire::NAME, |owner_id: PlayerId| {
        Box::new(DaperyllVampire::new(owner_id))
    });
