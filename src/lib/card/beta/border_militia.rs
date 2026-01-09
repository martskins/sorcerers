use crate::{
    card::{Card, CardBase, Edition, Plane, Rarity, Zone},
    effect::{Effect, TokenType},
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct BorderMilitia {
    pub card_base: CardBase,
}

impl BorderMilitia {
    pub const NAME: &'static str = "Border Militia";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 3,
                required_thresholds: Thresholds::parse("E"),
                plane: Plane::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for BorderMilitia {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    async fn on_cast(&mut self, state: &State, _caster_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        let mut sites: Vec<&Box<dyn Card>> = state
            .cards
            .iter()
            .filter(|c| c.is_site())
            .filter(|c| c.get_controller_id() == self.get_controller_id())
            .filter_map(|c| {
                for zone in c.get_zone().get_adjacent() {
                    match zone.get_site(state) {
                        Some(site) if site.get_controller_id() != self.get_controller_id() => return Some(c),
                        _ => {}
                    }
                }

                None
            })
            .collect();
        sites.dedup_by(|a, b| a.get_id() == b.get_id());

        Ok(sites
            .iter()
            .map(|site| Effect::SummonToken {
                player_id: self.get_controller_id().clone(),
                token_type: TokenType::FootSoldier,
                zone: site.get_zone().clone(),
            })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (BorderMilitia::NAME, |owner_id: PlayerId| {
    Box::new(BorderMilitia::new(owner_id))
});