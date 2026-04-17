use crate::{
    card::{Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::{PlayerId, pick_card, yes_or_no},
    query::ZoneQuery,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct FeyChangeling {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl FeyChangeling {
    pub const NAME: &'static str = "Fey Changeling";
    pub const DESCRIPTION: &'static str = "May be summoned to any site.\r \r Genesis → You may return a minion here to its owner's hand.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 2,
                abilities: vec![],
                types: vec![MinionType::Fairy],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "W"),
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
impl Card for FeyChangeling {
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

    fn get_valid_play_zones(&self, state: &State) -> anyhow::Result<Vec<Zone>> {
        Ok(CardQuery::new()
            .sites()
            .in_play()
            .all(state)
            .into_iter()
            .map(|cid| state.get_card(&cid).get_zone().clone())
            .collect())
    }

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let minions_here = CardQuery::new()
            .units()
            .in_zone(self.get_zone())
            .id_not_in(vec![*self.get_id()])
            .all(state);

        if minions_here.is_empty() {
            return Ok(vec![]);
        }

        let want = yes_or_no(
            &controller_id,
            state,
            "Fey Changeling Genesis: Return a minion here to its owner's hand?",
        )
        .await?;
        if !want {
            return Ok(vec![]);
        }

        let target_id = pick_card(
            &controller_id,
            &minions_here,
            state,
            "Fey Changeling Genesis: Pick a minion to return to hand",
        )
        .await?;
        let target = state.get_card(&target_id);
        Ok(vec![Effect::MoveCard {
            player_id: *target.get_owner_id(),
            card_id: target_id,
            from: target.get_zone().clone(),
            to: ZoneQuery::from_zone(Zone::Hand),
            tap: false,
            region: Region::Surface,
            through_path: None,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (FeyChangeling::NAME, |owner_id: PlayerId| {
        Box::new(FeyChangeling::new(owner_id))
    });
