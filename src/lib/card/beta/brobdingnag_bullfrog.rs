use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct BrobdingnagBullfrog {
    unit_base: UnitBase,
    card_base: CardBase,
    swallowed_minion: Option<uuid::Uuid>,
}

impl BrobdingnagBullfrog {
    pub const NAME: &'static str = "Brobdingnag Bullfrog";
    pub const DESCRIPTION: &'static str = "Genesis → Brobdingnag Bullfrog swallows another target minion here. He carries it disabled in his belly until he leaves the realm.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![],
                types: vec![MinionType::Beast],
                tapped: false,
                ..Default::default()
            },
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
            swallowed_minion: None,
        }
    }
}

#[async_trait::async_trait]
impl Card for BrobdingnagBullfrog {
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

    fn deathrite(&self, _state: &State, _from: &Zone) -> Vec<Effect> {
        if let Some(swallowed_minion_id) = self.swallowed_minion {
            return vec![Effect::SetBearer {
                card_id: swallowed_minion_id,
                bearer_id: None,
            }];
        }

        vec![]
    }

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let minions = CardQuery::new()
            .minions()
            .in_zone(self.get_zone())
            .id_not(self.get_id())
            .all(state);
        if minions.is_empty() {
            return Ok(vec![]);
        }

        let picked_card = pick_card(
            self.get_controller_id(state),
            &minions,
            state,
            "Brobdingnag Bullfrog: Pick a minon to swallow",
        )
        .await?;

        Ok(vec![
            Effect::SetCardData {
                card_id: *self.get_id(),
                data: std::sync::Arc::new(picked_card),
            },
            Effect::SetBearer {
                card_id: picked_card,
                bearer_id: Some(*self.get_id()),
            },
        ])
    }

    fn set_data(
        &mut self,
        data: &std::sync::Arc<dyn std::any::Any + Send + Sync>,
    ) -> anyhow::Result<()> {
        if let Some(swallowed_minion_id) = data.downcast_ref::<uuid::Uuid>() {
            self.swallowed_minion = Some(*swallowed_minion_id);
        }

        Ok(())
    }

    async fn get_continuous_effects(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<ContinuousEffect>> {
        if let Some(swallowed_minion) = self.swallowed_minion {
            Ok(vec![ContinuousEffect::GrantAbility {
                ability: Ability::Disabled,
                affected_cards: swallowed_minion.into(),
            }])
        } else {
            Ok(vec![])
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (BrobdingnagBullfrog::NAME, |owner_id: PlayerId| {
        Box::new(BrobdingnagBullfrog::new(owner_id))
    });
