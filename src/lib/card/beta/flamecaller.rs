use crate::{
    card::{
        AdditionalCost, AvatarBase, Card, CardBase, Cost, Costs, Edition, Rarity, Region, UnitBase,
        Zone,
    },
    effect::Effect,
    game::{ActivatedAbility, CARDINAL_DIRECTIONS, Element, PlayerId, Thresholds, pick_direction},
    state::State,
};

#[derive(Debug, Clone)]
struct ShootProjectile;

#[async_trait::async_trait]
impl ActivatedAbility for ShootProjectile {
    fn get_name(&self) -> String {
        "Shoot Projectile".to_string()
    }

    fn get_cost(&self, card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::tap(card_id)))
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        _player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let fire_minions = state
            .cards
            .iter()
            .filter(|c| c.get_zone() == &Zone::Cemetery)
            .filter(|c| {
                c.get_elements(state)
                    .unwrap_or_default()
                    .contains(&Element::Fire)
            })
            .map(|c| c.get_id().clone())
            .collect::<Vec<_>>();
        let damage = state
            .cards
            .iter()
            .filter(|c| c.get_zone() == &Zone::Cemetery)
            .filter(|c| {
                c.get_elements(state)
                    .unwrap_or_default()
                    .contains(&Element::Fire)
            })
            .map(|c| c.get_costs(state).unwrap().thresholds_cost().clone())
            .sum::<Thresholds>()
            .fire as u16;
        let avatar = state.get_card(card_id);
        let prompt = "Flamecaller: Pick a direction to shoot the projectile:";
        let direction =
            pick_direction(avatar.get_owner_id(), &CARDINAL_DIRECTIONS, state, prompt).await?;
        let mut effects = vec![Effect::ShootProjectile {
            id: uuid::Uuid::new_v4(),
            player_id: avatar.get_owner_id().clone(),
            from_zone: avatar.get_zone().clone(),
            shooter: card_id.clone(),
            direction,
            damage,
            piercing: false,
            splash_damage: None,
        }];
        for minion_id in fire_minions {
            effects.push(Effect::BanishCard { card_id: minion_id });
        }

        Ok(effects)
    }
}

#[derive(Debug, Clone)]
pub struct Flamecaller {
    card_base: CardBase,
    unit_base: UnitBase,
    avatar_base: AvatarBase,
}

impl Flamecaller {
    pub const NAME: &'static str = "Flamecaller";
    pub const DESCRIPTION: &'static str = "Tap → Play or draw a site.\r \r Tap, Banish all your dead fire minions → Shoot a projectile. It deals damage equal to to the sum of their (F).";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 20,
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::ZERO,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
            avatar_base: AvatarBase {
                ..Default::default()
            },
        }
    }
}

impl Card for Flamecaller {
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

    fn get_avatar_base(&self) -> Option<&AvatarBase> {
        Some(&self.avatar_base)
    }

    fn get_avatar_base_mut(&mut self) -> Option<&mut AvatarBase> {
        Some(&mut self.avatar_base)
    }

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(ShootProjectile)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Flamecaller::NAME, |owner_id: PlayerId| {
        Box::new(Flamecaller::new(owner_id))
    });
