use crate::{
    card::{Aura, AuraBase, Card, CardBase, CardConstructor, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{PlayerId, pick_zone},
    query::ZoneQuery,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Wildfire {
    aura_base: AuraBase,
    card_base: CardBase,
    sites_visited: Vec<Zone>,
}

impl Wildfire {
    pub const NAME: &'static str = "Wildfire";
    pub const DESCRIPTION: &'static str = "Conjure Wildfire atop a single site nearby.\r \r At the end of each turn, each unit here takes 3 damage, then move Wildfire to an adjacent location it hasn't visited before. If none remain, dispel Wildfire.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "F"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            aura_base: AuraBase {
                tapped: false,
                region: Region::Surface,
            },
            sites_visited: vec![],
        }
    }
}

impl Aura for Wildfire {}

#[async_trait::async_trait]
impl Card for Wildfire {
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

    fn get_aura_base(&self) -> Option<&AuraBase> {
        Some(&self.aura_base)
    }
    fn get_aura_base_mut(&mut self) -> Option<&mut AuraBase> {
        Some(&mut self.aura_base)
    }

    fn set_data(&mut self, data: &Box<dyn std::any::Any + Send + Sync>) -> anyhow::Result<()> {
        if let Some(sites_visited) = data.downcast_ref::<Vec<Zone>>() {
            self.sites_visited = sites_visited.clone();
        }

        Ok(())
    }

    async fn on_visit_zone(
        &self,
        _state: &State,
        _from: &Zone,
        to: &Zone,
    ) -> anyhow::Result<Vec<Effect>> {
        let mut sites_visited = self.sites_visited.clone();
        if sites_visited.is_empty() {
            sites_visited.push(self.get_zone().clone());
        }

        sites_visited.push(to.clone());
        Ok(vec![Effect::SetCardData {
            card_id: *self.get_id(),
            data: Box::new(sites_visited),
        }])
    }

    async fn on_turn_end(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let zones = self
            .get_zone()
            .get_adjacent()
            .iter()
            .filter(|z| !self.sites_visited.contains(z))
            .cloned()
            .collect::<Vec<Zone>>();
        if zones.is_empty() {
            return Ok(vec![Effect::BuryCard {
                card_id: *self.get_id(),
            }]);
        }

        let mut effects = CardQuery::new()
            .units()
            .in_zone(self.get_zone())
            .all(state)
            .into_iter()
            .map(|id| Effect::TakeDamage {
                card_id: id,
                from: *self.get_id(),
                damage: 3,
                is_strike: false,
                is_ranged: false,
            })
            .collect::<Vec<Effect>>();

        let prompt = "Wildfire: Pick a zone to move to:";
        let picked_zone = pick_zone(self.get_owner_id(), &zones, state, false, prompt).await?;
        effects.push(Effect::MoveCard {
            player_id: *self.get_owner_id(),
            card_id: *self.get_id(),
            from: self.get_zone().clone(),
            to: ZoneQuery::from_zone(picked_zone.clone()),
            tap: false,
            region: self.get_region(state).clone(),
            through_path: None,
        });

        Ok(effects)
    }

    fn get_valid_play_zones(
        &self,
        _state: &State,
        _player_id: &PlayerId,
    ) -> anyhow::Result<Vec<Zone>> {
        Ok(Zone::all_realm())
    }

    fn get_aura(&self) -> Option<&dyn Aura> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Wildfire::NAME, |owner_id: PlayerId| {
    Box::new(Wildfire::new(owner_id))
});
