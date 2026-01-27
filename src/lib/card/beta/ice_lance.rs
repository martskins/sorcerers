use crate::{
    card::{Card, CardBase, CardType, Cost, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{CARDINAL_DIRECTIONS, PlayerId, pick_direction},
    query::CardQuery,
    state::State,
};

#[derive(Debug, Clone)]
pub struct IceLance {
    pub card_base: CardBase,
}

impl IceLance {
    pub const NAME: &'static str = "Ice Lance";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(3, "W"),
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
impl Card for IceLance {
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
                let qry = CardQuery::InZone {
                    id: uuid::Uuid::new_v4(),
                    zone: zone.clone(),
                    card_types: Some(vec![CardType::Minion, CardType::Avatar]),
                    regions: Some(vec![caster.get_region(state).clone()]),
                    owner: None,
                    prompt: Some("Pick a unit to damage with Ice Lance".to_string()),
                    tapped: None,
                };

                let options = qry.options(state);
                if options.is_empty() {
                    continue;
                }

                if options.len() == 1 && &options[0] == caster_id {
                    // Don't allow self-damage if it's the only option
                    continue;
                }

                let card_id = qry.resolve(&controller_id, state).await?;
                effects.push(Effect::TakeDamage {
                    card_id,
                    from: caster_id.clone(),
                    damage: dmg,
                });
            }
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (IceLance::NAME, |owner_id: PlayerId| Box::new(IceLance::new(owner_id)));