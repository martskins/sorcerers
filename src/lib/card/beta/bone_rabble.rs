use crate::prelude::*;

const PLAY_EARTH_SITE_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct BoneRabble {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl BoneRabble {
    pub const NAME: &'static str = "Bone Rabble";
    pub const DESCRIPTION: &'static str = "Whenever you play an earth site, you may summon Bone Rabble from your cemetery to that site.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                abilities: vec![Ability::Airborne],
                types: vec![MinionType::Beast],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "E"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for BoneRabble {
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

    async fn hooks(&self, state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook {
            id: PLAY_EARTH_SITE_HOOK,
            trigger: EffectQuery::PlayCard {
                card: CardQuery::new()
                    .sites()
                    .controlled_by(&self.get_controller_id(state)),
                spellcaster: None,
            },
            timing: HookTiming::After,
            source_zones: HookSourceZones::Zone(Zone::Cemetery),
        }])
    }

    async fn resolve_hook(
        &self,
        hook: HookId,
        state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            PLAY_EARTH_SITE_HOOK => {
                let Effect::PlayCard { card_id, .. } = effect else {
                    return Ok(vec![]);
                };

                let owner_id = self.get_owner_id();
                let summon_bone_rabble = yes_or_no_source(
                    &owner_id,
                    state,
                    "Summon Bone Rabble atop the played site?",
                    Some(*self.get_id()),
                )
                .await?;
                if summon_bone_rabble {
                    let site = state.get_card(card_id);
                    Ok(vec![Effect::SummonCards {
                        summoned_cards: vec![SummonCard {
                            player_id: *owner_id,
                            card_id: *self.get_id(),
                            from_zone: Zone::Cemetery,
                            to_location: site
                                .get_zone()
                                .clone()
                                .into_location()
                                .expect("played site must be a location"),
                        }],
                    }])
                } else {
                    Ok(vec![])
                }
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (BoneRabble::NAME, |owner_id: PlayerId| {
    Box::new(BoneRabble::new(owner_id))
});
