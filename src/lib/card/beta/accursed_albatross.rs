use crate::{
    card::{Ability, Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct AccursedAlbatross {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl AccursedAlbatross {
    pub const NAME: &'static str = "Accursed Albatross";
    pub const DESCRIPTION: &'static str = "Airborne

When a unit kills Accursed Albatross, kill that unit's other allied minions it's nearby.";

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
                costs: Costs::basic(3, "W"),
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
            let allies = CardQuery::new()
                .controlled_by(&killer.get_controller_id(state))
                .near_to(killer.get_zone())
                .all(state);
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
