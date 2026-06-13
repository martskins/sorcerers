use crate::prelude::*;

const KILL_MINION_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct GrimReaper {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl GrimReaper {
    pub const NAME: &'static str = "Grim Reaper";
    pub const DESCRIPTION: &'static str = "Lethal\r \r Whenever Grim Reaper kills a minion, banish that minion and all copies. Search its owner's cemetery, hand, and spellbook and banish any copies. They shuffle.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                abilities: vec![Ability::Lethal],
                types: vec![MinionType::Spirit],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "AA"),
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for GrimReaper {
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

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook {
            id: KILL_MINION_HOOK,
            trigger: EffectQuery::UnitKilled {
                unit: CardQuery::new().minions(),
                killer: Some(self.get_id().into()),
                from_attack: None,
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
            KILL_MINION_HOOK => {
                let Effect::KillMinion { card_id, .. } = effect else {
                    return Ok(vec![]);
                };

                // Get the name of the buried card to banish all copies.
                let buried_name = state.get_card(card_id).get_name().to_string();
                let buried_owner_id = state.get_card(card_id).get_owner_id();

                // Banish the killed minion.
                let mut effects = vec![Effect::BanishCard { card_id: *card_id }];

                // Find all copies of that card (by name) in any zone belonging to its owner.
                let copies_in_play = CardQuery::new()
                    .minions()
                    .named(buried_name.clone())
                    .all(state);
                let copies_owned_by_owner = CardQuery::new()
                    .minions()
                    .owned_by(buried_owner_id)
                    .in_zones(&[Zone::Cemetery, Zone::Hand, Zone::Spellbook])
                    .named(buried_name)
                    .all(state);

                let mut all_copies = copies_in_play;
                all_copies.extend(&copies_owned_by_owner);
                for copy_id in all_copies {
                    effects.push(Effect::BanishCard { card_id: copy_id });
                }

                effects.push(Effect::ShuffleDeck {
                    player_id: *buried_owner_id,
                });

                Ok(effects)
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (GrimReaper::NAME, |owner_id: PlayerId| {
    Box::new(GrimReaper::new(owner_id))
});
