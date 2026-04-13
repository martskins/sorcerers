use crate::{
    card::{Ability, Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::{ActivatedAbility, Element, PlayerId, pick_zone_near},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
struct AdeptIllusionistAction;

#[async_trait::async_trait]
impl ActivatedAbility for AdeptIllusionistAction {
    fn get_name(&self) -> String {
        "Search for another Adept Illusionist".to_string()
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let Some(picked_card_id) = CardQuery::new()
            .controlled_by(&player_id.clone())
            .cards_named(AdeptIllusionist::NAME)
            .including_not_in_play()
            .id_not_in(vec![card_id.clone()])
            .pick(player_id, state, true)
            .await?
        else {
            return Ok(vec![]);
        };

        let card = state.get_card(card_id);
        let zone = pick_zone_near(
            player_id,
            card.get_zone(),
            state,
            false,
            "Pick a zone to summon the Adept Illusionist",
        )
        .await?;

        Ok(vec![
            Effect::SummonCard {
                player_id: player_id.clone(),
                card_id: picked_card_id.clone(),
                zone: zone.clone(),
            },
            Effect::ShuffleDeck {
                player_id: player_id.clone(),
            },
        ])
    }
}

#[derive(Debug, Clone)]
pub struct AdeptIllusionist {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl AdeptIllusionist {
    pub const NAME: &'static str = "Adept Illusionist";
    pub const DESCRIPTION: &'static str = "Spellcaster

Tap → Search your hand, cemetery, or spellbook for another Adept Illusionist and summon it nearby. Shuffle if needed.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                abilities: vec![Ability::Spellcaster(None)],
                types: vec![MinionType::Mortal],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "WW"),
                region: Region::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for AdeptIllusionist {
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

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(AdeptIllusionistAction)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (AdeptIllusionist::NAME, |owner_id: PlayerId| {
        Box::new(AdeptIllusionist::new(owner_id))
    });
