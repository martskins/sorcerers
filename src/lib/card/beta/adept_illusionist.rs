use crate::{
    card::{Ability, Card, CardBase, Cost, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::{ActivatedAbility, Element, PlayerId, pick_card_with_preview, pick_zone_near},
    state::{CardMatcher, State},
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
        let card_ids = CardMatcher::new()
            .controller_id(&player_id.clone())
            .with_names(vec![AdeptIllusionist::NAME.to_string()])
            .include_not_in_play(true)
            .not_in_ids(vec![card_id.clone()])
            .resolve_ids(state);

        if card_ids.is_empty() {
            return Ok(vec![]);
        }

        let picked_card_id =
            pick_card_with_preview(player_id, &card_ids, state, "Pick an Adept Illusionist to summon").await?;
        let picked_card = state.get_card(&picked_card_id);
        let zone = pick_zone_near(
            player_id,
            picked_card.get_zone(),
            state,
            false,
            "Pick a zone to summon the Adept Illusionist",
        )
        .await?;

        Ok(vec![
            Effect::SummonCard {
                player_id: player_id.clone(),
                card_id: card_id.clone(),
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

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                abilities: vec![
                    Ability::Spellcaster(Element::Fire),
                    Ability::Spellcaster(Element::Air),
                    Ability::Spellcaster(Element::Water),
                    Ability::Spellcaster(Element::Earth),
                ],
                types: vec![MinionType::Mortal],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(2, "WW"),
                region: Region::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for AdeptIllusionist {
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

    fn get_additional_activated_abilities(&self, _state: &State) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(AdeptIllusionistAction)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (AdeptIllusionist::NAME, |owner_id: PlayerId| {
    Box::new(AdeptIllusionist::new(owner_id))
});
