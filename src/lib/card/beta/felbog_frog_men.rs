use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    game::PlayerId,
};

#[derive(Debug, Clone)]
pub struct FelbogFrogMen {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl FelbogFrogMen {
    pub const NAME: &'static str = "Felbog Frog Men";
    pub const DESCRIPTION: &'static str = "Can leap entirely over adjacent sites in one step.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                // TODO: This should use a new Leap ability. Leap is different to Movement in that
                // it will ignore effects of entering the region it jumps over. For example, if a
                // site that is one step away from Felbog Frog Men doesn't allow minions entering
                // it, it can still leap over it, because it doesn't actually enter that site. A
                // card with Movement(2) would not be able to traverse that site, because it would
                // have to enter it to get to the next site.
                abilities: vec![Ability::Movement(2)],
                types: vec![MinionType::Beast],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "W"),
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
impl Card for FelbogFrogMen {
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
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (FelbogFrogMen::NAME, |owner_id: PlayerId| {
        Box::new(FelbogFrogMen::new(owner_id))
    });
