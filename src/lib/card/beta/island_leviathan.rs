use crate::{
    card::{
        Card, CardBase, CardConstructor, Cost, Costs, Edition, MinionType, Rarity, Site, SiteBase,
        UnitBase, Zone,
    },
    effect::{Effect, TokenType},
    game::{ActivatedAbility, PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
struct TransformIntoAMonster;

#[async_trait::async_trait]
impl ActivatedAbility for TransformIntoAMonster {
    fn get_name(&self) -> String {
        "Transform into a Monster".to_string()
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let card = state.get_card(card_id);
        Ok(vec![
            Effect::SetCardData {
                card_id: *card_id,
                data: Box::new(UnitBase {
                    power: 8,
                    toughness: 8,
                    abilities: vec![],
                    damage: 0,
                    power_counters: vec![],
                    ability_counters: vec![],
                    types: vec![MinionType::Monster],
                    ..Default::default()
                }),
            },
            Effect::SummonToken {
                player_id: *player_id,
                token_type: TokenType::Rubble,
                zone: card.get_zone().clone(),
            },
        ])
    }

    fn can_activate(
        &self,
        card_id: &uuid::Uuid,
        _player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<bool> {
        let card = state.get_card(card_id);
        Ok(card.get_site_base().is_some())
    }

    fn get_cost(&self, _card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::thresholds_only("WWWWWWWW"))
    }
}

#[derive(Debug, Clone)]
pub struct IslandLeviathan {
    site_base: Option<SiteBase>,
    card_base: CardBase,
    unit_base: Option<UnitBase>,
}

impl IslandLeviathan {
    pub const NAME: &'static str = "Island Leviathan";
    pub const DESCRIPTION: &'static str =
        "(W)(W)(W)(W)(W)(W)(W)(W) — May transform into a Monster. Place flooded Rubble underneath.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: Some(SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("W"),
                types: vec![],
                ..Default::default()
            }),
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            unit_base: None,
        }
    }
}

impl Site for IslandLeviathan {}

#[async_trait::async_trait]
impl Card for IslandLeviathan {
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

    fn get_site_base(&self) -> Option<&SiteBase> {
        self.site_base.as_ref()
    }

    fn get_site_base_mut(&mut self) -> Option<&mut SiteBase> {
        self.site_base.as_mut()
    }

    fn get_unit_base(&self) -> Option<&UnitBase> {
        self.unit_base.as_ref()
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        self.unit_base.as_mut()
    }

    fn get_site(&self) -> Option<&dyn Site> {
        if self.site_base.is_some() {
            Some(self)
        } else {
            None
        }
    }

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(TransformIntoAMonster)])
    }

    fn set_data(&mut self, data: &Box<dyn std::any::Any + Send + Sync>) -> anyhow::Result<()> {
        if let Some(unit_base) = data.downcast_ref::<UnitBase>() {
            self.unit_base = Some(unit_base.clone());
            self.site_base = None;
        }

        Ok(())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (IslandLeviathan::NAME, |owner_id: PlayerId| {
        Box::new(IslandLeviathan::new(owner_id))
    });
