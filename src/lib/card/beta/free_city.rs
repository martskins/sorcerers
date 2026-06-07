use crate::prelude::*;

#[derive(Debug, Clone)]
struct FreeCityAttack;

#[async_trait::async_trait]
impl ActivatedAbility for FreeCityAttack {
    fn get_name(&self) -> String {
        "Attack or defend against enemies here".to_string()
    }

    fn get_cost(&self, _card_id: &CardId, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::ZERO.clone())
    }

    fn can_activate(
        &self,
        card_id: &CardId,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<bool> {
        let card = state.get_card(card_id);
        let enemies_here = CardQuery::new()
            .units()
            .in_zone(card.get_zone())
            .all(state)
            .into_iter()
            .filter(|id| &state.get_card(id).get_controller_id(state) != player_id)
            .count();
        Ok(enemies_here > 0)
    }

    async fn on_select(
        &self,
        card_id: &CardId,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let card = state.get_card(card_id);
        let enemies_here: Vec<CardId> = CardQuery::new()
            .units()
            .in_zone(card.get_zone())
            .all(state)
            .into_iter()
            .filter(|id| &state.get_card(id).get_controller_id(state) != player_id)
            .collect();

        if enemies_here.is_empty() {
            return Ok(vec![]);
        }

        let target = pick_card(
            player_id,
            &enemies_here,
            state,
            "Free City: pick an enemy to attack",
        )
        .await?;

        Ok(vec![
            Effect::DeclareAttack {
                attacker_id: *card_id,
                target_id: target,
            },
            Effect::SetCardData {
                card_id: *card_id,
                data: std::sync::Arc::new(true),
            },
            Effect::Animate {
                card_id: *card_id,
                unit_base: FreeCity::animated_unit_base(),
                expires_on_effect: EffectQuery::TurnStart {
                    player_id: Some(*player_id),
                },
            },
        ])
    }
}

#[derive(Debug, Clone)]
pub struct FreeCity {
    site_base: SiteBase,
    card_base: CardBase,
    used_ability: bool,
}

impl FreeCity {
    pub const NAME: &'static str = "Free City";
    pub const DESCRIPTION: &'static str =
        "Once per turn, may attack or defend against enemy units here.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 3,
                provided_thresholds: Thresholds::new(),
                types: vec![],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            used_ability: false,
        }
    }

    fn animated_unit_base() -> UnitBase {
        UnitBase {
            power: 3,
            toughness: 3,
            tapped: false,
            ..Default::default()
        }
    }
}

#[async_trait::async_trait]
impl Site for FreeCity {}

impl ResourceProvider for FreeCity {}

const TURN_START_HOOK: HookId = 1;
const DEFEND_DECLARED_HOOK: HookId = 2;

#[async_trait::async_trait]
impl Card for FreeCity {
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

    fn get_site_base(&self) -> Option<&SiteBase> {
        Some(&self.site_base)
    }

    fn get_site_base_mut(&mut self) -> Option<&mut SiteBase> {
        Some(&mut self.site_base)
    }

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }

    fn set_data(
        &mut self,
        data: &std::sync::Arc<dyn std::any::Any + Send + Sync>,
    ) -> anyhow::Result<()> {
        if let Some(site_data) = data.downcast_ref::<bool>() {
            self.used_ability = *site_data;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid data type for FreeCity"))
        }
    }

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![
            Hook {
                id: TURN_START_HOOK,
                trigger: EffectQuery::TurnStart { player_id: None },
                timing: HookTiming::After,
                source_zones: HookSourceZones::InPlay,
            },
            Hook {
                id: DEFEND_DECLARED_HOOK,
                trigger: EffectQuery::DefendDeclared {
                    attacker: CardQuery::new(),
                    defender: self.get_id().into(),
                },
                timing: HookTiming::After,
                source_zones: HookSourceZones::InPlay,
            },
        ])
    }

    async fn resolve_hook(
        &self,
        hook: HookId,
        state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            TURN_START_HOOK => Ok(vec![Effect::SetCardData {
                card_id: *self.get_id(),
                data: std::sync::Arc::new(false),
            }]),
            DEFEND_DECLARED_HOOK => Ok(vec![
                Effect::SetCardData {
                    card_id: *self.get_id(),
                    data: std::sync::Arc::new(true),
                },
                Effect::Animate {
                    card_id: *self.get_id(),
                    unit_base: Self::animated_unit_base(),
                    expires_on_effect: EffectQuery::TurnStart {
                        player_id: Some(self.get_controller_id(state)),
                    },
                },
            ]),
            _ => Ok(vec![]),
        }
    }

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        if self.used_ability {
            return Ok(vec![]);
        }

        Ok(vec![Box::new(FreeCityAttack)])
    }

    fn can_defend_attack(
        &self,
        state: &State,
        attacker_id: &CardId,
        _defender_id: &CardId,
    ) -> bool {
        if self.used_ability
            || self.is_tapped()
            || self.has_status(state, &CardStatus::Disabled)
            || self.has_ability(state, &Ability::CannotDefend)
        {
            return false;
        }

        let attacker = state.get_card(attacker_id);
        state.is_unit_card(attacker_id)
            && attacker.get_controller_id(state) != self.get_controller_id(state)
            && attacker.get_zone() == self.get_zone()
    }

    fn area_modifiers(&self, _state: &State) -> Vec<OngoingEffect> {
        if !self.used_ability {
            return vec![];
        }

        vec![OngoingEffect::GrantAbility {
            ability: Ability::CannotDefend,
            affected_cards: CardQuery::from_id(*self.get_id()),
        }]
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (FreeCity::NAME, |owner_id: PlayerId| {
    Box::new(FreeCity::new(owner_id))
});
