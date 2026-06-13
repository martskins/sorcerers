use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct MonsterHunter {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl MonsterHunter {
    pub const NAME: &'static str = "Monster Hunter";
    pub const DESCRIPTION: &'static str = "Genesis → Kill a nearby Monster.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                types: vec![MinionType::Mortal],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "E"),
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
impl Card for MonsterHunter {
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
                let my_location = self.get_location().clone();

                let nearby_monsters = CardQuery::new()
                    .minions()
                    .controlled_by(&controller_id)
                    .nearby_to_card(self.get_id())
                    .minion_type(&MinionType::Monster)
                    .all(state);
                if nearby_monsters.is_empty() {
                    return Ok(vec![]);
                }

                let chosen = pick_card(
                    &controller_id,
                    &nearby_monsters,
                    state,
                    "Monster Hunter: Pick a nearby Monster to kill",
                )
                .await?;

                Ok(vec![Effect::KillMinion {
                    card_id: chosen,
                    killer_id: *self.get_id(),
                    from_attack: false,
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (MonsterHunter::NAME, |owner_id: PlayerId| {
        Box::new(MonsterHunter::new(owner_id))
    });
