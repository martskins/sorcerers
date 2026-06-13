use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Battlefield {
    site_base: SiteBase,
    card_base: CardBase,
}

impl Battlefield {
    pub const NAME: &'static str = "Battlefield";
    pub const DESCRIPTION: &'static str = "Genesis → Conjure a broken Weapon or Armor here.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 0,
                provided_thresholds: Thresholds::ZERO,
                types: vec![],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Site for Battlefield {}

impl ResourceProvider for Battlefield {}

#[async_trait::async_trait]
impl Card for Battlefield {
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
                let controller_id = self.get_controller_id(state);
                let Some(picked_card_id) = CardQuery::new()
                    .in_zone(&Zone::Cemetery)
                    .artifact_types(vec![ArtifactType::Weapon, ArtifactType::Armor])
                    .with_prompt("Pick a weapon or armor to conjure")
                    .with_source_card(*self.get_id())
                    .pick(&controller_id, state, true)
                    .await?
                else {
                    return Ok(vec![]);
                };

                Ok(vec![Effect::SummonCards {
                    summoned_cards: vec![SummonCard {
                        player_id: controller_id,
                        card_id: picked_card_id,
                        from_zone: Zone::Cemetery,
                        to_location: self
                            .get_zone()
                            .clone()
                            .location().cloned()
                            .expect("Battlefield must be in a location"),
                    }],
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Battlefield::NAME, |owner_id: PlayerId| {
    Box::new(Battlefield::new(owner_id))
});
