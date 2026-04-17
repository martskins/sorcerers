use crate::{
    card::{AvatarBase, Card, CardBase, Cost, Costs, Edition, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::{ActivatedAbility, PlayerId, pick_zone},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
struct DeathspeakerAbility;

#[async_trait::async_trait]
impl ActivatedAbility for DeathspeakerAbility {
    fn get_name(&self) -> String {
        "Banish a dead minion to cast a copy".to_string()
    }

    fn can_activate(
        &self,
        _card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<bool> {
        let dead_minions = CardQuery::new().dead().minions().all(state);
        let has_dead_minion = !dead_minions.is_empty();
        let can_afford_any = dead_minions.iter().any(|id| {
            let card = state.get_card(id);
            let mana_cost = card.get_base().costs.mana_cost();
            let can_afford = mana_cost.can_afford(state, player_id).unwrap_or_default();
            can_afford
        });

        Ok(has_dead_minion && can_afford_any)
    }

    fn get_cost(&self, _card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        // The activation is free; the copy's mana cost is consumed inside on_select.
        Ok(Cost::ZERO.clone())
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        // Step 1: Pick a dead minion to copy.
        let Some(chosen_id) = CardQuery::new()
            .dead()
            .minions()
            .with_prompt("Deathspeaker: Pick a dead minion to copy")
            .pick(player_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };

        let chosen = state.get_card(&chosen_id);
        let card_name = chosen.get_name().to_string();
        let mana_cost = chosen.get_base().costs.mana_value();

        // Step 2: Check Death's Door (avatar has taken max damage).
        let deaths_door = state
            .get_player_avatar_id(player_id)
            .ok()
            .and_then(|avatar_id| {
                state
                    .get_card(&avatar_id)
                    .get_avatar_base()
                    .map(|ab| ab.deaths_door)
            })
            .unwrap_or(false);

        // Step 3: Check affordability (skip if on Death's Door).
        if !deaths_door {
            let available = *state.player_mana.get(player_id).unwrap_or(&0);
            if available < mana_cost {
                return Ok(vec![]);
            }
        }

        // Step 4: Pick a zone to summon the copy.
        let valid_zones = chosen.get_valid_play_zones(state)?;
        if valid_zones.is_empty() {
            return Ok(vec![]);
        }

        let chosen_zone = pick_zone(
            player_id,
            &valid_zones,
            state,
            false,
            "Deathspeaker: Pick a zone to summon the copy",
        )
        .await?;

        // Build effects.
        let mut effects: Vec<Effect> = vec![];

        // Banish the original dead minion.
        effects.push(Effect::BanishCard {
            card_id: chosen_id.clone(),
        });

        // Consume mana unless on Death's Door.
        if !deaths_door && mana_cost > 0 {
            effects.push(Effect::ConsumeMana {
                player_id: player_id.clone(),
                mana: mana_cost,
            });
        }

        // Summon a token copy (it will trigger Genesis then be auto-banished).
        effects.push(Effect::SummonCopy {
            card_name,
            player_id: player_id.clone(),
            zone: chosen_zone,
        });

        // Record that this ability was used this turn.
        effects.push(Effect::SetCardData {
            card_id: card_id.clone(),
            data: Box::new(true),
        });

        Ok(effects)
    }
}

#[derive(Debug, Clone)]
pub struct Deathspeaker {
    card_base: CardBase,
    unit_base: UnitBase,
    avatar_base: AvatarBase,
    has_used_ability: bool,
}

impl Deathspeaker {
    pub const NAME: &'static str = "Deathspeaker";
    pub const DESCRIPTION: &'static str = "Tap → Play or draw a site.\r \r You may banish a dead minion each turn to cast a copy of it, and for (0) if you're on Death's Door. The copy enters the realm, uses its Genesis, then is banished.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 20,
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::ZERO,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
            avatar_base: AvatarBase {
                ..Default::default()
            },
            has_used_ability: false,
        }
    }
}

#[async_trait::async_trait]
impl Card for Deathspeaker {
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

    fn get_avatar_base(&self) -> Option<&AvatarBase> {
        Some(&self.avatar_base)
    }

    fn get_avatar_base_mut(&mut self) -> Option<&mut AvatarBase> {
        Some(&mut self.avatar_base)
    }

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        if self.has_used_ability {
            return Ok(vec![]);
        }

        Ok(vec![Box::new(DeathspeakerAbility)])
    }

    fn set_data(&mut self, data: &Box<dyn std::any::Any + Send + Sync>) -> anyhow::Result<()> {
        if let Some(ability_used) = data.downcast_ref::<bool>() {
            self.has_used_ability = *ability_used;
        }

        Ok(())
    }

    async fn on_turn_start(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        // Reset the once-per-turn ability usage at the start of the turn.
        Ok(vec![Effect::SetCardData {
            card_id: self.card_base.id,
            data: Box::new(false),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Deathspeaker::NAME, |owner_id: PlayerId| {
        Box::new(Deathspeaker::new(owner_id))
    });
