use crate::{effect::FightContext, prelude::*};

#[derive(Debug, Clone)]
struct KarkemishChimeraAttack;

#[async_trait::async_trait]
impl ActivatedAbility for KarkemishChimeraAttack {
    fn get_name(&self) -> String {
        "Attack up to three units here".to_string()
    }

    fn get_cost(&self, card_id: &CardId, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::tap(card_id)))
    }

    fn can_activate(
        &self,
        card_id: &CardId,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<bool> {
        let card = state.get_card(card_id);
        let targets = CardQuery::new()
            .units()
            .in_zone(card.get_zone())
            .id_not(*card_id)
            .all(state)
            .into_iter()
            .filter(|id| state.get_card(id).get_controller_id(state) != *player_id)
            .count();

        Ok(targets > 0)
    }

    async fn on_select(
        &self,
        card_id: &CardId,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let card = state.get_card(card_id);
        let targets: Vec<CardId> = CardQuery::new()
            .units()
            .in_zone(card.get_zone())
            .id_not(*card_id)
            .all(state)
            .into_iter()
            .filter(|id| state.get_card(id).get_controller_id(state) != *player_id)
            .collect();
        if targets.is_empty() {
            return Ok(vec![]);
        }

        let mut picked = pick_cards(
            player_id,
            &targets,
            state,
            "Karkemish Chimera: pick up to three units to attack",
        )
        .await?;
        picked.retain(|id| targets.contains(id));
        picked.truncate(3);
        if picked.is_empty() {
            return Ok(vec![]);
        }

        Ok(vec![Effect::Fight {
            attacker_id: *card_id,
            defender_id: picked[0],
            defending_ids: picked,
            damage_assignment: None,
            context: FightContext::Attack,
        }])
    }
}

#[derive(Debug, Clone)]
pub struct KarkemishChimera {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl KarkemishChimera {
    pub const NAME: &'static str = "Karkemish Chimera";
    pub const DESCRIPTION: &'static str =
        "Can simultaneously attack up to three units at the same location.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 6,
                toughness: 6,
                types: vec![MinionType::Beast],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(6, "FF"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for KarkemishChimera {
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

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(KarkemishChimeraAttack)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (KarkemishChimera::NAME, |owner_id: PlayerId| {
        Box::new(KarkemishChimera::new(owner_id))
    });
