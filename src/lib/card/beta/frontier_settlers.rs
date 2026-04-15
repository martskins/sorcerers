use crate::{
    card::{
        AdditionalCost, Card, CardBase, Cost, Costs, Edition, MinionType, Rarity, Region, Rubble,
        UnitBase, Zone,
    },
    effect::Effect,
    game::{ActivatedAbility, PlayerId, pick_zone},
    state::State,
};

#[derive(Debug, Clone)]
struct SettleAction;

fn adjacent_void_or_rubble(card_id: &uuid::Uuid, state: &State) -> Vec<Zone> {
    let card = state.get_card(card_id);
    card.get_zone()
        .get_adjacent()
        .into_iter()
        .filter(|z| match z.get_site(state) {
            None => true,
            Some(site) => site.get_name() == Rubble::NAME,
        })
        .collect()
}

#[async_trait::async_trait]
impl ActivatedAbility for SettleAction {
    fn get_name(&self) -> String {
        "Tap → Reveal and play topmost site to adjacent void or Rubble; move there and lose this ability".to_string()
    }

    fn get_cost(&self, card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::tap(card_id)))
    }

    fn can_activate(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<bool> {
        // Needs at least one adjacent void/Rubble zone AND at least one site in the atlas deck.
        let has_valid_zone = !adjacent_void_or_rubble(card_id, state).is_empty();
        let has_site = state
            .decks
            .get(player_id)
            .map(|d| !d.sites.is_empty())
            .unwrap_or(false);
        Ok(has_valid_zone && has_site)
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let valid_zones = adjacent_void_or_rubble(card_id, state);
        if valid_zones.is_empty() {
            return Ok(vec![]);
        }

        let site_id = match state.decks.get(player_id).and_then(|d| d.sites.last()) {
            Some(id) => id.clone(),
            None => return Ok(vec![]),
        };

        let chosen_zone = pick_zone(
            player_id,
            &valid_zones,
            state,
            false,
            "Pick an adjacent void or Rubble zone to settle",
        )
        .await?;

        Ok(vec![
            // Draw the top site to hand, then immediately summon it to the chosen zone.
            Effect::DrawSite {
                player_id: player_id.clone(),
                count: 1,
            },
            Effect::SummonCard {
                player_id: player_id.clone(),
                card_id: site_id,
                zone: chosen_zone.clone(),
            },
            // Move the settlers to their new home.
            Effect::MoveCard {
                player_id: player_id.clone(),
                card_id: card_id.clone(),
                from: state.get_card(card_id).get_zone().clone(),
                to: crate::query::ZoneQuery::from_zone(chosen_zone.clone()),
                tap: false,
                region: Region::Surface,
                through_path: None,
            },
            Effect::SetCardData {
                card_id: card_id.clone(),
                data: Box::new(false),
            },
        ])
    }
}

#[derive(Debug, Clone)]
pub struct FrontierSettlers {
    unit_base: UnitBase,
    card_base: CardBase,
    has_ability: bool,
}

impl FrontierSettlers {
    pub const NAME: &'static str = "Frontier Settlers";
    pub const DESCRIPTION: &'static str = "Tap → Reveal and play your topmost site to an adjacent void or Rubble. Frontier Settlers move there and lose this ability.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 2,
                abilities: vec![],
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "EE"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
            has_ability: true,
        }
    }
}

#[async_trait::async_trait]
impl Card for FrontierSettlers {
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

    fn set_data(&mut self, data: &Box<dyn std::any::Any + Send + Sync>) -> anyhow::Result<()> {
        if let Some(has_ability) = data.downcast_ref::<bool>() {
            self.has_ability = *has_ability;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid data type for Frontier Settlers"))
        }
    }

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        if !self.has_ability {
            return Ok(vec![]);
        }

        Ok(vec![Box::new(SettleAction)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (FrontierSettlers::NAME, |owner_id: PlayerId| {
        Box::new(FrontierSettlers::new(owner_id))
    });
