use crate::{
    card::{Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    query::ZoneQuery,
    state::State,
};

#[derive(Debug, Clone)]
pub struct DeepSeaMermaids {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl DeepSeaMermaids {
    pub const NAME: &'static str = "Deep-Sea Mermaids";
    pub const DESCRIPTION: &'static str = "Submerge\r \r Genesis → Draw your bottommost spell.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                abilities: vec![Ability::Submerge],
                types: vec![MinionType::Merfolk],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "WW"),
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
impl Card for DeepSeaMermaids {
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

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let deck = state.get_player_deck(&controller_id)?;
        if let Some(card_id) = deck.spells.first() {
            return Ok(vec![Effect::MoveCard {
                player_id: controller_id,
                card_id: *card_id,
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
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (DeepSeaMermaids::NAME, |owner_id: PlayerId| {
        Box::new(DeepSeaMermaids::new(owner_id))
    });
