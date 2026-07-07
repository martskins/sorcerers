use crate::prelude::*;

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

    fn get_magic(&self) -> Option<&dyn Magic> {
        Some(self)
    }
}

#[async_trait::async_trait]
impl Magic for Geyser {
    async fn resolve_magic(
        &self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let player_id = self.get_controller_id(state);
        let Some(target_site_id) = CardQuery::new()
            .sites()
            .count(1)
            .with_source_card(*self.get_id())
            .with_prompt("Pick a site to flood")
            .pick(&player_id, state)
            .await?
        else {
            return Ok(vec![]);
        };

        let target_site = state.get_card(&target_site_id);
        let minions_there = CardQuery::new()
            .minions()
            .occupying_site_at_location(target_site.get_location().clone())
            .all(state);

        let mut effects = vec![
            Effect::DrawCard {
                player_id,
                count: 1,
                kind: DrawKind::Choice,
            },
            Effect::AddTemporaryEffect {
                effect: TemporaryEffect::GrantAbility {
                    ability: Ability::Flooded,
                    affected_cards: target_site_id.into(),
                    expires_on_effect: EffectQuery::TurnEnd { player_id: None },
                },
            },
        ];

        for minion_id in minions_there {
            effects.push(Effect::AddAbilityCounter {
                card_id: minion_id,
                counter: AbilityCounter {
                    id: uuid::Uuid::new_v4(),
                    ability: Ability::Airborne,
                    expires_on_effect: Some(EffectQuery::TurnEnd { player_id: None }),
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
