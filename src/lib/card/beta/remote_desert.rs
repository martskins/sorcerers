use crate::{
    card::{Card, CardBase, Edition, Plane, Rarity, SiteBase, SiteType, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds, pick_card},
    state::State,
};

#[derive(Debug, Clone)]
pub struct RemoteDesert {
    pub site_base: SiteBase,
    pub card_base: CardBase,
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
                plane: Plane::Surface,
                rarity: Rarity::Ordinary,
            },
        }
    }
}

#[async_trait::async_trait]
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

    async fn genesis(&self, state: &State) -> Vec<Effect> {
        let site_ids = self
            .get_zone()
            .get_nearby_sites(state, None)
            .iter()
            .map(|c| c.get_id().clone())
            .collect::<Vec<uuid::Uuid>>();
        if site_ids.is_empty() {
            return vec![];
        }

        let prompt = "Remote Desert: Pick a site to deal 1 damage to all atop units";
        let picked_card_id = pick_card(self.get_owner_id(), &site_ids, state, prompt).await;
        let site = state.get_card(&picked_card_id).unwrap();
        // TODO: filter atop units only
        let units = state.get_minions_in_zone(site.get_zone());
        let mut effects = vec![];
        for unit in units {
            effects.push(Effect::take_damage(&unit.get_id(), site.get_id(), 1));
        }
        effects
    }

    fn get_site_base(&self) -> Option<&SiteBase> {
        Some(&self.site_base)
    }

    fn get_site_base_mut(&mut self) -> Option<&mut SiteBase> {
        Some(&mut self.site_base)
    }
}
