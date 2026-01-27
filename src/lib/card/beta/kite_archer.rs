use crate::{
    card::{Ability, Card, CardBase, Cost, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::{BaseOption, PlayerId, pick_option, pick_zone},
    query::ZoneQuery,
    state::State,
};

#[derive(Debug, Clone)]
pub struct KiteArcher {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl KiteArcher {
    pub const NAME: &'static str = "Kite Archer";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                abilities: vec![Ability::Ranged(1)],
                types: vec![MinionType::Mortal],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(3, "A"),
                region: Region::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for KiteArcher {
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

    async fn after_ranged_attack(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let options = vec![BaseOption::Yes, BaseOption::No];
        let option_labels = options.iter().map(|o| o.to_string()).collect::<Vec<_>>();
        let picked_action = pick_option(self.get_controller_id(state), &option_labels, state, "Take Step").await?;
        if options[picked_action] == BaseOption::No {
            return Ok(vec![]);
        }

        let zones = self.get_zone().get_adjacent();
        let picked_zone = pick_zone(self.get_owner_id(), &zones, state, false, "Choose to step to").await?;
        Ok(vec![Effect::MoveCard {
            player_id: self.get_owner_id().clone(),
            card_id: self.get_id().clone(),
            from: self.get_zone().clone(),
            to: ZoneQuery::Specific {
                id: uuid::Uuid::new_v4(),
                zone: picked_zone.clone(),
            },
            tap: false,
            region: self.card_base.region.clone(),
            through_path: None,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (KiteArcher::NAME, |owner_id: PlayerId| {
    Box::new(KiteArcher::new(owner_id))
});
