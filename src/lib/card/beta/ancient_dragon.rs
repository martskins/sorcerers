use crate::{
    card::{
        Ability, AdditionalCost, Card, CardBase, CardConstructor, Cost, Costs, Edition, MinionType,
        Rarity, Region, UnitBase, Zone,
    },
    effect::Effect,
    game::{ActivatedAbility, PlayerId, pick_zone_near},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
struct AncientDragonAbility;

#[async_trait::async_trait]
impl ActivatedAbility for AncientDragonAbility {
    fn get_name(&self) -> String {
        "Tap to deal 4 damage".to_string()
    }

    fn get_cost(&self, card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::tap(card_id)))
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
        let unit_ids = CardQuery::new()
            .in_zone(&picked_zone)
            .units()
            .id_not_in(vec![*card_id])
            .all(state);
        let mut effects = vec![];
        for unit_id in unit_ids {
            effects.push(Effect::TakeDamage {
                card_id: unit_id,
                from: *card_id,
                damage: 4,
                is_strike: false,
                is_ranged: false,
            });
        }

        Ok(effects)
    }
}

#[derive(Debug, Clone)]
pub struct AncientDragon {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl AncientDragon {
    pub const NAME: &'static str = "Ancient Dragon";
    pub const DESCRIPTION: &'static str =
        "Airborne\r \r Tap → Deal 4 damage to each other unit at target location nearby.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 6,
                toughness: 6,
                abilities: vec![Ability::Airborne],
                types: vec![MinionType::Dragon],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(7, "FFF"),
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
impl Card for AncientDragon {
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
        Ok(vec![Box::new(AncientDragonAbility)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (AncientDragon::NAME, |owner_id: PlayerId| {
        Box::new(AncientDragon::new(owner_id))
    });
