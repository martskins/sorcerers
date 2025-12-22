use crate::{
    card::{AvatarBase, Card, CardBase, Edition, Plane, UnitBase, Zone},
    effect::Effect,
    game::{Action, CARDINAL_DIRECTIONS, Element, PlayerId, Thresholds, pick_direction},
    state::State,
};

#[derive(Debug, Clone)]
enum FlamecallerAction {
    ShootProjectile,
}

#[async_trait::async_trait]
impl Action for FlamecallerAction {
    fn get_name(&self) -> &str {
        match self {
            FlamecallerAction::ShootProjectile => "Shoot Projectile",
        }
    }

    async fn on_select(
        &self,
        card_id: Option<&uuid::Uuid>,
        _player_id: &PlayerId,
        state: &State,
    ) -> Vec<crate::effect::Effect> {
        match self {
            FlamecallerAction::ShootProjectile => {
                let fire_minions = state
                    .cards
                    .iter()
                    .filter(|c| c.get_zone() == &Zone::Cemetery)
                    .filter(|c| c.get_elements(state).contains(&Element::Fire))
                    .map(|c| c.get_id().clone())
                    .collect::<Vec<_>>();
                let damage = state
                    .cards
                    .iter()
                    .filter(|c| c.get_zone() == &Zone::Cemetery)
                    .filter(|c| c.get_elements(state).contains(&Element::Fire))
                    .map(|c| c.get_required_thresholds(state).clone())
                    .sum::<Thresholds>()
                    .fire;
                let avatar = state.get_card(card_id.unwrap()).unwrap();
                let direction = pick_direction(avatar.get_owner_id(), &CARDINAL_DIRECTIONS, state).await;
                let mut effects = vec![
                    Effect::ShootProjectile {
                        player_id: avatar.get_owner_id().clone(),
                        from_zone: avatar.get_zone().clone(),
                        shooter: card_id.unwrap().clone(),
                        direction,
                        damage,
                        piercing: false,
                    },
                    Effect::tap_card(card_id.unwrap()),
                ];
                for minion_id in fire_minions {
                    effects.push(Effect::banish_card(&minion_id, &Zone::Cemetery));
                }

                effects
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Flamecaller {
    pub card_base: CardBase,
    pub unit_base: UnitBase,
    pub avatar_base: AvatarBase,
}

impl Flamecaller {
    pub const NAME: &'static str = "Flamecaller";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 0,
                required_thresholds: Thresholds::new(),
                plane: Plane::Surface,
            },
            avatar_base: AvatarBase {},
        }
    }
}

impl Card for Flamecaller {
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

    fn get_avatar_base(&self) -> Option<&AvatarBase> {
        Some(&self.avatar_base)
    }

    fn get_avatar_base_mut(&mut self) -> Option<&mut AvatarBase> {
        Some(&mut self.avatar_base)
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

    fn get_actions(&self, _: &State) -> Vec<Box<dyn Action>> {
        let mut actions = self.base_avatar_actions();
        actions.push(Box::new(FlamecallerAction::ShootProjectile));
        actions
    }
}
