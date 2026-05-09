use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, UnitBase, Zone,
    },
    effect::Effect,
    game::{PlayerId, pick_card},
    state::State,
};

#[derive(Debug, Clone)]
pub struct TvinnaxBerserker {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl TvinnaxBerserker {
    pub const NAME: &'static str = "Tvinnax Berserker";
    pub const DESCRIPTION: &'static str = "Whenever Tvinnax Berserker can attack a unit, he must. Untap Tvinnax Berserker whenever he attacks and kills an enemy minion.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                types: vec![MinionType::Mortal],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "FF"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for TvinnaxBerserker {
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

    async fn on_turn_start(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        if !self.can_attack(state) {
            return Ok(vec![]);
        }

        let valid_targets = self.get_valid_attack_targets(state, false);
        if valid_targets.is_empty() {
            return Ok(vec![]);
        }

        let player_id = self.get_controller_id(state);
        let picked_card_id = pick_card(
            player_id,
            &valid_targets,
            state,
            "Tvinnax Berserker: Choose a unit to attack",
        )
        .await?;
        Ok(vec![Effect::Attack {
            attacker_id: *self.get_id(),
            defender_id: picked_card_id,
        }])
    }

    async fn replace_effect(
        &self,
        _state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Option<Vec<Effect>>> {
        if let Effect::KillMinion {
            killer_id,
            from_attack: true,
            ..
        } = effect
            && killer_id == self.get_id()
        {
            return Ok(Some(vec![Effect::UntapCard {
                card_id: *self.get_id(),
            }]));
        }

        Ok(None)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (TvinnaxBerserker::NAME, |owner_id: PlayerId| {
        Box::new(TvinnaxBerserker::new(owner_id))
    });
