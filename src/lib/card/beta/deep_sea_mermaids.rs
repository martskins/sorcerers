use crate::{
    card::{Ability, Card, CardBase, Cost, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    query::ZoneQuery,
    state::State,
};

#[derive(Debug, Clone)]
pub struct DeepSeaMermaids {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl DeepSeaMermaids {
    pub const NAME: &'static str = "Deep-Sea Mermaids";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                abilities: vec![Ability::Submerge],
                types: vec![MinionType::Merfolk],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(3, "WW"),
                region: Region::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for DeepSeaMermaids {
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

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let deck = state.get_player_deck(&controller_id)?;
        if let Some(card_id) = deck.spells.first() {
            return Ok(vec![Effect::MoveCard {
                player_id: controller_id.clone(),
                card_id: card_id.clone(),
                from: Zone::Spellbook,
                to: ZoneQuery::from_zone(Zone::Hand),
                tap: false,
                region: Region::Surface,
                through_path: None,
            }]);
        }

        Ok(vec![])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (DeepSeaMermaids::NAME, |owner_id: PlayerId| {
    Box::new(DeepSeaMermaids::new(owner_id))
});
