use crate::{
    card::{AvatarBase, Card, CardBase, Cost, Edition, Rarity, Region, UnitBase, Zone},
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
            .fire as u16;
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
            Effect::TapCard {
                card_id: card_id.clone(),
            },
        ];
        for minion_id in fire_minions {
            effects.push(Effect::BanishCard {
                card_id: minion_id,
                from: Zone::Cemetery,
            });
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
                region: Region::Surface,
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

    fn get_additional_activated_abilities(&self, _state: &State) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(ShootProjectile)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (Flamecaller::NAME, |owner_id: PlayerId| {
    Box::new(Flamecaller::new(owner_id))
});
