use crate::{
    card::{
        Ability, AdditionalCost, Card, CardBase, CardConstructor, Cost, Costs, Edition, MinionType,
        Rarity, Region, UnitBase, Zone,
    },
    effect::Effect,
    game::{
        ActivatedAbility, BaseOption, CARDINAL_DIRECTIONS, PlayerId, pick_direction, pick_option,
    },
    query::ZoneQuery,
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
            player_id: *player_id,
            shooter: *card_id,
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
                    target = Some(target_id);
                    break;
                }
                _ => {}
            }
        }

        let mut effects = vec![effect];
        if let Some(target) = target {
            effects.push(Effect::MoveCard {
                card_id: target,
                player_id: *player_id,
                from: state.get_card(&target).get_zone().clone(),
                to: ZoneQuery::from_zone(pudge.get_zone().clone()),
                tap: false,
                region: pudge.get_region(state).clone(),
                through_path: None,
            });
            let target_name = state.get_card(&target).get_name().to_string();
            let options = [BaseOption::Yes, BaseOption::No];
            let option_labels: Vec<String> = options.iter().map(|o| o.to_string()).collect();
            let picked_option = pick_option(
                player_id,
                &option_labels,
                state,
                format!("Pudge Butcher: Fight {}?", target_name),
                false,
            )
            .await?;
            if options[picked_option] == BaseOption::Yes {
                effects.push(Effect::Attack {
                    attacker_id: *card_id,
                    defender_id: target,
                });
            }
        }

        effects.reverse();
        Ok(effects)
    }

    fn get_cost(&self, card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::tap(card_id)))
    }
}

#[derive(Debug, Clone)]
pub struct PudgeButcher {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl PudgeButcher {
    pub const NAME: &'static str = "Pudge Butcher";
    pub const DESCRIPTION: &'static str = "Immobile\r \r Tap → Shoot a projectile. If it hits a unit, drag it to this location. Pudge may fight it when it arrives.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 5,
                toughness: 5,
                abilities: vec![Ability::Immobile],
                types: vec![MinionType::Demon],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "EE"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for PudgeButcher {
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

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(ShootProjectile)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (PudgeButcher::NAME, |owner_id: PlayerId| {
    Box::new(PudgeButcher::new(owner_id))
});
