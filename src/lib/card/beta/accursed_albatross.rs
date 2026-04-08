use crate::{
    card::{Ability, Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardMatcher, State},
};

#[derive(Debug, Clone)]
pub struct AccursedAlbatross {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl AccursedAlbatross {
    pub const NAME: &'static str = "Accursed Albatross";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                abilities: vec![Ability::Airborne],
                types: vec![MinionType::Beast],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::from_mana_and_threshold(3, "W"),
                region: Region::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for AccursedAlbatross {
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

    fn on_take_damage(&mut self, state: &State, from: &uuid::Uuid, damage: u16) -> anyhow::Result<Vec<Effect>> {
        let damage_effects = self.base_take_damage(state, from, damage)?;
        let mut was_killed = false;
        for effect in damage_effects {
            if matches!(effect, Effect::BuryCard { .. }) {
                was_killed = true;
                break;
            }
        }

        let mut effects = vec![];
        if was_killed {
            let killer = state.get_card(from);
            let allies = CardMatcher::new()
                .with_controller_id(&killer.get_controller_id(state))
                .with_zone_near_to(killer.get_zone())
                .resolve_ids(state);
            for ally in allies {
                if &ally == self.get_id() {
                    continue;
                }

                effects.push(Effect::BuryCard { card_id: ally });
            }
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (AccursedAlbatross::NAME, |owner_id: PlayerId| {
    Box::new(AccursedAlbatross::new(owner_id))
});
