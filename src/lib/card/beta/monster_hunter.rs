use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone,
    },
    effect::Effect,
    game::{PlayerId, pick_card},
    state::State,
};

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
                region: Region::Surface,
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

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let my_zone = self.get_zone().clone();

        let nearby_monsters: Vec<uuid::Uuid> = state
            .cards
            .iter()
            .filter(|c| c.is_minion())
            .filter(|c| c.get_controller_id(state) != controller_id)
            .filter(|c| c.get_zone().is_nearby(&my_zone))
            .filter(|c| {
                c.get_unit_base()
                    .map(|ub| ub.types.contains(&MinionType::Monster))
                    .unwrap_or(false)
            })
            .map(|c| *c.get_id())
            .collect();

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
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (MonsterHunter::NAME, |owner_id: PlayerId| {
        Box::new(MonsterHunter::new(owner_id))
    });
