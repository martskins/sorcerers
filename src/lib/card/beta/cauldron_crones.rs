use crate::{
    card::{Ability, Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::{PlayerId, yes_or_no},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct CauldronCrones {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl CauldronCrones {
    pub const NAME: &'static str = "Cauldron Crones";
    pub const DESCRIPTION: &'static str =
        "Spellcaster\r \r Genesis → You may sacrifice another minion here to draw a spell.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                abilities: vec![Ability::Spellcaster(None)],
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "F"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for CauldronCrones {
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
        let sacrifice = yes_or_no(
            &controller_id,
            state,
            "Cauldron Crones: Sacrifice a minion here to draw a spell?",
        )
        .await?;

        if !sacrifice {
            return Ok(vec![]);
        }

        let Some(picked) = CardQuery::new()
            .minions()
            .controlled_by(&controller_id)
            .in_zone(self.get_zone())
            .id_not_in(vec![self.get_id().clone()])
            .with_prompt("Cauldron Crones: Choose a minion to sacrifice")
            .pick(&controller_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };
        Ok(vec![
            Effect::BuryCard { card_id: picked },
            Effect::DrawSpell {
                player_id: controller_id,
                count: 1,
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (CauldronCrones::NAME, |owner_id: PlayerId| {
        Box::new(CauldronCrones::new(owner_id))
    });
