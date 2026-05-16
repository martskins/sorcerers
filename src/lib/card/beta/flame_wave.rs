use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct FlameWave {
    card_base: CardBase,
}

impl FlameWave {
    pub const NAME: &'static str = "Flame Wave";
    pub const DESCRIPTION: &'static str = "Flame Wave flows horizontally, from one edge of the realm to the other. Deal damage to each unit atop sites in the area of effect:";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(6, "FF"),
                rarity: crate::card::Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for FlameWave {
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

    async fn on_cast(
        &mut self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let spell_id = *self.get_id();
        let controller_id = self.get_controller_id(state);
        let options = vec![
            "Start at the left edge".to_string(),
            "Start at the right edge".to_string(),
        ];
        let from_left = pick_option(
            &controller_id,
            &options,
            state,
            "Flame Wave: Pick where the wave starts",
            false,
        )
        .await?
            == 0;
        let damage_by_distance = [7, 5, 3, 1, 1];
        let all_units = CardQuery::new()
            .units()
            .in_play()
            .all(state)
            .into_iter()
            .filter(|unit_id| {
                let unit = state.get_card(unit_id);
                unit.get_region(state) == &Region::Surface && unit.get_zone().get_site(state).is_some()
            })
            .collect::<Vec<uuid::Uuid>>();
        let effects = all_units
            .into_iter()
            .filter_map(|unit_id| {
                let square = state.get_card(&unit_id).get_zone().get_square()?;
                let col = ((square - 1) % 5) as usize;
                let distance = if from_left { col } else { 4 - col };
                Some(Effect::TakeDamage {
                    card_id: unit_id,
                    from: spell_id,
                    damage: Damage::basic(damage_by_distance[distance]),
                })
            })
            .collect();
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (FlameWave::NAME, |owner_id: PlayerId| {
    Box::new(FlameWave::new(owner_id))
});
