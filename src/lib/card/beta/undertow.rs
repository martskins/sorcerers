use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Undertow {
    site_base: SiteBase,
    card_base: CardBase,
}

impl Undertow {
    pub const NAME: &'static str = "Undertow";
    pub const DESCRIPTION: &'static str =
        "Genesis → Staying within this body of water, move target unit one step.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("W"),
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
        }
    }
}

#[async_trait::async_trait]
impl Site for Undertow {}

impl ResourceProvider for Undertow {}

#[async_trait::async_trait]
impl Card for Undertow {
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

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook::genesis(self.get_id())])
    }

    async fn resolve_hook(
        &self,
        hook: HookId,
        state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            GENESIS_HOOK_ID => {
                let player_id = self.get_controller_id(state);
                let body_of_water = state
                    .get_body_of_water_at(self.get_location())
                    .ok_or(anyhow::anyhow!("Undertow must be in a body of water"))?;
                let controller_id = self.get_controller_id(state);
                let Some(unit_id) = CardQuery::new()
                    .units()
                    .with_prompt("Choose a unit in the same body of water to move")
                    .with_source_card(*self.get_id())
                    .in_locations(&body_of_water)
                    .pick(&controller_id, state, false)
                    .await?
                else {
                    return Ok(vec![]);
                };
                let unit = state.get_card(&unit_id);
                let zones = unit.get_locations_within_steps(state, 1);
                let picked_zone = pick_location(
                    player_id,
                    &zones,
                    state,
                    false,
                    "Undertow: Choose a zone to move the unit to",
                )
                .await?;
                Ok(vec![Effect::MoveCard {
                    card_id: unit_id,
                    to: LocationQuery::from_location(
                        (picked_zone).with_region(unit.get_region(state).clone()),
                    ),
                    player_id,
                    from: (unit.get_zone().clone())
                        .into_location()
                        .expect("MoveCard source must be a location"),
                    tap: false,
                    through_path: None,
                }])
            }
            _ => Ok(vec![]),
        }
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Undertow::NAME, |owner_id: PlayerId| {
    Box::new(Undertow::new(owner_id))
});
