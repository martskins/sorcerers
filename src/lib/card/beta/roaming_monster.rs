use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone,
    },
    game::PlayerId,
    state::State,
};

#[derive(Debug, Clone)]
pub struct RoamingMonster {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl RoamingMonster {
    pub const NAME: &'static str = "Roaming Monster";
    pub const DESCRIPTION: &'static str = "May be summoned to any site.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                abilities: vec![],
                types: vec![MinionType::Monster],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "A"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for RoamingMonster {
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

    fn get_valid_play_zones(&self, state: &State) -> anyhow::Result<Vec<Zone>> {
        Ok((1..=20)
            .filter_map(|z| {
                state
                    .get_cards_in_zone(&Zone::Realm(z))
                    .iter()
                    .find(|c| c.is_site())
                    .map(|_| Zone::Realm(z))
            })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (RoamingMonster::NAME, |owner_id: PlayerId| {
        Box::new(RoamingMonster::new(owner_id))
    });
