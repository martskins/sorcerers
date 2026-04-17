use crate::{
    card::{Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct GhostShip {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl GhostShip {
    pub const NAME: &'static str = "Ghost Ship";
    pub const DESCRIPTION: &'static str = "Voidwalk\r \r Whenever Ghost Ship enters a site from the void, you may summon a Spirit from any cemetery to its location.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 5,
                toughness: 5,
                abilities: vec![Ability::Voidwalk],
                types: vec![MinionType::Spirit],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(6, "W"),
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
impl Card for GhostShip {
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

    async fn on_visit_zone(
        &self,
        state: &State,
        from: &Zone,
        to: &Zone,
    ) -> anyhow::Result<Vec<Effect>> {
        // Only trigger if entering a site from the void.
        if from.get_site(state).is_some() {
            return Ok(vec![]);
        }

        // Only trigger if entering a site.
        if to.get_site(state).is_none() {
            return Ok(vec![]);
        }

        let player_id = self.get_controller_id(state);
        let Some(target_spirit) = CardQuery::new()
            .dead()
            .minions()
            .minion_type(&MinionType::Spirit)
            .pick(&player_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };

        Ok(vec![Effect::SummonCard {
            player_id,
            card_id: target_spirit,
            zone: to.clone(),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (GhostShip::NAME, |owner_id: PlayerId| {
    Box::new(GhostShip::new(owner_id))
});
