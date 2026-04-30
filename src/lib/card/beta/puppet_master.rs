use crate::{
    card::{Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct PuppetMaster {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl PuppetMaster {
    pub const NAME: &'static str = "Puppet Master";
    pub const DESCRIPTION: &'static str = "Airborne. Genesis → gain control of all tapped minions here.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                abilities: vec![Ability::Airborne],
                types: vec![MinionType::Spirit],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(6, "AA"),
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
impl Card for PuppetMaster {
    fn get_name(&self) -> &str { Self::NAME }
    fn get_description(&self) -> &str { Self::DESCRIPTION }
    fn get_base_mut(&mut self) -> &mut CardBase { &mut self.card_base }
    fn get_base(&self) -> &CardBase { &self.card_base }
    fn get_unit_base(&self) -> Option<&UnitBase> { Some(&self.unit_base) }
    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> { Some(&mut self.unit_base) }

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let self_id = *self.get_id();
        let effects = CardQuery::new()
            .units()
            .tapped()
            .in_zone(self.get_zone())
            .id_not(&self_id)
            .all(state)
            .into_iter()
            .map(|unit_id| Effect::SetController {
                card_id: unit_id,
                player_id: controller_id,
            })
            .collect();
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (PuppetMaster::NAME, |owner_id: PlayerId| {
    Box::new(PuppetMaster::new(owner_id))
});
