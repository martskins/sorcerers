use crate::{
    card::{AvatarBase, Card, CardBase, Cost, Edition, Rarity, Region, UnitBase, Zone},
    game::PlayerId,
    state::State,
};

#[derive(Debug, Clone)]
pub struct AvatarOfWater {
    pub card_base: CardBase,
    pub unit_base: UnitBase,
    pub avatar_base: AvatarBase,
}

impl AvatarOfWater {
    pub const NAME: &'static str = "Avatar of Water";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 20,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::zero(),
                region: Region::Surface,
                rarity: Rarity::Unique,
                edition: Edition::Alpha,
                controller_id: owner_id.clone(),
            },
            avatar_base: AvatarBase {},
        }
    }
}

impl Card for AvatarOfWater {
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

    fn get_avatar_base(&self) -> Option<&AvatarBase> {
        Some(&self.avatar_base)
    }

    fn get_avatar_base_mut(&mut self) -> Option<&mut AvatarBase> {
        Some(&mut self.avatar_base)
    }

    fn get_image_path(&self) -> String {
        "https://d27a44hjr9gen3.cloudfront.net/cards/alp-avatar_of_water-pd-s.png".to_string()
    }

    // TODO: Implement the special abilities of Avatar of Water
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (AvatarOfWater::NAME, |owner_id: PlayerId| {
    Box::new(AvatarOfWater::new(owner_id))
});
