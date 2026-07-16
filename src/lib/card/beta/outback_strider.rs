use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct OutbackStrider {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl OutbackStrider {
    pub const NAME: &'static str = "Outback Strider";
    pub const DESCRIPTION: &'static str =
        "Moves freely between land sites not occupied by other units.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![Ability::MovesFreely],
                types: vec![MinionType::Mortal],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "FF"),
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
impl Card for OutbackStrider {
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

    async fn get_valid_move_locations(&self, state: &State) -> anyhow::Result<Vec<Location>> {
        Ok(CardQuery::new()
            .land_sites()
            .in_play()
            .all(state)
            .into_iter()
            .map(|site_id| state.get_card(&site_id).get_location().clone())
            .filter(|location| {
                CardQuery::new()
                    .units()
                    .in_location(location.clone())
                    .id_not(*self.get_id())
                    .all(state)
                    .is_empty()
            })
            .collect())
    }

    async fn get_valid_move_paths(
        &self,
        state: &State,
        to: &Location,
    ) -> anyhow::Result<Vec<Vec<Location>>> {
        if self.get_valid_move_locations(state).await?.contains(to) {
            Ok(vec![vec![self.get_location().clone(), to.clone()]])
        } else {
            Ok(vec![])
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (OutbackStrider::NAME, |owner_id: PlayerId| {
        Box::new(OutbackStrider::new(owner_id))
    });
