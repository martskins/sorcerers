use crate::{
    card::{Ability, AdditionalCost, Card, CardBase, Cost, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::{ActivatedAbility, BaseOption, CARDINAL_DIRECTIONS, PlayerId, pick_direction, pick_option},
    query::{CardQuery, ZoneQuery},
    state::State,
};

#[derive(Debug, Clone)]
struct ShootProjectile;

#[async_trait::async_trait]
impl ActivatedAbility for ShootProjectile {
    fn get_name(&self) -> String {
        "Tap to shoot projectile".to_string()
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let direction = pick_direction(
            player_id,
            &CARDINAL_DIRECTIONS,
            state,
            "Pudge Butcher: Choose a direction to shoot the projectile in",
        )
        .await?;

        let mut snapshot = state.snapshot();
        let pudge = state.get_card(card_id);
        let effect = Effect::ShootProjectile {
            id: uuid::Uuid::new_v4(),
            player_id: player_id.clone(),
            shooter: card_id.clone(),
            from_zone: pudge.get_zone().clone(),
            direction,
            damage: 0,
            piercing: false,
            splash_damage: None,
        };
        effect.apply(&mut snapshot).await?;
        let mut target = None;
        for effect in snapshot.effects {
            match *effect {
                Effect::TakeDamage {
                    card_id: target_id,
                    from,
                    damage: 0,
                    ..
                } if &from == card_id => {
                    target = Some(target_id.clone());
                    break;
                }
                _ => {}
            }
        }

        let mut effects = vec![effect];
        if let Some(target) = target {
            effects.push(Effect::MoveCard {
                card_id: target.clone(),
                player_id: player_id.clone(),
                from: state.get_card(&target).get_zone().clone(),
                to: ZoneQuery::Specific {
                    id: uuid::Uuid::new_v4(),
                    zone: pudge.get_zone().clone(),
                },
                tap: false,
                region: pudge.get_region(state).clone(),
                through_path: None,
            });
            let target_name = state.get_card(&target).get_name().to_string();
            let options = vec![BaseOption::Yes, BaseOption::No];
            let option_labels: Vec<String> = options.iter().map(|o| o.to_string()).collect();
            let picked_option = pick_option(
                player_id,
                &option_labels,
                state,
                format!("Pudge Butcher: Fight {}?", target_name),
            )
            .await?;
            if options[picked_option] == BaseOption::Yes {
                effects.push(Effect::Attack {
                    attacker_id: card_id.clone(),
                    defender_id: target,
                });
            }
        }

        effects.reverse();
        Ok(effects)
    }

    fn get_cost(&self, card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost {
            additional: vec![AdditionalCost::Tap {
                card: CardQuery::Specific {
                    id: uuid::Uuid::new_v4(),
                    card_id: card_id.clone(),
                },
            }],
            ..Default::default()
        })
    }
}

#[derive(Debug, Clone)]
pub struct PudgeButcher {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl PudgeButcher {
    pub const NAME: &'static str = "Pudge Butcher";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 5,
                toughness: 5,
                abilities: vec![Ability::Immobile],
                types: vec![MinionType::Demon],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(4, "EE"),
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
impl Card for PudgeButcher {
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

    fn get_additional_activated_abilities(&self, _state: &State) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(ShootProjectile)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (PudgeButcher::NAME, |owner_id: PlayerId| {
    Box::new(PudgeButcher::new(owner_id))
});
