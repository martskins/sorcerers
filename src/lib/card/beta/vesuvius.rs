use crate::{
    card::{Card, CardBase, Edition, MessageHandler, Rubble, SiteBase, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds},
    networking::message::ClientMessage,
    state::State,
};

#[derive(Debug, Clone)]
enum Status {
    None,
    ChoosingAction,
    ChoosingSite,
}

#[derive(Debug, Clone)]
enum Action {
    UseAbility,
    Cancel,
}

impl Action {
    pub fn get_name(&self) -> &str {
        match self {
            Action::UseAbility => "Use Vesuvius Ability",
            Action::Cancel => "Cancel",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Vesuvius {
    pub site_base: SiteBase,
    pub card_base: CardBase,
    status: Status,
    actions: Vec<Action>,
}

impl Vesuvius {
    pub const NAME: &'static str = "Vesuvius";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("F"),
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Atlasbook,
                mana_cost: 0,
                required_thresholds: Thresholds::new(),
            },
            status: Status::None,
            actions: vec![Action::UseAbility, Action::Cancel],
        }
    }
}

impl Card for Vesuvius {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn is_tapped(&self) -> bool {
        self.card_base.tapped
    }

    fn get_owner_id(&self) -> &PlayerId {
        &self.card_base.owner_id
    }

    fn get_edition(&self) -> Edition {
        Edition::Beta
    }

    fn get_id(&self) -> &uuid::Uuid {
        &self.card_base.id
    }

    fn get_site_base(&self) -> Option<&SiteBase> {
        Some(&self.site_base)
    }

    fn get_site_base_mut(&mut self) -> Option<&mut SiteBase> {
        Some(&mut self.site_base)
    }

    fn set_status(&mut self, status: &Box<dyn std::any::Any>) -> anyhow::Result<()> {
        let status = status
            .downcast_ref::<Status>()
            .ok_or_else(|| anyhow::anyhow!("Failed to downcast status for {}", Self::NAME))?;
        self.status = status.clone();
        Ok(())
    }
}

impl MessageHandler for Vesuvius {
    fn handle_message(&mut self, message: &ClientMessage, state: &State) -> Vec<Effect> {
        match (&self.status, message) {
            (Status::None, ClientMessage::ClickCard { card_id, .. }) if card_id == self.get_id() => {
                if !state
                    .get_player_resources(self.get_owner_id())
                    .has_resources(0, Thresholds::parse("FFF"))
                {
                    return vec![];
                }

                vec![
                    Effect::set_card_status(self.get_id(), Status::ChoosingAction),
                    Effect::select_action(
                        self.get_owner_id(),
                        self.actions.iter().map(|c| c.get_name().to_string()).collect(),
                    ),
                ]
            }
            (Status::ChoosingAction, ClientMessage::PickAction { action_idx, .. }) => {
                let action = &self.actions[*action_idx];
                match action {
                    Action::UseAbility => {
                        self.status = Status::ChoosingSite;
                        let site_ids = self
                            .get_zone()
                            .get_nearby_sites(state, None)
                            .iter()
                            .map(|c| c.get_id().clone())
                            .collect();
                        vec![
                            Effect::set_card_status(self.get_id(), Status::ChoosingSite),
                            Effect::select_card(self.get_owner_id(), site_ids, Some(self.get_id())),
                        ]
                    }
                    Action::Cancel => {
                        self.status = Status::None;
                        vec![Effect::wait_for_play(self.get_owner_id())]
                    }
                }
            }
            (Status::ChoosingSite, ClientMessage::PickCard { card_id, .. }) => {
                let site = state.get_card(card_id).unwrap();
                let unit_ids: Vec<&uuid::Uuid> = state
                    .get_cards_in_zone(site.get_zone())
                    .iter()
                    .filter(|c| c.is_unit())
                    .map(|c| c.get_id())
                    .collect();
                let rubble = Rubble::new(self.get_owner_id().clone());
                let rubble_id = rubble.get_id().clone();
                let mut effects = vec![
                    Effect::set_card_status(self.get_id(), Status::None),
                    Effect::AddCard { card: Box::new(rubble) },
                    Effect::PlayCard {
                        player_id: self.get_owner_id().clone(),
                        card_id: rubble_id,
                        zone: self.get_zone().clone(),
                    },
                    Effect::bury_card(self.get_id(), self.get_zone()),
                ];
                for unit_id in unit_ids {
                    effects.push(Effect::take_damage(unit_id, self.get_id(), 3));
                }
                effects.push(Effect::wait_for_play(self.get_owner_id()));
                effects
            }
            _ => vec![],
        }
    }
}
