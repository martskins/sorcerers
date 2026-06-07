use std::sync::Arc;

use crate::prelude::*;

#[derive(Debug, Clone)]
struct WaveshaperFlood;

#[async_trait::async_trait]
impl ActivatedAbility for WaveshaperFlood {
    fn get_name(&self) -> String {
        "Flood Site".to_string()
    }

    fn get_cost(&self, card_id: &CardId, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::tap(card_id)))
    }

    async fn on_select(
        &self,
        card_id: &CardId,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let avatar = state.get_card(card_id);
        let controller_id = avatar.get_controller_id(state);
        let Some(body_of_water) = state.get_body_of_water_at(avatar.get_zone()) else {
            return Ok(vec![]);
        };
        let near_body_of_water: Vec<Zone> = body_of_water
            .iter()
            .flat_map(|zone| zone.get_nearby_sites(state))
            .collect();
        let Some(picked_site_id) = CardQuery::new()
            .sites()
            .in_zones(&near_body_of_water)
            .with_prompt("Pick a site to flood")
            .with_source_card(*card_id)
            .pick(&controller_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };

        let picked_site = state.get_card(&picked_site_id);
        let mut effects = vec![Effect::SetCardData {
            card_id: *card_id,
            data: std::sync::Arc::new(OngoingEffect::GrantAbility {
                ability: Ability::Flooded,
                affected_cards: CardQuery::from_id(picked_site_id),
            }),
        }];

        effects.extend(
            CardQuery::new()
                .minions()
                .in_zone(picked_site.get_zone())
                .without_ability(Ability::Submerge)
                .all(state)
                .into_iter()
                .map(|card_id| Effect::SetTapped {
                    card_id,
                    tapped: true,
                }),
        );

        let tapped_minions = CardQuery::new()
            .minions()
            .in_zone(picked_site.get_zone())
            .without_ability(Ability::Submerge)
            .all(state);

        for tapped_minion_id in tapped_minions {
            effects.push(Effect::AddTemporaryEffect {
                effect: TemporaryEffect::ModifyEffect {
                    trigger_on_effect: EffectQuery::UntapCard {
                        card: CardQuery::from_id(tapped_minion_id),
                    },
                    expires_on_effect: EffectQuery::UntapCard {
                        card: CardQuery::from_id(tapped_minion_id),
                    },
                    on_effect: Arc::new(move |_state: &State, effect: &mut Effect| {
                        Box::pin(async move {
                            *effect = Effect::Noop;
                            Ok(())
                        })
                    }),
                },
            });
        }

        let _ = player_id;
        Ok(effects)
    }
}

#[derive(Debug, Clone)]
pub struct Waveshaper {
    card_base: CardBase,
    unit_base: UnitBase,
    avatar_base: AvatarBase,
    flood_effect: Option<OngoingEffect>,
}

impl Waveshaper {
    pub const NAME: &'static str = "Waveshaper";
    pub const DESCRIPTION: &'static str = "Tap → Play or draw a site.\r \r Tap → Flood a site near your body of water until you do so again. Tap minions without submerge there. They don't untap the next time they would.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 20,
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::ZERO,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            avatar_base: AvatarBase {
                ..Default::default()
            },
            flood_effect: None,
        }
    }
}

#[async_trait::async_trait]
impl Card for Waveshaper {
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

    fn get_avatar(&self) -> Option<&dyn Avatar> {
        Some(self)
    }

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(WaveshaperFlood)])
    }

    async fn get_continuous_effects(&self, _state: &State) -> anyhow::Result<Vec<OngoingEffect>> {
        match &self.flood_effect {
            Some(effect) => Ok(vec![effect.clone()]),
            None => Ok(vec![]),
        }
    }

    fn set_data(
        &mut self,
        data: &std::sync::Arc<dyn std::any::Any + Send + Sync>,
    ) -> anyhow::Result<()> {
        if let Some(effect) = data.downcast_ref::<OngoingEffect>() {
            self.flood_effect = Some(effect.clone());
        }

        Ok(())
    }
}

impl Avatar for Waveshaper {}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Waveshaper::NAME, |owner_id: PlayerId| {
    Box::new(Waveshaper::new(owner_id))
});
