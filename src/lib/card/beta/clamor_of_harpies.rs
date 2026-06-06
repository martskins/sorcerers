use crate::prelude::*;

#[derive(Debug, Clone, PartialEq)]
enum ClamorOfHarpiesAction {
    Strike,
    DoNotStrike,
}

#[async_trait::async_trait]
impl ActivatedAbility for ClamorOfHarpiesAction {
    fn get_name(&self) -> String {
        match self {
            ClamorOfHarpiesAction::Strike => "Strike".to_string(),
            ClamorOfHarpiesAction::DoNotStrike => "Do Not Strike".to_string(),
        }
    }

    async fn on_select(
        &self,
        card_id: &CardId,
        _player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        match self {
            ClamorOfHarpiesAction::Strike => {
                let target_card = state.get_card(card_id);
                Ok(vec![Effect::TakeDamage {
                    card_id: *target_card.get_id(),
                    from: *card_id,
                    damage: Damage::strike(
                        state.get_card(card_id).get_power(state)?.unwrap_or(0),
                        false,
                    ),
                }])
            }
            ClamorOfHarpiesAction::DoNotStrike => Ok(vec![]),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClamorOfHarpies {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl ClamorOfHarpies {
    pub const NAME: &'static str = "Clamor of Harpies";
    pub const DESCRIPTION: &'static str = "Airborne\r \r Genesis → Teleport target weaker minion to this location. Clamor of Harpies may strike it.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                types: vec![MinionType::Monster],
                tapped: false,
                ..Default::default()
            },
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
        }
    }
}

#[async_trait::async_trait]
impl Card for ClamorOfHarpies {
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

    async fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook::genesis(self.get_id())])
    }

    async fn resolve_hook(
        &self,
        hook: HookId,
        state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            GENESIS_HOOK_ID => {
        let valid_cards: Vec<CardId> = state
            .cards
            .values()
            .filter(|c| c.is_unit())
            .filter(|c| c.can_be_targetted_by_player(state, &self.get_controller_id(state)))
            .filter(|c| c.get_zone().is_in_play())
            .filter(|c| {
                c.get_power(state).unwrap_or_default().unwrap_or(0)
                    < self.get_power(state).unwrap_or_default().unwrap_or(0)
            })
            .map(|c| *c.get_id())
            .collect();
        let prompt = "Pick a unit to bring here";
        let card_id = pick_card_source(
            self.get_controller_id(state),
            &valid_cards,
            state,
            prompt,
            Some(*self.get_id()),
        )
        .await?;
        let card = state.get_card(&card_id);
        let activated_abilities: Vec<Box<dyn ActivatedAbility>> = vec![
            Box::new(ClamorOfHarpiesAction::Strike),
            Box::new(ClamorOfHarpiesAction::DoNotStrike),
        ];
        let prompt = "Strike selected unit?";
        let action = pick_action_source(
            self.get_controller_id(state),
            &activated_abilities,
            state,
            prompt,
            false,
            Some(*self.get_id()),
        )
        .await?;
        let mut effects = vec![Effect::MoveCard {
            player_id: self.get_controller_id(state),
            card_id,
            from: (card.get_zone().clone())
                .into_location()
                .expect("MoveCard source must be a location"),
            to: LocationQuery::from_zone(
                (self.get_zone().clone()).with_region(self.get_region(state).clone()),
            ),
            tap: false,
            through_path: None,
        }];
        effects.extend(
            action
                .on_select(card.get_id(), &self.get_controller_id(state), state)
                .await?,
        );
        Ok(effects)
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (ClamorOfHarpies::NAME, |owner_id: PlayerId| {
        Box::new(ClamorOfHarpies::new(owner_id))
    });
