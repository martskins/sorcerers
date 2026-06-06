use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct DeathDealer {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl DeathDealer {
    pub const NAME: &'static str = "Death Dealer";
    pub const DESCRIPTION: &'static str = "Genesis → Kill all other minions.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 2,
                abilities: vec![],
                types: vec![MinionType::Mortal],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(7, "FF"),
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
impl Card for DeathDealer {
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

    async fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
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
        let self_id = *self.get_id();

        Ok(CardQuery::new()
            .minions()
            .in_play()
            .id_not_in(vec![self_id])
            .all(state)
            .into_iter()
            .map(|id| Effect::KillMinion {
                card_id: id,
                killer_id: *self.get_id(),
                from_attack: false,
            })
            .collect())
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (DeathDealer::NAME, |owner_id: PlayerId| {
    Box::new(DeathDealer::new(owner_id))
});
