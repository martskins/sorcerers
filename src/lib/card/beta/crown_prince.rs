use crate::{
    card::{Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct CrownPrince {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl CrownPrince {
    pub const NAME: &'static str = "Crown Prince";
    pub const DESCRIPTION: &'static str =
        "Deathrite → If you control another Mortal, return Crown Prince to its owner's hand.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![],
                types: vec![MinionType::Mortal],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "EE"),
                region: Region::Surface,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for CrownPrince {
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

    fn deathrite(&self, state: &State, _from: &Zone) -> Vec<Effect> {
        let controller_id = self.get_controller_id(state);
        let self_id = self.get_id().clone();

        let other_mortal_exists = CardQuery::new()
            .controlled_by(&controller_id)
            .minions()
            .in_play()
            .id_not_in(vec![self_id.clone()])
            .all(state)
            .into_iter()
            .any(|id| {
                state
                    .get_card(&id)
                    .get_unit_base()
                    .map_or(false, |ub| ub.types.contains(&MinionType::Mortal))
            });

        if other_mortal_exists {
            vec![Effect::SetCardZone {
                card_id: self_id,
                zone: Zone::Hand,
            }]
        } else {
            vec![]
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (CrownPrince::NAME, |owner_id: PlayerId| {
        Box::new(CrownPrince::new(owner_id))
    });
