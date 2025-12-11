use crate::{
    card::{ClamorOfHarpies, Flamecaller},
    effect::Effect,
    game::{PlayerId, PlayerStatus},
    networking::message::ClientMessage,
    state::State,
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CardType {
    Site,
    Spell,
    Avatar,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Edition {
    Alpha,
    Beta,
    ArthurianLegends,
    Dragonlord,
    Gothic,
}

impl Edition {
    pub fn url_name(&self) -> &str {
        match self {
            Edition::Alpha => "alp",
            Edition::Beta => "bet",
            Edition::ArthurianLegends => "art",
            Edition::Dragonlord => "drg",
            Edition::Gothic => "got",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Zone {
    None,
    Hand,
    Spellbook,
    Atlasbook,
    Realm(u8),
    Cemetery,
}

pub trait MessageHandler {
    fn handle_message(&mut self, message: &ClientMessage, state: &State) -> Vec<Effect> {
        Vec::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardInfo {
    pub id: uuid::Uuid,
    pub name: String,
    pub owner_id: PlayerId,
    pub tapped: bool,
    pub edition: Edition,
    pub zone: Zone,
    pub card_type: CardType,
}

impl CardInfo {
    pub fn is_site(&self) -> bool {
        self.card_type == CardType::Site
    }

    pub fn is_spell(&self) -> bool {
        self.card_type == CardType::Spell
    }
}

pub trait Card: Debug + Send + Sync + MessageHandler {
    fn get_name(&self) -> &str;
    fn get_edition(&self) -> Edition;
    fn get_owner_id(&self) -> PlayerId;
    fn is_tapped(&self) -> bool;
    fn get_card_type(&self) -> CardType;
    fn get_id(&self) -> uuid::Uuid;
    fn get_base_mut(&mut self) -> &mut CardBase;
    fn get_base(&self) -> &CardBase;
    fn set_status(&mut self, status: AvatarStatus) {
        assert_eq!(self.get_card_type(), CardType::Avatar);
    }

    fn get_zone(&self) -> Zone {
        self.get_base().zone.clone()
    }

    fn set_zone(&mut self, zone: Zone) {
        self.get_base_mut().zone = zone;
    }

    fn genesis(&mut self, state: &State) -> Vec<Effect> {
        vec![]
    }

    fn deathrite(&self, state: &State) -> Vec<Effect> {
        vec![]
    }

    fn is_site(&self) -> bool {
        false
    }
}

#[derive(Debug)]
pub struct UnitBase {
    pub power: u8,
    pub toughness: u8,
}

#[derive(Debug)]
pub struct CardBase {
    pub id: uuid::Uuid,
    pub owner_id: PlayerId,
    pub tapped: bool,
    pub zone: Zone,
    pub actions: Vec<Box<dyn CardAction>>,
}

pub trait CardAction: Debug + Send + Sync {
    fn get_name(&self) -> &str;
    fn on_pick(&self, _player_id: PlayerId, _card_id: Option<uuid::Uuid>, state: &State) -> Vec<Effect> {
        vec![]
    }
}

#[derive(Debug, Clone)]
pub enum AvatarStatus {
    PlaySite,
}

#[derive(Debug)]
pub enum BaseAvatarAction {
    PlaySite,
    DrawSite,
}

impl CardAction for BaseAvatarAction {
    fn get_name(&self) -> &str {
        match self {
            BaseAvatarAction::PlaySite => "Play Site",
            BaseAvatarAction::DrawSite => "Draw Site",
        }
    }

    fn on_pick(&self, player_id: PlayerId, card_id: Option<uuid::Uuid>, state: &State) -> Vec<Effect> {
        match self {
            BaseAvatarAction::PlaySite => {
                if let Some(card_id) = card_id {
                    vec![
                        Effect::SetPlayerStatus {
                            status: PlayerStatus::WaitingForPlay {
                                player_id: player_id.clone(),
                            },
                        },
                        Effect::SetAvatarStatus {
                            player_id: player_id.clone(),
                            status: AvatarStatus::PlaySite,
                            card_id: card_id,
                        },
                    ]
                } else {
                    vec![]
                }
            }
            BaseAvatarAction::DrawSite => vec![Effect::DrawCard {
                player_id: player_id.clone(),
                card_type: CardType::Site,
            }],
        }
    }
}

#[derive(Debug)]
pub struct AvatarBase {
    pub card_base: CardBase,
}

impl AvatarBase {
    pub fn base_actions() -> Vec<Box<dyn CardAction>> {
        vec![
            Box::new(BaseAvatarAction::PlaySite),
            Box::new(BaseAvatarAction::DrawSite),
        ]
    }
}

impl MessageHandler for AvatarBase {
    fn handle_message(&mut self, message: &ClientMessage, state: &State) -> Vec<Effect> {
        match message {
            ClientMessage::ClickCard { card_id, player_id, .. } if *card_id == self.card_base.id => {
                let actions = AvatarBase::base_actions();
                self.card_base.actions = actions;
                vec![Effect::PromptDecision {
                    player_id: player_id.clone(),
                    source_id: Some(self.card_base.id),
                    options: self
                        .card_base
                        .actions
                        .iter()
                        .map(|a| a.get_name().to_string())
                        .collect(),
                }]
            }
            ClientMessage::PickAction {
                action_idx,
                source_id,
                player_id,
                ..
            } if *player_id == self.card_base.owner_id => {
                let action = &self.card_base.actions[*action_idx];
                action.on_pick(player_id.clone(), source_id.clone(), state)
            }
            _ => vec![],
        }
    }
}

pub fn from_name(name: &str, player_id: PlayerId) -> Box<dyn Card> {
    match name {
        Flamecaller::NAME => Box::new(Flamecaller::new(player_id)),
        ClamorOfHarpies::NAME => Box::new(ClamorOfHarpies::new(player_id)),
        _ => panic!("Unknown card name: {}", name),
    }
}

pub fn from_name_and_zone(name: &str, player_id: PlayerId, zone: Zone) -> Box<dyn Card> {
    let mut card = from_name(name, player_id);
    card.set_zone(zone);
    card
}
