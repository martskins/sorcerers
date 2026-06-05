use crate::prelude::*;

const ON_SUMMON_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct ScourgeZombies {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl ScourgeZombies {
    pub const NAME: &'static str = "Scourge Zombies";
    pub const DESCRIPTION: &'static str = "Whenever an allied Mortal dies on land, you may summon Scourge Zombies from your cemetery to its location, tapped.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                types: vec![MinionType::Undead],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "E"),
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
impl Card for ScourgeZombies {
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
        let controller_id = self.get_controller_id(state);
        Ok(vec![Hook {
            id: ON_SUMMON_HOOK,
            trigger: EffectQuery::UnitKilled {
                unit: CardQuery::new()
                    .minions()
                    .minion_type(&MinionType::Mortal)
                    .controlled_by(&controller_id),
                killer: None,
            },
            timing: HookTiming::After,
            source_zones: HookSourceZones::Zone(Zone::Cemetery),
        }])
    }

    async fn resolve_hook(
        &self,
        hook_id: HookId,
        state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            ON_SUMMON_HOOK => {
                let self_id = *self.get_id();
                let Effect::KillMinion { card_id, .. } = effect else {
                    return Ok(vec![]);
                };

                let self_controller = self.get_controller_id(state);
                let killed_card = state.get_card(card_id);
                if killed_card.get_controller_id(state) != self_controller
                    || killed_card.get_region(state) != &Region::Surface
                    || !killed_card.get_zone().is_in_play()
                {
                    return Ok(vec![]);
                }

                if !yes_or_no_source(
                    &self_controller,
                    state,
                    "Summon Scourge Zombies to the fallen Mortal's location tapped?",
                    Some(self_id),
                )
                .await?
                {
                    return Ok(vec![]);
                }

                Ok(vec![
                    Effect::SummonCards {
                        cards: vec![(
                            self_controller,
                            self_id,
                            Zone::Cemetery,
                            killed_card
                                .get_zone()
                                .clone()
                                .into_location()
                                .expect("Scourge Zombies trigger must have a location"),
                        )],
                    },
                    Effect::SetTapped {
                        card_id: self_id,
                        tapped: true,
                    },
                ])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (ScourgeZombies::NAME, |owner_id: PlayerId| {
        Box::new(ScourgeZombies::new(owner_id))
    });
