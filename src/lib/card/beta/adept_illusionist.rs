use crate::prelude::*;

#[derive(Debug, Clone)]
struct AdeptIllusionistAction;

#[async_trait::async_trait]
impl ActivatedAbility for AdeptIllusionistAction {
    fn get_name(&self) -> String {
        "Tap -> Search for another Adept Illusionist".to_string()
    }

    fn get_cost(&self, card_id: &CardId, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::tap(card_id)))
    }

    async fn on_select(
        &self,
        card_id: &CardId,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        // TODO: I don't like how this displays only the adept illusioninsts on each zone. I think
        // I'd want to show all cards on that section and then highlight the cards that can be
        // picked instead. We do that somewhere else, but I want something easier to use than that.
        // Also, if more than one window is opened, they are opened one on top of each other, and
        // it's not clear that there are more than one until you close it or move it.
        let Some(picked_card_id) = CardQuery::new()
            .controlled_by(&player_id.clone())
            .named(AdeptIllusionist::NAME.to_string())
            .in_zones(&[Zone::Hand, Zone::Cemetery, Zone::Spellbook])
            .id_not(*card_id)
            .pick(player_id, state)
            .await?
        else {
            return Ok(vec![]);
        };

        let card = state.get_card(card_id);
        let location = pick_location_near(
            player_id,
            card.get_location(),
            state,
            false,
            "Pick a zone to summon the Adept Illusionist",
        )
        .await?;

        Ok(vec![
            Effect::SummonCards {
                summoned_cards: vec![SummonCard {
                    player_id: *player_id,
                    card_id: picked_card_id,
                    from_zone: Zone::Spellbook,
                    to_location: location,
                }],
            },
            Effect::ShuffleDeck {
                player_id: *player_id,
            },
        ])
    }
}

#[derive(Debug, Clone)]
pub struct AdeptIllusionist {
    unit_base: UnitBase,
    card_base: CardBase,
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
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "WW"),
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
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (AdeptIllusionist::NAME, |owner_id: PlayerId| {
        Box::new(AdeptIllusionist::new(owner_id))
    });
