use crate::{
    card::{Card, CardBase, Cost, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{CARDINAL_DIRECTIONS, PlayerId, pick_direction},
    state::State,
};

#[derive(Debug, Clone)]
pub struct HeatRay {
    pub card_base: CardBase,
}

impl HeatRay {
    pub const NAME: &'static str = "Heat Ray";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(2, "F"),
                region: Region::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for HeatRay {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    async fn on_cast(&mut self, state: &State, caster_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        let caster = state.get_card(caster_id);
        let prompt = "Heat Ray: Pick a direction to cast the spell:";
        let direction = pick_direction(self.get_owner_id(), &CARDINAL_DIRECTIONS, state, prompt).await?;
        Ok(vec![Effect::ShootProjectile {
            id: uuid::Uuid::new_v4(),
            player_id: self.get_owner_id().clone(),
            shooter: caster.get_id().clone(),
            from_zone: caster.get_zone().clone(),
            direction,
            damage: 2,
            piercing: true,
            splash_damage: None,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (HeatRay::NAME, |owner_id: PlayerId| Box::new(HeatRay::new(owner_id)));
