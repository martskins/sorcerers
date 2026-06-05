use crate::prelude::*;

const ON_CARD_STOP_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct VaultsOfZul {
    site_base: SiteBase,
    card_base: CardBase,
    triggered: bool,
}

impl VaultsOfZul {
    pub const NAME: &'static str = "Vaults of Zul";
    pub const DESCRIPTION: &'static str =
        "The first time an Avatar stops here, they draw three cards and skip their next turn.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::ZERO,
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            triggered: false,
        }
    }
}

#[async_trait::async_trait]
impl Site for VaultsOfZul {}

impl ResourceProvider for VaultsOfZul {}

#[async_trait::async_trait]
impl Card for VaultsOfZul {
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

    async fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        if self.triggered || !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        Ok(vec![Hook {
            id: ON_CARD_STOP_HOOK,
            trigger: EffectQuery::StopAtZone {
                card: CardQuery::new().avatars(),
                zone: ZoneQuery::from_zone(self.get_zone().clone()),
            },
            timing: HookTiming::After,
            source_zones: HookSourceZones::InPlay,
        }])
    }

    async fn resolve_hook(
        &self,
        hook_id: HookId,
        state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            ON_CARD_STOP_HOOK => {
                if self.triggered {
                    return Ok(vec![]);
                }

                let stopped_avatar_id = match effect {
                    Effect::MoveCard { card_id, .. } if state.get_card(card_id).is_avatar() => {
                        *card_id
                    }
                    Effect::MoveCard { card_id, .. } => CardQuery::new()
                        .carried_by(card_id)
                        .avatars()
                        .first(state)
                        .unwrap_or(*card_id),
                    _ => return Ok(vec![]),
                };
                let stopped_avatar = state.get_card(&stopped_avatar_id);
                if !stopped_avatar.is_avatar()
                    || stopped_avatar
                        .get_zone()
                        .get_site_at_square(state)
                        .map(|site| site.get_id())
                        != Some(self.get_id())
                {
                    return Ok(vec![]);
                }

                let controller_id = stopped_avatar.get_controller_id(state);
                Ok(vec![
                    Effect::SetCardData {
                        card_id: *self.get_id(),
                        data: std::sync::Arc::new(true),
                    },
                    Effect::DrawCard {
                        player_id: controller_id,
                        count: 3,
                        kind: DrawKind::Choice,
                    },
                    Effect::SkipNextTurn {
                        player_id: controller_id,
                    },
                ])
            }
            _ => Ok(vec![]),
        }
    }

    fn set_data(
        &mut self,
        data: &std::sync::Arc<dyn std::any::Any + Send + Sync>,
    ) -> anyhow::Result<()> {
        if let Some(triggered) = data.downcast_ref::<bool>() {
            self.triggered = *triggered;
        }

        Ok(())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (VaultsOfZul::NAME, |owner_id: PlayerId| {
    Box::new(VaultsOfZul::new(owner_id))
});
