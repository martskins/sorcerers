use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone,
    },
    effect::Effect,
    game::{PlayerId, pick_card},
    state::State,
};

#[derive(Debug, Clone)]
pub struct HighlandPrincess {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl HighlandPrincess {
    pub const NAME: &'static str = "Highland Princess";
    pub const DESCRIPTION: &'static str = "Genesis → Search your spellbook for an artifact with a mana cost of ≤ 1 and add it to your hand.";

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
                costs: Costs::basic(2, "AA"),
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
impl Card for HighlandPrincess {
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

        let targets: Vec<uuid::Uuid> = state
            .cards
            .iter()
            .filter(|c| c.is_artifact())
            .filter(|c| c.get_zone() == &Zone::Spellbook)
            .filter(|c| c.get_controller_id(state) == controller_id)
            .filter(|c| {
                c.get_costs(state)
                    .map(|costs| costs.mana_value())
                    .unwrap_or(u8::MAX)
                    <= 1
            })
            .map(|c| *c.get_id())
            .collect();

        if targets.is_empty() {
            return Ok(vec![]);
        }

        let chosen = pick_card(
            &controller_id,
            &targets,
            state,
            "Search for artifact (≤1 mana)",
        )
        .await?;
        Ok(vec![Effect::SetCardZone {
            card_id: chosen,
            zone: Zone::Hand,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (HighlandPrincess::NAME, |owner_id: PlayerId| {
        Box::new(HighlandPrincess::new(owner_id))
    });
