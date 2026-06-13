use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct VileImp {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl VileImp {
    pub const NAME: &'static str = "Vile Imp";
    pub const DESCRIPTION: &'static str = "Genesis → May deal 2 damage to target adjacent unit.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                abilities: vec![],
                types: vec![MinionType::Demon],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "F"),
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
impl Card for VileImp {
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
                let imp_id = *self.get_id();

                let targets = CardQuery::new()
                    .units()
                    .adjacent_to(self.get_location())
                    .id_not_in(vec![imp_id])
                    .all(state);
                if targets.is_empty() {
                    return Ok(vec![]);
                }

                let use_genesis = yes_or_no_source(
                    &controller_id,
                    state,
                    "Deal 2 damage to an adjacent unit?",
                    Some(imp_id),
                )
                .await?;
                if !use_genesis {
                    return Ok(vec![]);
                };

                let target_id = pick_card_source(
                    &controller_id,
                    &targets,
                    state,
                    "Vile Imp: Pick an adjacent unit",
                    Some(*self.get_id()),
                )
                .await?;

                Ok(vec![Effect::TakeDamage {
                    card_id: target_id,
                    from: imp_id,
                    damage: Damage::basic(2),
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (VileImp::NAME, |owner_id: PlayerId| {
    Box::new(VileImp::new(owner_id))
});
