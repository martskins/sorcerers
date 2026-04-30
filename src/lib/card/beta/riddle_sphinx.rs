use crate::{
    card::{Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct RiddleSphinx {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl RiddleSphinx {
    pub const NAME: &'static str = "Riddle Sphinx";
    pub const DESCRIPTION: &'static str = "Airborne. Genesis → draw a spell.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                abilities: vec![Ability::Airborne],
                types: vec![MinionType::Monster],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(6, "A"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for RiddleSphinx {
    fn get_name(&self) -> &str { Self::NAME }
    fn get_description(&self) -> &str { Self::DESCRIPTION }
    fn get_base_mut(&mut self) -> &mut CardBase { &mut self.card_base }
    fn get_base(&self) -> &CardBase { &self.card_base }
    fn get_unit_base(&self) -> Option<&UnitBase> { Some(&self.unit_base) }
    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> { Some(&mut self.unit_base) }

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        Ok(vec![Effect::DrawSpell { player_id: controller_id, count: 1 }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (RiddleSphinx::NAME, |owner_id: PlayerId| {
    Box::new(RiddleSphinx::new(owner_id))
});
