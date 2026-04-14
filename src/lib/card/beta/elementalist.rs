use crate::{
    card::{
        AvatarBase, Card, CardBase, Costs, Edition, Rarity, Region, ResourceProvider, UnitBase,
        Zone,
    },
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct Elementalist {
    pub card_base: CardBase,
    pub unit_base: UnitBase,
    pub avatar_base: AvatarBase,
}

impl Elementalist {
    pub const NAME: &'static str = "Elementalist";
    pub const DESCRIPTION: &'static str =
        "Tap → Play or draw a site.\r \r You have an additional (E)(F)(W)(A).";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 20,
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::ZERO,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
            avatar_base: AvatarBase {
                ..Default::default()
            },
        }
    }
}

impl ResourceProvider for Elementalist {
    fn provided_mana(&self, _state: &State) -> anyhow::Result<u8> {
        Ok(0)
    }

    fn provided_affinity(&self, _state: &State) -> anyhow::Result<Thresholds> {
        Ok(Thresholds {
            earth: 1,
            fire: 1,
            water: 1,
            air: 1,
        })
    }
}

impl Card for Elementalist {
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

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Elementalist::NAME, |owner_id: PlayerId| {
        Box::new(Elementalist::new(owner_id))
    });
