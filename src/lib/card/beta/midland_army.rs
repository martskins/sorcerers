use crate::{
    card::{
        AdditionalCost, Card, CardBase, CardConstructor, Cost, Costs, Edition, MinionType, Rarity,
        Region, UnitBase, Zone,
    },
    effect::{Effect, TokenType},
    game::{ActivatedAbility, PlayerId, pick_zone},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
struct ArtilleryBarrage;

#[async_trait::async_trait]
impl ActivatedAbility for ArtilleryBarrage {
    fn get_name(&self) -> String {
        "Artillery Barrage".to_string()
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
        let zones = card.get_zones_within_steps(state, 3);

        let target_zone = pick_zone(
            player_id,
            &zones,
            state,
            false,
            "Midland Army: Pick a zone to bombard (up to 3 steps away)",
        )
        .await?;

        let effects = CardQuery::new()
            .units()
            .in_zone(&target_zone)
            .all(state)
            .into_iter()
            .map(|unit_id| Effect::TakeDamage {
                card_id: unit_id,
                from: *card_id,
                damage: 4,
                is_strike: false,
            })
            .collect();

        Ok(effects)
    }
}

#[derive(Debug, Clone)]
pub struct MidlandArmy {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl MidlandArmy {
    pub const NAME: &'static str = "Midland Army";
    pub const DESCRIPTION: &'static str =
        "Tap → Each unit within 3 steps takes 4 damage.\r \r Deathrite → Summon a Foot Soldier at each adjacent site.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 8,
                toughness: 8,
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(8, "EEEE"),
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
impl Card for MidlandArmy {
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
        Ok(vec![Box::new(ArtilleryBarrage)])
    }

    fn deathrite(&self, state: &State, from: &Zone) -> Vec<Effect> {
        let controller_id = self.get_controller_id(state);
        from.get_adjacent()
            .into_iter()
            .filter(|z| z.get_site(state).is_some())
            .map(|zone| Effect::SummonToken {
                player_id: controller_id,
                token_type: TokenType::FootSoldier,
                zone,
            })
            .collect()
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (MidlandArmy::NAME, |owner_id: PlayerId| {
    Box::new(MidlandArmy::new(owner_id))
});
