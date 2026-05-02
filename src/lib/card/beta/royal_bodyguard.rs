use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone,
    },
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct RoyalBodyguard {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl RoyalBodyguard {
    pub const NAME: &'static str = "Royal Bodyguard";
    pub const DESCRIPTION: &'static str = "If a nearby Avatar or royalty (King, Queen, Prince, or Princess) would take damage, Royal Bodyguard may take that damage instead.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "EE"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for RoyalBodyguard {
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

    async fn replace_effect(
        &self,
        state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Option<Vec<Effect>>> {
        match effect {
            Effect::TakeDamage {
                card_id,
                from,
                damage,
                is_strike,
                is_ranged,
            } => {
                let target = state.get_card(card_id);
                let is_nearby = self.get_zone().is_nearby(target.get_zone());
                if !is_nearby {
                    return Ok(None);
                }

                let is_royalty = target.is_minion()
                    && ["King", "Queen", "Prince", "Princess"]
                        .iter()
                        .any(|title| self.get_name().contains(title));
                let is_avatar = target.is_avatar();
                let is_royalty_or_avatar = is_royalty || is_avatar;
                if !is_royalty_or_avatar {
                    return Ok(None);
                }

                Ok(Some(vec![Effect::TakeDamage {
                    card_id: *self.get_id(),
                    from: *from,
                    damage: *damage,
                    is_strike: *is_strike,
                    is_ranged: *is_ranged,
                }]))
            }
            _ => Ok(None),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (RoyalBodyguard::NAME, |owner_id: PlayerId| {
        Box::new(RoyalBodyguard::new(owner_id))
    });
