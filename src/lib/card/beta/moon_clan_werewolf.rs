use crate::{
    card::{Card, CardBase, CardConstructor, Costs, Edition, Rarity, Region, UnitBase, Zone},
    game::PlayerId,
};

#[derive(Debug, Clone)]
pub struct MoonClanWerewolf {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl MoonClanWerewolf {
    pub const NAME: &'static str = "Moon Clan Werewolf";
    pub const DESCRIPTION: &'static str =
        "Sacrifice an allied Mortal → Summon Moon Clan Werewolf from your hand to the Mortal's location.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![],
                types: vec![],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "A"),
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
impl Card for MoonClanWerewolf {
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
    (MoonClanWerewolf::NAME, |owner_id: PlayerId| {
        Box::new(MoonClanWerewolf::new(owner_id))
    });
