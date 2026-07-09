use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Boneyard {
    site_base: SiteBase,
    card_base: CardBase,
}

impl Boneyard {
    pub const NAME: &'static str = "Boneyard";
    pub const DESCRIPTION: &'static str =
        "Genesis -> Each player may summon a minion from their cemetery here.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::ZERO,
                types: vec![],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Site for Boneyard {}

impl ResourceProvider for Boneyard {}

#[async_trait::async_trait]
impl Card for Boneyard {
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

    fn get_site_base(&self) -> Option<&SiteBase> {
        Some(&self.site_base)
    }

    fn get_site_base_mut(&mut self) -> Option<&mut SiteBase> {
        Some(&mut self.site_base)
    }

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook::genesis(self.get_id())])
    }

    async fn resolve_hook(
        &self,
        hook: HookId,
        state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            GENESIS_HOOK_ID => {
                let active_player = state.current_player();
                let mut player_ids = state
                    .players
                    .iter()
                    .map(|player| player.id)
                    .collect::<Vec<_>>();
                player_ids.sort_by_key(|player_id| *player_id != active_player);

                let mut cards = vec![];
                for player_id in player_ids {
                    let minion = CardQuery::new()
                        .in_zone(&Zone::Cemetery)
                        .minions()
                        .controlled_by(&player_id)
                        .with_source_card(*self.get_id())
                        .with_prompt("Pick a minion in your cemetery to summon in Boneyard")
                        .pick(&player_id, state)
                        .await?;
                    let Some(minion) = minion else {
                        continue;
                    };

                    let minion_card = state.get_card(&minion);
                    let locations = minion_card.base_play_locations_at(self.get_location(), state);
                    if locations.is_empty() {
                        continue;
                    }
                    let to_location = if locations.len() == 1 {
                        locations[0].clone()
                    } else {
                        LocationQuery::from_locations(locations)
                            .with_prompt("Pick where to summon your minion")
                            .with_source_card(*self.get_id())
                            .pick(&player_id, state)
                            .await?
                    };

                    cards.push(SummonCard {
                        player_id,
                        card_id: minion,
                        from_zone: Zone::Cemetery,
                        to_location,
                    });
                }

                Ok(vec![Effect::SummonCards {
                    summoned_cards: cards,
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Boneyard::NAME, |owner_id: PlayerId| {
    Box::new(Boneyard::new(owner_id))
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        card::{CaveTrolls, FootSoldier, Sorcerer},
        deck::Deck,
        networking::message::{ClientMessage, ServerMessage},
        query::QueryCache,
        state::{Player, PlayerWithDeck},
    };

    fn make_state() -> (
        State,
        async_channel::Receiver<ServerMessage>,
        async_channel::Sender<ClientMessage>,
    ) {
        QueryCache::init();

        let player_one_id = uuid::Uuid::new_v4();
        let player_two_id = uuid::Uuid::new_v4();
        let avatar_one = Sorcerer::new(player_one_id);
        let avatar_one_id = *avatar_one.get_id();
        let avatar_two = Sorcerer::new(player_two_id);
        let avatar_two_id = *avatar_two.get_id();

        let player1 = PlayerWithDeck {
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
        let player2 = PlayerWithDeck {
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
        let (client_tx, client_rx) = async_channel::unbounded();
        (
            State::new(
                uuid::Uuid::new_v4(),
                vec![player1, player2],
                server_tx,
                client_rx,
            ),
            server_rx,
            client_tx,
        )
    }

    #[tokio::test]
    async fn boneyard_player_without_minion_does_not_cancel_other_player() {
        let (mut state, server_rx, client_tx) = make_state();
        let game_id = state.game_id;
        let player_id = state.players[1].id;

        let mut boneyard = Boneyard::new(state.players[0].id);
        boneyard.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
        state.add_card(Box::new(boneyard.clone()));

        let mut minion = FootSoldier::new(player_id);
        let minion_id = *minion.get_id();
        minion.set_zone(Zone::Cemetery);
        state.add_card(Box::new(minion));

        tokio::spawn(async move {
            while let Ok(message) = server_rx.recv().await {
                match message {
                    ServerMessage::PickCard {
                        player_id, cards, ..
                    } => {
                        assert_eq!(cards, vec![minion_id]);
                        client_tx
                            .send(ClientMessage::PickCard {
                                game_id,
                                player_id,
                                card_id: minion_id,
                            })
                            .await
                            .unwrap();
                    }
                    ServerMessage::PickLocation {
                        player_id,
                        locations,
                        ..
                    } => {
                        let location = Location::Square(8, Region::Surface);
                        assert!(locations.contains(&location));
                        client_tx
                            .send(ClientMessage::PickLocation {
                                game_id,
                                player_id,
                                location,
                            })
                            .await
                            .unwrap();
                    }
                    _ => {}
                }
            }
        });

        let effects = boneyard
            .resolve_hook(GENESIS_HOOK_ID, &state, &Effect::Noop)
            .await
            .expect("boneyard genesis should resolve");

        let [Effect::SummonCards { summoned_cards }] = effects.as_slice() else {
            panic!("expected one SummonCards effect");
        };
        assert_eq!(summoned_cards.len(), 1);
        assert_eq!(summoned_cards[0].player_id, player_id);
        assert_eq!(summoned_cards[0].card_id, minion_id);
        assert_eq!(
            summoned_cards[0].to_location,
            Location::Square(8, Region::Surface)
        );
    }

    #[tokio::test]
    async fn boneyard_can_summon_to_non_surface_region_on_site() {
        let (mut state, server_rx, client_tx) = make_state();
        let game_id = state.game_id;
        let player_id = state.players[0].id;

        let mut boneyard = Boneyard::new(player_id);
        boneyard.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
        state.add_card(Box::new(boneyard.clone()));

        let mut minion = CaveTrolls::new(player_id);
        let minion_id = *minion.get_id();
        minion.set_zone(Zone::Cemetery);
        state.add_card(Box::new(minion));

        tokio::spawn(async move {
            while let Ok(message) = server_rx.recv().await {
                match message {
                    ServerMessage::PickCard {
                        player_id, cards, ..
                    } => {
                        assert_eq!(cards, vec![minion_id]);
                        client_tx
                            .send(ClientMessage::PickCard {
                                game_id,
                                player_id,
                                card_id: minion_id,
                            })
                            .await
                            .unwrap();
                    }
                    ServerMessage::PickLocation {
                        player_id,
                        locations,
                        ..
                    } => {
                        let location = Location::Square(8, Region::Underground);
                        assert!(locations.contains(&location));
                        client_tx
                            .send(ClientMessage::PickLocation {
                                game_id,
                                player_id,
                                location,
                            })
                            .await
                            .unwrap();
                    }
                    _ => {}
                }
            }
        });

        let effects = boneyard
            .resolve_hook(GENESIS_HOOK_ID, &state, &Effect::Noop)
            .await
            .expect("boneyard genesis should resolve");

        let [Effect::SummonCards { summoned_cards }] = effects.as_slice() else {
            panic!("expected one SummonCards effect");
        };
        assert_eq!(
            summoned_cards[0].to_location,
            Location::Square(8, Region::Underground)
        );
    }
}
