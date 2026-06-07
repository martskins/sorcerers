use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct AtlanteanFate {
    aura_base: AuraBase,
    card_base: CardBase,
}

impl AtlanteanFate {
    pub const NAME: &'static str = "Atlantean Fate";
    pub const DESCRIPTION: &'static str = "Affected non-Ordinary sites are flooded. They are water sites, only provide Water threshold, and lose all other abilities.\r \r Genesis → Submerge all minions and artifacts atop affected sites.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "WW"),
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            aura_base: AuraBase { tapped: false },
        }
    }

    fn flooded_site_ids(&self, state: &State) -> Vec<CardId> {
        let affected_zones = self.get_affected_zones(state);
        state
            .cards
            .values()
            .filter(|c| affected_zones.contains(c.get_zone()))
            .filter(|c| c.is_site())
            .filter(|c| c.get_zone().is_in_play())
            .filter(|c| c.get_base().rarity != Rarity::Ordinary)
            .map(|c| *c.get_id())
            .collect()
    }
}

impl Aura for AtlanteanFate {}

#[async_trait::async_trait]
impl Card for AtlanteanFate {
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

    fn get_aura_base(&self) -> Option<&AuraBase> {
        Some(&self.aura_base)
    }
    fn get_aura_base_mut(&mut self) -> Option<&mut AuraBase> {
        Some(&mut self.aura_base)
    }

    fn get_aura(&self) -> Option<&dyn Aura> {
        Some(self)
    }

    async fn get_continuous_effects(&self, state: &State) -> anyhow::Result<Vec<OngoingEffect>> {
        let flooded_sites = self.flooded_site_ids(state);
        if flooded_sites.is_empty() {
            return Ok(vec![]);
        }

        Ok(vec![
            OngoingEffect::GrantAbility {
                ability: Ability::Flooded,
                affected_cards: CardQuery::from_ids(flooded_sites.clone()),
            },
            OngoingEffect::ModifyProvidedAffinities {
                modifier: AffinityModifier::Set(Thresholds::parse("W")),
                affected_sites: CardQuery::from_ids(flooded_sites.clone()),
            },
            OngoingEffect::RemoveAbilities {
                removal: AbilityRemoval::AllAbilitiesExcept(vec![Ability::Flooded]),
                affected_cards: CardQuery::from_ids(flooded_sites),
            },
        ])
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
                let affected_zones: Vec<Zone> = self
                    .get_affected_zones(state)
                    .into_iter()
                    .filter(|z| z.get_site(state).is_some())
                    .collect();
                Ok(CardQuery::new()
                    .card_types(vec![CardType::Minion, CardType::Artifact])
                    .in_zones(&affected_zones)
                    .all(state)
                    .into_iter()
                    .map(|id| Effect::SetCardRegion {
                        card_id: id,
                        destination: Region::Underwater,
                        tap: false,
                    })
                    .collect())
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (AtlanteanFate::NAME, |owner_id: PlayerId| {
        Box::new(AtlanteanFate::new(owner_id))
    });
