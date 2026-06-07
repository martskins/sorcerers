use crate::prelude::*;

const ENTER_WATER_SITE_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct MarinersCurse {
    aura_base: AuraBase,
    card_base: CardBase,
}

impl MarinersCurse {
    pub const NAME: &'static str = "Mariner's Curse";
    pub const DESCRIPTION: &'static str = "When a minion enters an affected water site, it submerges. Then return Mariner's Curse to your hand.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "WW"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            aura_base: AuraBase { tapped: false },
        }
    }
}

impl Aura for MarinersCurse {}

#[async_trait::async_trait]
impl Card for MarinersCurse {
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

    async fn hooks(&self, state: &State) -> anyhow::Result<Vec<Hook>> {
        let mut affected_water_sites = self.get_affected_zones(state);
        affected_water_sites.retain(|z| match z.get_site(state) {
            Some(site) => site.is_water_site(state).unwrap_or_default(),
            None => false,
        });

        Ok(vec![Hook {
            id: ENTER_WATER_SITE_HOOK,
            trigger: EffectQuery::EnterZone {
                card: CardQuery::new().minions(),
                zone: ZoneQuery::from_options(affected_water_sites, None),
                from: None,
            },
            timing: HookTiming::After,
            source_zones: HookSourceZones::InPlay,
        }])
    }

    async fn resolve_hook(
        &self,
        hook_id: HookId,
        _state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            ENTER_WATER_SITE_HOOK => {
                let card_id = match effect {
                    Effect::SummonCards { cards } => {
                        let mut output = None;
                        for (_, card_id, _, loc) in cards {
                            if loc.square() == self.get_zone().get_square()
                                && loc.region() == &Region::Surface
                            {
                                output = Some(card_id);
                            }
                        }

                        match output {
                            Some(card_id) => card_id,
                            None => return Ok(vec![]),
                        }
                    }
                    Effect::MoveCard { card_id, .. } => card_id,
                    _ => return Ok(vec![]),
                };

                Ok(vec![
                    Effect::SetCardRegion {
                        card_id: *card_id,
                        destination: Region::Underwater,
                        tap: false,
                    },
                    Effect::SetCardZone {
                        card_id: *self.get_id(),
                        zone: Zone::Hand,
                    },
                ])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (MarinersCurse::NAME, |owner_id: PlayerId| {
        Box::new(MarinersCurse::new(owner_id))
    });
