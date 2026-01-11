use crate::{
    card::{AvatarBase, Card, CardBase, Cost, Edition, Plane, Rarity, UnitBase, Zone},
    effect::Effect,
    game::{ActivatedAbility, CARDINAL_DIRECTIONS, Element, PlayerId, Thresholds, pick_direction},
    state::State,
};

#[derive(Debug, Clone)]
struct ShootProjectile;

#[async_trait::async_trait]
impl ActivatedAbility for ShootProjectile {
    fn get_name(&self) -> &str {
        "Shoot Projectile"
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
            .filter(|c| c.get_elements(state).unwrap_or_default().contains(&Element::Fire))
            .map(|c| c.get_id().clone())
            .collect::<Vec<_>>();
        let damage = state
            .cards
            .iter()
            .filter(|c| c.get_zone() == &Zone::Cemetery)
            .filter(|c| c.get_elements(state).unwrap_or_default().contains(&Element::Fire))
            .map(|c| c.get_cost(state).unwrap_or_default().thresholds.clone())
            .sum::<Thresholds>()
            .fire;
        let avatar = state.get_card(card_id);
        let prompt = "Flamecaller: Pick a direction to shoot the projectile:";
        let direction = pick_direction(avatar.get_owner_id(), &CARDINAL_DIRECTIONS, state, prompt).await?;
        let mut effects = vec![
            Effect::ShootProjectile {
                id: uuid::Uuid::new_v4(),
                player_id: avatar.get_owner_id().clone(),
                from_zone: avatar.get_zone().clone(),
                shooter: card_id.clone(),
                direction,
                damage,
                piercing: false,
                splash_damage: None,
            },
            Effect::tap_card(card_id),
        ];
        for minion_id in fire_minions {
            effects.push(Effect::banish_card(&minion_id, &Zone::Cemetery));
        }

        Ok(effects)
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
                toughness: 20,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::zero(),
                plane: Plane::Surface,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
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

    fn get_activated_abilities(&self, state: &State) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        let mut activated_abilities = self.base_avatar_activated_abilities(state)?;
        activated_abilities.push(Box::new(ShootProjectile));
        Ok(activated_abilities)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (Flamecaller::NAME, |owner_id: PlayerId| {
    Box::new(Flamecaller::new(owner_id))
});
