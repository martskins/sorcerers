use crate::{
    card::{AdditionalCost, AvatarBase, Card, CardBase, CardType, Cost, Edition, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::{ActivatedAbility, PlayerId, Thresholds, pick_card, yes_or_no},
    query::CardQuery,
    state::{CardMatcher, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
struct FloodSite;

#[async_trait::async_trait]
impl ActivatedAbility for FloodSite {
    fn get_name(&self) -> String {
        "Flood Site".to_string()
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let avatar = state.get_card(card_id);
        match state.get_body_of_water_at(avatar.get_zone()) {
            Some(body_of_water) => {
                let adjacent_sites = CardMatcher::new()
                    .adjacent_to_zones(&body_of_water)
                    .card_type(CardType::Site)
                    .resolve_ids(state);
                let prompt = "Avatar of Water: Pick a site to flood";
                let picked_site_id = pick_card(player_id, &adjacent_sites, state, prompt).await?;
                let mut effects = vec![Effect::SetCardData {
                    card_id: card_id.clone(),
                    data: Box::new(ContinuousEffect::FloodSites {
                        affected_sites: CardMatcher::from_id(picked_site_id),
                    }),
                }];
                let teleport = yes_or_no(player_id, state, "Avatar of Water: Teleport to the flooded site?").await?;
                if teleport {
                    let picked_site = state.get_card(&picked_site_id);
                    effects.push(Effect::SetCardZone {
                        card_id: card_id.clone(),
                        zone: picked_site.get_zone().clone(),
                    });
                }
                effects.reverse();
                Ok(effects)
            }
            None => Ok(vec![]),
        }
    }

    fn get_cost(&self, card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost {
            mana: 0,
            thresholds: Thresholds::ZERO,
            additional: vec![AdditionalCost::Tap {
                card: CardQuery::from_id(card_id.clone()),
            }],
        })
    }
}

#[derive(Debug, Clone)]
pub struct AvatarOfWater {
    pub card_base: CardBase,
    pub unit_base: UnitBase,
    pub avatar_base: AvatarBase,
    flood_effect: Option<ContinuousEffect>,
}

impl AvatarOfWater {
    pub const NAME: &'static str = "Avatar of Water";

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
                edition: Edition::Alpha,
                controller_id: owner_id.clone(),
            },
            avatar_base: AvatarBase {},
            flood_effect: None,
        }
    }
}

#[async_trait::async_trait]
impl Card for AvatarOfWater {
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

    fn get_image_path(&self) -> String {
        "https://d27a44hjr9gen3.cloudfront.net/cards/alp-avatar_of_water-pd-s.png".to_string()
    }

    fn get_additional_activated_abilities(
        &self,
        _state: &crate::state::State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(FloodSite)])
    }

    async fn get_continuous_effects(&self, _state: &State) -> anyhow::Result<Vec<ContinuousEffect>> {
        match &self.flood_effect {
            Some(effect) => Ok(vec![effect.clone()]),
            None => Ok(vec![]),
        }
    }

    fn set_data(&mut self, data: &Box<dyn std::any::Any + Send + Sync>) -> anyhow::Result<()> {
        if let Some(effect) = data.downcast_ref::<ContinuousEffect>() {
            self.flood_effect = Some(effect.clone());
        }

        Ok(())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (AvatarOfWater::NAME, |owner_id: PlayerId| {
    Box::new(AvatarOfWater::new(owner_id))
});
