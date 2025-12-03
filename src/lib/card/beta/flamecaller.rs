use crate::{
    card::{AvatarBase, AvatarStatus, Card, CardBase, CardType, Edition, MessageHandler, Zone},
    effect::{CardStatus, Effect},
    game::PlayerId,
    networking::message::ClientMessage,
    state::State,
};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
enum Status {
    None,
    AvatarStatus(AvatarStatus),
}

#[derive(Debug)]
pub struct Flamecaller {
    pub avatar_base: AvatarBase,
    pub targeted_minion: uuid::Uuid,
}

impl Flamecaller {
    pub const NAME: &'static str = "Flamecaller";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            avatar_base: AvatarBase {
                card_base: CardBase::<Status> {
                    id: uuid::Uuid::new_v4(),
                    owner_id,
                    tapped: false,
                    zone: Zone::Spellbook,
                    actions: Vec::new(),
                    status: Status::None,
                },
            },
            targeted_minion: uuid::Uuid::nil(),
        }
    }
}

impl Card<Status> for Flamecaller {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase<Status> {
        &mut self.avatar_base.card_base
    }

    fn get_base(&self) -> &CardBase<Status> {
        &self.avatar_base.card_base
    }

    fn is_tapped(&self) -> bool {
        self.avatar_base.card_base.tapped
    }

    fn get_owner_id(&self) -> PlayerId {
        self.avatar_base.card_base.owner_id
    }

    fn get_edition(&self) -> Edition {
        Edition::Beta
    }

    fn get_id(&self) -> uuid::Uuid {
        self.avatar_base.card_base.id
    }

    fn get_card_type(&self) -> crate::card::CardType {
        CardType::Spell
    }

    fn genesis(&mut self, state: &State) -> Vec<Effect> {
        vec![]
    }
}

impl MessageHandler for Flamecaller {
    fn handle_message(&mut self, message: &ClientMessage) -> Vec<Effect> {
        self.avatar_base.handle_message(message)
    }
}
