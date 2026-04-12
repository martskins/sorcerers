use crate::{
    card::{AvatarBase, Card, CardBase, Costs, Edition, Rarity, Region, UnitBase, Zone},
    game::PlayerId,
};

#[derive(Debug, Clone)]
pub struct Battlemage {
    pub card_base: CardBase,
    pub unit_base: UnitBase,
    pub avatar_base: AvatarBase,
}

impl Battlemage {
    pub const NAME: &'static str = "Battlemage";
    pub const DESCRIPTION: &'static str =
        "Tap → Play or draw a site.\r \r Whenever Battlemage attacks and kills an enemy, you may draw a spell.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 20,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::ZERO,
                region: Region::Surface,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
            avatar_base: AvatarBase { ..Default::default() },
        }
    }
}

impl Card for Battlemage {
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

    fn get_avatar_base(&self) -> Option<&AvatarBase> {
        Some(&self.avatar_base)
    }

    fn get_avatar_base_mut(&mut self) -> Option<&mut AvatarBase> {
        Some(&mut self.avatar_base)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (Battlemage::NAME, |owner_id: PlayerId| {
    Box::new(Battlemage::new(owner_id))
});
