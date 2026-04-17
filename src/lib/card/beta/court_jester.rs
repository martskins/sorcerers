use crate::{
    card::{Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct CourtJester {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl CourtJester {
    pub const NAME: &'static str = "Court Jester";
    pub const DESCRIPTION: &'static str =
        "At the end of your turn, each nearby Avatar discards a random card.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "W"),
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
impl Card for CourtJester {
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

    async fn on_turn_end(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);

        // Only trigger at the end of the controller's turn.
        if state.current_player != controller_id {
            return Ok(vec![]);
        }

        let zone = self.get_zone();
        if !zone.is_in_play() {
            return Ok(vec![]);
        }

        let nearby_avatars = CardQuery::new().avatars().near_to(zone).all(state);
        let mut effects = vec![];
        for avatar_id in nearby_avatars {
            let avatar = state.get_card(&avatar_id);
            let avatar_controller = avatar.get_controller_id(state);

            // Pick a random card from their hand to discard.
            let random_hand_card = CardQuery::new()
                .in_zone(&Zone::Hand)
                .controlled_by(&avatar_controller)
                .randomised()
                .count(1)
                .pick(&avatar_controller, state, false)
                .await?;

            if let Some(card_id) = random_hand_card {
                effects.push(Effect::DiscardCard {
                    player_id: avatar_controller,
                    card_id,
                });
            }
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (CourtJester::NAME, |owner_id: PlayerId| {
    Box::new(CourtJester::new(owner_id))
});
