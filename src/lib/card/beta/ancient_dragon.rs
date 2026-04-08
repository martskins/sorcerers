use crate::{
    card::{Ability, Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::{ActivatedAbility, PlayerId, pick_zone_near},
    state::{CardMatcher, State},
};

#[derive(Debug, Clone)]
struct AncientDragonAbility;

#[async_trait::async_trait]
impl ActivatedAbility for AncientDragonAbility {
    fn get_name(&self) -> String {
        "Tap to deal 4 damage".to_string()
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let card = state.get_card(card_id);
        let picked_zone = pick_zone_near(
            player_id,
            card.get_zone(),
            state,
            false,
            "Pick a zone to deal damage in",
        )
        .await?;
        let unit_ids = CardMatcher::new()
            .in_zone(&picked_zone)
            .not_in_ids(vec![card_id.clone()])
            .resolve_ids(state);
        let mut effects = vec![Effect::TapCard {
            card_id: card_id.clone(),
        }];
        for unit_id in unit_ids {
            effects.push(Effect::take_damage(&unit_id, card_id, 4));
        }

        Ok(effects)
    }
}

#[derive(Debug, Clone)]
pub struct AncientDragon {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl AncientDragon {
    pub const NAME: &'static str = "Ancient Dragon";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 6,
                toughness: 6,
                abilities: vec![Ability::Airborne],
                types: vec![MinionType::Dragon],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::from_mana_and_threshold(7, "FFF"),
                region: Region::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for AncientDragon {
    fn get_name(&self) -> &str {
        Self::NAME
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

    fn get_additional_activated_abilities(&self, _state: &State) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(AncientDragonAbility)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (AncientDragon::NAME, |owner_id: PlayerId| {
    Box::new(AncientDragon::new(owner_id))
});
