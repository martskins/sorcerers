use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    effect::Effect,
    game::{PlayerId, take_action},
    state::State,
};

#[derive(Debug, Clone)]
pub struct RiddleSphinx {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl RiddleSphinx {
    pub const NAME: &'static str = "Riddle Sphinx";
    pub const DESCRIPTION: &'static str = "Airborne Genesis → Look at your topmost spell. You may put it on the bottom of your spellbook, then an opponent may exchange your top and bottommost spells. Draw a card.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                abilities: vec![Ability::Airborne],
                types: vec![MinionType::Monster],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(6, "A"),
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
impl Card for RiddleSphinx {
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

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let original_deck = state.get_player_deck(&controller_id)?.clone();
        let Some(top_spell_id) = original_deck.peek_spell() else {
            return Ok(vec![]);
        };

        let mut deck = original_deck.clone();
        let put_on_bottom = take_action(
            &controller_id,
            &[*top_spell_id],
            state,
            "Riddle Sphinx: Viewing the top card of your spellbook",
            "Put it on the bottom of your spellbook?",
        )
        .await?;
        if put_on_bottom {
            deck.rotate_spells(1);
        }

        if deck.spells.len() >= 2 {
            let opponent_id = state.get_opponent_id(&controller_id)?;
            let top_id = *deck.spells.last().expect("deck has at least two spells");
            let bottom_id = *deck.spells.first().expect("deck has at least two spells");
            let exchange = take_action(
                &opponent_id,
                &[top_id, bottom_id],
                state,
                "Riddle Sphinx: You may exchange your opponent's top and bottommost spells",
                "Exchange them?",
            )
            .await?;
            if exchange {
                let last_idx = deck.spells.len() - 1;
                deck.spells.swap(0, last_idx);
            }
        }

        let mut effects = vec![];
        if deck.spells != original_deck.spells {
            effects.push(Effect::RearrangeDeck {
                spells: deck.spells,
                sites: deck.sites,
            });
        }
        effects.push(Effect::DrawSpell {
            player_id: controller_id,
            count: 1,
        });
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (RiddleSphinx::NAME, |owner_id: PlayerId| {
    Box::new(RiddleSphinx::new(owner_id))
});
