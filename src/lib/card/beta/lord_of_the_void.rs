use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    effect::Effect,
    game::{PlayerId, yes_or_no},
    query::ZoneQuery,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct LordOfTheVoid {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl LordOfTheVoid {
    pub const NAME: &'static str = "Lord of the Void";
    pub const DESCRIPTION: &'static str = "Voidwalk At the end of your turn, Lord of the Void may banish an adjacent site, unless there's an Avatar there.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 0,
                toughness: 0,
                abilities: vec![Ability::Voidwalk],
                types: vec![MinionType::Spirit],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(9, "AAA"),
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
impl Card for LordOfTheVoid {
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

    async fn on_turn_end(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        if state.current_player != controller_id {
            return Ok(vec![]);
        }
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        // Find adjacent zones that have a site and no avatar
        let my_zone = self.get_zone().clone();
        let candidate_sites: Vec<uuid::Uuid> = CardQuery::new()
            .sites()
            .near_to(&my_zone)
            .all(state)
            .into_iter()
            .filter(|site_id| {
                let site_zone = state.get_card(site_id).get_zone().clone();
                // Exclude zones with any avatar present
                let has_avatar = CardQuery::new().avatars().in_zone(&site_zone).any(state);
                !has_avatar
            })
            .collect();

        if candidate_sites.is_empty() {
            return Ok(vec![]);
        }

        let do_banish = yes_or_no(
            &controller_id,
            state,
            "Lord of the Void: Banish an adjacent site?",
        )
        .await?;

        if !do_banish {
            return Ok(vec![]);
        }

        let Some(target_site_id) = CardQuery::from_ids(candidate_sites)
            .with_prompt("Lord of the Void: Pick a site to banish")
            .pick(&controller_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };

        let target_zone = state.get_card(&target_site_id).get_zone().clone();

        // Move all units at the target zone back to their owners, then banish the site
        let units_there = CardQuery::new().units().in_zone(&target_zone).all(state);
        let mut effects: Vec<Effect> = units_there
            .into_iter()
            .map(|unit_id| {
                let unit = state.get_card(&unit_id);
                let unit_owner = *unit.get_owner_id();
                Effect::MoveCard {
                    player_id: unit_owner,
                    card_id: unit_id,
                    from: target_zone.clone(),
                    to: ZoneQuery::from_zone(Zone::Spellbook),
                    tap: false,
                    region: Region::Surface,
                    through_path: None,
                }
            })
            .collect();

        effects.push(Effect::BanishCard {
            card_id: target_site_id,
        });

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (LordOfTheVoid::NAME, |owner_id: PlayerId| {
        Box::new(LordOfTheVoid::new(owner_id))
    });
