use crate::{
    card::{Card, CardBase, Cost, Costs, Edition, Rarity, Zone},
    effect::Effect,
    game::{CARDINAL_DIRECTIONS, PlayerId, pick_direction},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct IceLance {
    pub card_base: CardBase,
}

impl IceLance {
    pub const NAME: &'static str = "Ice Lance";
    pub const DESCRIPTION: &'static str = "Shoot a piercing projectile. Deal 3, then 2, then 1 damage to up to one hit unit at each of the first three locations along its path.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "W"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for IceLance {
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

    async fn on_cast(
        &mut self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let prompt = "Ice Lance: Pick a direction to shoot the lance";
        let direction = pick_direction(controller_id, &CARDINAL_DIRECTIONS, state, prompt).await?;
        let caster = state.get_card(caster_id);

        let zone_dmg = vec![
            (Some(caster.get_zone().clone()), 3),
            (caster.get_zone().zone_in_direction(&direction, 1), 2),
            (caster.get_zone().zone_in_direction(&direction, 2), 1),
        ];

        let mut effects = vec![];
        for (zone, dmg) in zone_dmg {
            if let Some(zone) = zone {
                let qry = CardQuery::new()
                    .in_zone(&zone)
                    .units()
                    .id_not_in(vec![caster_id.clone()])
                    .in_region(caster.get_region(state))
                    .with_prompt("Pick a unit to damage with Ice Lance");

                if let Some(card_id) = qry.pick(&controller_id, state, false).await? {
                    effects.push(Effect::TakeDamage {
                        card_id,
                        from: caster_id.clone(),
                        damage: dmg,
                        is_strike: false,
                    });
                }
            }
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (IceLance::NAME, |owner_id: PlayerId| {
        Box::new(IceLance::new(owner_id))
    });
