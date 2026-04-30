use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    effect::Effect,
    game::{PlayerId, pick_card},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct HighlandFalconer {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl HighlandFalconer {
    pub const NAME: &'static str = "Highland Falconer";
    pub const DESCRIPTION: &'static str = "Genesis → Search your spellbook for an Airborne Beast with a mana cost of ≤ 2 and summon it.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "A"),
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
impl Card for HighlandFalconer {
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

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);

        let targets: Vec<uuid::Uuid> = CardQuery::new()
            .minions()
            .in_zone(&Zone::Spellbook)
            .controlled_by(&controller_id)
            .with_abilities(vec![Ability::Airborne])
            .minion_type(&crate::card::MinionType::Beast)
            .all(state)
            .into_iter()
            .filter(|id| {
                state
                    .get_card(id)
                    .get_costs(state)
                    .map(|c| c.mana_value())
                    .unwrap_or(u8::MAX)
                    <= 2
            })
            .collect();

        if targets.is_empty() {
            return Ok(vec![]);
        }

        let chosen = pick_card(
            &controller_id,
            &targets,
            state,
            "Summon an Airborne Beast (≤2)",
        )
        .await?;
        Ok(vec![Effect::SummonCard {
            card_id: chosen,
            zone: self.get_zone().clone(),
            player_id: controller_id,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (HighlandFalconer::NAME, |owner_id: PlayerId| {
        Box::new(HighlandFalconer::new(owner_id))
    });
