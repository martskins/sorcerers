use crate::{
    card::{Ability, Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::{AbilityCounter, Effect},
    game::PlayerId,
    query::EffectQuery,
    state::{CardQuery, State, TemporaryEffect},
};

#[derive(Debug, Clone)]
pub struct Geyser {
    card_base: CardBase,
}

impl Geyser {
    pub const NAME: &'static str = "Geyser";
    pub const DESCRIPTION: &'static str =
        "This turn, flood target site and give each minion there Airborne. Draw a card.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "W"),
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
impl Card for Geyser {
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
        let player_id = self.get_controller_id(state);
        let Some(target_site_id) = CardQuery::new()
            .sites()
            .count(1)
            .pick(&player_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };

        let target_site = state.get_card(&target_site_id);
        let minions_there = CardQuery::new()
            .minions()
            .in_zone(target_site.get_zone())
            .all(state);

        let mut effects = vec![
            Effect::DrawCard {
                player_id,
                count: 1,
            },
            Effect::AddTemporaryEffect {
                effect: TemporaryEffect::FloodSites {
                    affected_sites: target_site_id.into(),
                    expires_on_effect: EffectQuery::TurnEnd {
                        player_id: Some(player_id),
                    },
                },
            },
        ];

        for minion_id in minions_there {
            effects.push(Effect::AddAbilityCounter {
                card_id: minion_id,
                counter: AbilityCounter {
                    id: uuid::Uuid::new_v4(),
                    ability: Ability::Airborne,
                    expires_on_effect: Some(EffectQuery::TurnEnd {
                        player_id: Some(player_id),
                    }),
                },
            });
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Geyser::NAME, |owner_id: PlayerId| {
    Box::new(Geyser::new(owner_id))
});
