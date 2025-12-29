use crate::{
    card::{Card, CardBase, Edition, Plane, Rarity, Zone},
    effect::{CardQuery, Effect},
    game::{PlayerId, Thresholds, pick_zone},
    state::State,
};

#[derive(Debug, Clone)]
pub struct RaiseDead {
    pub card_base: CardBase,
}

impl RaiseDead {
    pub const NAME: &'static str = "Raise Dead";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 4,
                required_thresholds: Thresholds::parse("AA"),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for RaiseDead {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn is_tapped(&self) -> bool {
        self.card_base.tapped
    }

    fn get_owner_id(&self) -> &PlayerId {
        &self.card_base.owner_id
    }

    fn get_edition(&self) -> Edition {
        Edition::Beta
    }

    fn get_id(&self) -> &uuid::Uuid {
        &self.card_base.id
    }

    async fn on_cast(&mut self, state: &State, _caster_id: &uuid::Uuid) -> Vec<Effect> {
        let prompt = "Summon a random dead minion".to_string();
        let query = CardQuery::RandomUnitInZone { zone: Zone::Cemetery };
        let unit_id = query.resolve(self.get_owner_id(), state).await;
        let unit = state.get_card(&unit_id).unwrap();
        let zones = unit.get_valid_play_zones(state);
        let picked_zone = pick_zone(self.get_owner_id(), &zones, state, &prompt).await;
        vec![Effect::SummonCard {
            player_id: self.get_owner_id().clone(),
            card_id: unit_id,
            zone: picked_zone,
        }]
    }
}
