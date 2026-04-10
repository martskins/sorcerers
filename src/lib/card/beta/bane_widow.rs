use crate::{
    card::{Card, CardBase, CardType, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::{PlayerId, pick_card},
    state::{CardMatcher, State},
};

#[derive(Debug, Clone)]
pub struct BaneWidow {
    pub card_base: CardBase,
    pub unit_base: UnitBase,
}

impl BaneWidow {
    pub const NAME: &'static str = "Bane Widow";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                types: vec![MinionType::Beast],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "FF"),
                region: Region::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for BaneWidow {
    fn get_name(&self) -> &str {
        Self::NAME
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
        let targets = CardMatcher::new()
            .with_card_type(CardType::Minion)
            .in_zone(self.get_zone())
            .with_id_not_in(vec![self.get_id().clone()])
            .resolve_ids(state);
        if targets.is_empty() {
            return Ok(vec![]);
        }

        let picked = pick_card(&controller_id, &targets, state, "Bane Widow: Pick a minion to kill").await?;

        Ok(vec![Effect::BuryCard { card_id: picked }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (BaneWidow::NAME, |owner_id: PlayerId| Box::new(BaneWidow::new(owner_id)));
