use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct MountainGiant {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl MountainGiant {
    pub const NAME: &'static str = "Mountain Giant";
    pub const DESCRIPTION: &'static str = "Occupies four locations.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 8,
                toughness: 8,
                abilities: vec![Ability::Oversized],
                types: vec![MinionType::Giant],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(8, "EEEE"),
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
impl Card for MountainGiant {
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

}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (MountainGiant::NAME, |owner_id: PlayerId| {
        Box::new(MountainGiant::new(owner_id))
    });

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        card::{AridDesert, Sorcerer},
        deck::Deck,
        networking::message::{ClientMessage, ServerMessage},
        state::{Player, PlayerWithDeck},
    };

    fn state_with_sites(squares: &[u8]) -> (State, async_channel::Receiver<ServerMessage>) {
        let player_one_id = uuid::Uuid::new_v4();
        let player_two_id = uuid::Uuid::new_v4();
        let avatar_one = Sorcerer::new(player_one_id);
        let avatar_one_id = *avatar_one.get_id();
        let avatar_two = Sorcerer::new(player_two_id);
        let avatar_two_id = *avatar_two.get_id();
        let player_one = PlayerWithDeck {
            player: Player {
                id: player_one_id,
                name: "Player 1".to_string(),
            },
            deck: Deck::new(
                &player_one_id,
                "Test Deck".to_string(),
                vec![],
                vec![],
                avatar_one_id,
            ),
            cards: vec![Box::new(avatar_one)],
        };
        let player_two = PlayerWithDeck {
            player: Player {
                id: player_two_id,
                name: "Player 2".to_string(),
            },
            deck: Deck::new(
                &player_two_id,
                "Test Deck".to_string(),
                vec![],
                vec![],
                avatar_two_id,
            ),
            cards: vec![Box::new(avatar_two)],
        };
        let (server_tx, server_rx) = async_channel::unbounded();
        let (_client_tx, client_rx) = async_channel::unbounded::<ClientMessage>();
        let mut state = State::new(
            uuid::Uuid::new_v4(),
            vec![player_one, player_two],
            server_tx,
            client_rx,
        );
        for square in squares {
            let mut site = AridDesert::new(player_one_id);
            site.set_zone(Zone::Location(Location::Square(*square, Region::Surface)));
            state.add_card(Box::new(site));
        }

        (state, server_rx)
    }

    #[test]
    fn can_be_summoned_to_a_fully_occupied_intersection() {
        let mut state = State::new_mock_state(vec![1, 2, 6, 7]);
        let player_id = state.players[0].id;
        *state.get_player_mana_mut(&player_id) = 8;

        let mut giant = MountainGiant::new(player_id);
        giant.set_zone(Zone::Hand);
        state.add_card(Box::new(giant.clone()));
        let avatar_id = state.get_player_avatar_id(&player_id).unwrap();

        assert_eq!(
            giant
                .get_valid_play_locations(&state, &player_id, &avatar_id)
                .unwrap(),
            vec![Location::Intersection(vec![1, 2, 6, 7], Region::Surface)]
        );
    }

    #[test]
    fn cannot_be_summoned_to_an_intersection_with_a_void_square() {
        let mut state = State::new_mock_state(vec![1, 2, 6]);
        let player_id = state.players[0].id;
        *state.get_player_mana_mut(&player_id) = 8;

        let mut giant = MountainGiant::new(player_id);
        giant.set_zone(Zone::Hand);
        state.add_card(Box::new(giant.clone()));
        let avatar_id = state.get_player_avatar_id(&player_id).unwrap();

        assert!(giant
            .get_valid_play_locations(&state, &player_id, &avatar_id)
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn playing_to_an_intersection_summons_the_giant() {
        let (mut state, _server_rx) = state_with_sites(&[1, 2, 6, 7]);
        let player_id = state.players[0].id;
        *state.get_player_mana_mut(&player_id) = 8;
        let avatar_id = state.get_player_avatar_id(&player_id).unwrap();

        let mut giant = MountainGiant::new(player_id);
        let giant_id = *giant.get_id();
        giant.set_zone(Zone::Hand);
        state.add_card(Box::new(giant));

        let location = Location::Intersection(vec![1, 2, 6, 7], Region::Surface);
        state.queue_one(Effect::PlayCard {
            player_id,
            card_id: giant_id,
            location: location.clone(),
            spellcaster: avatar_id,
        });
        state.apply_effects_without_log().await.unwrap();

        assert_eq!(state.get_card(&giant_id).get_zone(), &Zone::Location(location));
    }
}
