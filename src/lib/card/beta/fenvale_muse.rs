use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        SiteType, UnitBase, Zone,
    },
    effect::Effect,
    game::{PlayerId, pick_card, yes_or_no},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct FenvaleMuse {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl FenvaleMuse {
    pub const NAME: &'static str = "Fenvale Muse";
    pub const DESCRIPTION: &'static str = "Spellcaster\r \r Whenever Fenvale Muse casts a spell, you may trigger the Genesis of a nearby River.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 0,
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
                costs: Costs::basic(1, "W"),
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
impl Card for FenvaleMuse {
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

    async fn on_cast_spell(
        &self,
        state: &State,
        _spell_id: &uuid::Uuid,
    ) -> anyhow::Result<Vec<Effect>> {
        let nearby_rivers = CardQuery::new()
            .sites()
            .near_to(self.get_zone())
            .site_types(vec![SiteType::River])
            .all(state);

        if nearby_rivers.is_empty() {
            return Ok(vec![]);
        }

        let controller_id = self.get_controller_id(state);
        let want = yes_or_no(
            &controller_id,
            state,
            "Fenvale Muse: Trigger the Genesis of a nearby River?",
        )
        .await?;
        if !want {
            return Ok(vec![]);
        }

        let river_id = pick_card(
            &controller_id,
            &nearby_rivers,
            state,
            "Fenvale Muse: Pick a nearby River to trigger",
        )
        .await?;
        let river = state.get_card(&river_id);
        river.genesis(state).await
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (FenvaleMuse::NAME, |owner_id: PlayerId| {
    Box::new(FenvaleMuse::new(owner_id))
});
