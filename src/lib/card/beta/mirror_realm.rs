use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct MirrorRealm {
    site_base: SiteBase,
    card_base: CardBase,
}

impl MirrorRealm {
    pub const NAME: &'static str = "Mirror Realm";
    pub const DESCRIPTION: &'static str =
        "This site enters the realm as a copy of another nearby site.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::ZERO,
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
impl Site for MirrorRealm {}

impl ResourceProvider for MirrorRealm {}

#[async_trait::async_trait]
impl Card for MirrorRealm {
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

    async fn play_mechanic(
        &self,
        state: &State,
        player_id: &PlayerId,
        caster_id: &uuid::Uuid,
    ) -> anyhow::Result<Vec<Effect>> {
        let locations = self.get_valid_play_locations(state, player_id, caster_id)?;
        let location = LocationQuery::from_locations(locations)
            .with_prompt("Pick a zone to play the site")
            .with_source_card(*self.get_id())
            .pick(player_id, state)
            .await?;
        self.play_mechanic_at_location(state, player_id, caster_id, &location)
            .await
    }

    async fn play_mechanic_at_location(
        &self,
        state: &State,
        player_id: &PlayerId,
        caster_id: &uuid::Uuid,
        location: &Location,
    ) -> anyhow::Result<Vec<Effect>> {
        let Some(picked_site_id) = CardQuery::new()
            .sites()
            .near_to(location)
            .id_not(*self.get_id())
            .with_prompt("Pick a nearby site to copy")
            .with_source_card(*self.get_id())
            .pick(player_id, state)
            .await?
        else {
            return Ok(vec![Effect::PlayCard {
                player_id: *player_id,
                card_id: *self.get_id(),
                location: location.clone(),
                spellcaster: *caster_id,
            }]);
        };

        Ok(vec![
            Effect::PlayCard {
                player_id: *player_id,
                card_id: *self.get_id(),
                location: location.clone(),
                spellcaster: *caster_id,
            },
            Effect::MakeCardCopyOf {
                card_id: *self.get_id(),
                copy_source_id: picked_site_id,
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (MirrorRealm::NAME, |owner_id: PlayerId| {
    Box::new(MirrorRealm::new(owner_id))
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        card::{AridDesert, FootSoldier, Sorcerer},
        deck::Deck,
        networking::message::{ClientMessage, ServerMessage},
        query::QueryCache,
        state::{Player, PlayerWithDeck},
    };

    #[tokio::test]
    async fn mirror_realm_triggers_copied_site_genesis_when_entering() {
        QueryCache::init();

        let game_id = uuid::Uuid::new_v4();
        let player_id = uuid::Uuid::new_v4();
        let opponent_id = uuid::Uuid::new_v4();

        let avatar = Sorcerer::new(player_id);
        let avatar_id = *avatar.get_id();
        let opponent_avatar = Sorcerer::new(opponent_id);
        let opponent_avatar_id = *opponent_avatar.get_id();

        let player = PlayerWithDeck {
            player: Player {
                id: player_id,
                name: "Player 1".to_string(),
            },
            deck: Deck::new(
                &player_id,
                "Test Deck".to_string(),
                vec![],
                vec![],
                avatar_id,
            ),
            cards: vec![Box::new(avatar)],
        };
        let opponent = PlayerWithDeck {
            player: Player {
                id: opponent_id,
                name: "Player 2".to_string(),
            },
            deck: Deck::new(
                &opponent_id,
                "Opponent Deck".to_string(),
                vec![],
                vec![],
                opponent_avatar_id,
            ),
            cards: vec![Box::new(opponent_avatar)],
        };

        let (server_tx, server_rx) = async_channel::unbounded();
        let (client_tx, client_rx) = async_channel::unbounded();
        let mut state = State::new(game_id, vec![player, opponent], server_tx, client_rx);

        let mut source_site = AridDesert::new(player_id);
        let source_site_id = *source_site.get_id();
        source_site.set_zone(Zone::Location(Location::Square(2, Region::Surface)));
        state.add_card(Box::new(source_site));

        let mut target_minion = FootSoldier::new(opponent_id);
        let target_minion_id = *target_minion.get_id();
        target_minion.set_zone(Zone::Location(Location::Square(2, Region::Surface)));
        state.add_card(Box::new(target_minion));

        let mut mirror_realm = MirrorRealm::new(player_id);
        let mirror_realm_id = *mirror_realm.get_id();
        mirror_realm.set_zone(Zone::Hand);
        state.add_card(Box::new(mirror_realm));

        let responder = tokio::spawn(async move {
            let mut copied_source = false;
            let mut picked_genesis_target = false;
            loop {
                let message = server_rx.recv().await.unwrap();
                match message {
                    ServerMessage::ForceSync { .. }
                    | ServerMessage::Wait { .. } => {}
                    ServerMessage::Resume { .. } if picked_genesis_target => break,
                    ServerMessage::Resume { .. } => {}
                    ServerMessage::PickCard {
                        player_id,
                        prompt,
                        cards,
                        ..
                    } if prompt == "Pick a nearby site to copy" => {
                        assert!(cards.contains(&source_site_id));
                        client_tx
                            .send(ClientMessage::PickCard {
                                game_id,
                                player_id,
                                card_id: source_site_id,
                            })
                            .await
                            .unwrap();
                        copied_source = true;
                    }
                    ServerMessage::PickCard {
                        player_id,
                        prompt,
                        cards,
                        ..
                    } if prompt == "Pick a site to deal 1 damage to all atop units" => {
                        assert!(copied_source);
                        assert!(cards.contains(&source_site_id));
                        client_tx
                            .send(ClientMessage::PickCard {
                                game_id,
                                player_id,
                                card_id: source_site_id,
                            })
                            .await
                            .unwrap();
                        picked_genesis_target = true;
                    }
                    other => panic!("expected Mirror Realm prompts, got {:?}", other),
                }
            }
        });

        let effects = state
            .get_card(&mirror_realm_id)
            .play_mechanic_at_location(
                &state,
                &player_id,
                &avatar_id,
                &Location::Square(1, Region::Surface),
            )
            .await
            .unwrap();
        state.queue(effects);
        state.apply_effects_without_log().await.unwrap();

        responder.await.unwrap();

        assert_eq!(state.get_card(&mirror_realm_id).get_name(), AridDesert::NAME);
        assert_eq!(
            state
                .get_card(&target_minion_id)
                .get_damage_taken()
                .unwrap_or_default(),
            1
        );
    }
}
