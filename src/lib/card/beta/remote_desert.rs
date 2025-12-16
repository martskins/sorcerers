use crate::{
    card::{Card, CardBase, Edition, MessageHandler, SiteBase, SiteType, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds},
    networking::message::ClientMessage,
    state::State,
};

#[derive(Debug, Clone)]
enum Status {
    None,
    PickingSite,
}

#[derive(Debug, Clone)]
pub struct RemoteDesert {
    pub site_base: SiteBase,
    pub card_base: CardBase,
    status: Status,
}

impl RemoteDesert {
    pub const NAME: &'static str = "Remote Desert";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("F"),
                types: vec![SiteType::Desert],
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
        }
    }
}

impl Card for RemoteDesert {
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

    fn genesis(&self, state: &State) -> Vec<Effect> {
        let mut effects = vec![];
        effects.push(Effect::set_card_status(self.get_id(), Status::PickingSite));
        let units = self
            .get_zone()
            .get_nearby()
            .iter()
            .flat_map(|z| {
                state
                    .get_cards_in_zone(z)
                    .iter()
                    .filter(|c| c.is_site())
                    .map(|c| c.get_id().clone())
                    .collect::<Vec<uuid::Uuid>>()
            })
            .collect();

        effects.push(Effect::select_card(self.get_owner_id(), units, Some(self.get_id())));
        effects
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

impl MessageHandler for RemoteDesert {
    fn handle_message(&mut self, message: &ClientMessage, state: &State) -> Vec<Effect> {
        match (&self.status, message) {
            (Status::PickingSite, ClientMessage::PickCard { card_id, .. }) => {
                let site = state.get_card(card_id).unwrap();
                let units: Vec<uuid::Uuid> = state
                    .get_cards_in_zone(site.get_zone())
                    .iter()
                    .filter(|c| c.is_unit())
                    .map(|c| c.get_id().clone())
                    .collect();
                let mut effects = vec![];
                for unit_id in dbg!(units) {
                    effects.push(Effect::take_damage(&unit_id, site.get_id(), 1));
                }
                effects.push(Effect::wait_for_play(self.get_owner_id()));
                effects
            }
            _ => vec![],
        }
    }
}
