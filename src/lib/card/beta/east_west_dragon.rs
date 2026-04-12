use crate::{
    card::{Ability, Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    game::PlayerId,
    state::State,
};

#[derive(Debug, Clone)]
pub struct EastWestDragon {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl EastWestDragon {
    pub const NAME: &'static str = "East-West Dragon";
    pub const DESCRIPTION: &'static str = "Airborne\r Moves freely sideways.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                abilities: vec![Ability::Airborne],
                types: vec![MinionType::Dragon],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "AA"),
                region: Region::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for EastWestDragon {
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

    fn get_valid_move_zones(&self, state: &State) -> anyhow::Result<Vec<Zone>> {
        let sq = match self.get_zone().get_square() {
            Some(s) => s,
            None => return Ok(vec![]),
        };

        let row = (sq - 1) / 5;
        let row_start = row * 5 + 1;
        let row_end = row * 5 + 5;
        let self_zone = self.get_zone().clone();

        let same_row_zones: Vec<Zone> = (row_start..=row_end)
            .map(|s| Zone::Realm(s))
            .filter(|z| z != &self_zone)
            .filter(|z| z.get_site(state).is_some())
            .collect();

        Ok(same_row_zones)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (EastWestDragon::NAME, |owner_id: PlayerId| {
    Box::new(EastWestDragon::new(owner_id))
});
