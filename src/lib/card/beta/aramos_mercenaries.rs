use crate::{
    card::{
        AdditionalCost, Card, CardBase, Cost, Costs, Edition, MinionType, Rarity, Region, UnitBase,
        Zone,
    },
    game::PlayerId,
    state::CardQuery,
};

#[derive(Debug, Clone)]
pub struct AramosMercenaries {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl AramosMercenaries {
    pub const NAME: &'static str = "Aramos Mercenaries";
    pub const DESCRIPTION: &'static str =
        "You may discard a random card rather than pay this spell's mana cost.";

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
                costs: Costs::basic(3, "FF").with_alternative(Cost::additional_only(
                    AdditionalCost::discard(
                        CardQuery::new()
                            .in_zone(&Zone::Hand)
                            .controlled_by(&owner_id)
                            .count(1)
                            .randomised(),
                    ),
                )),
                region: Region::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for AramosMercenaries {
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
    (AramosMercenaries::NAME, |owner_id: PlayerId| {
        Box::new(AramosMercenaries::new(owner_id))
    });
