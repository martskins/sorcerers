use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::Effect,
    game::{Element, PlayerId},
    state::State,
};

#[derive(Debug, Clone)]
pub struct OccultRitual {
    card_base: CardBase,
}

impl OccultRitual {
    pub const NAME: &'static str = "Occult Ritual";
    pub const DESCRIPTION: &'static str = "Gain ② this turn for each allied Spellcaster here.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "A"),
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
impl Card for OccultRitual {
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
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        use crate::card::Ability;

        let controller_id = self.get_controller_id(state);
        let caster = state.get_card(caster_id);
        let caster_zone = caster.get_zone().clone();

        let spellcaster_abilities = [
            Ability::Spellcaster(None),
            Ability::Spellcaster(Some(Element::Fire)),
            Ability::Spellcaster(Some(Element::Air)),
            Ability::Spellcaster(Some(Element::Earth)),
            Ability::Spellcaster(Some(Element::Water)),
        ];

        let count = state
            .cards
            .iter()
            .filter(|c| c.get_zone() == &caster_zone)
            .filter(|c| c.get_controller_id(state) == controller_id)
            .filter(|c| {
                spellcaster_abilities
                    .iter()
                    .any(|a| c.has_ability(state, a))
            })
            .count() as u8;

        if count == 0 {
            return Ok(vec![]);
        }

        Ok(vec![Effect::AddMana {
            player_id: controller_id,
            mana: count * 2,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (OccultRitual::NAME, |owner_id: PlayerId| {
    Box::new(OccultRitual::new(owner_id))
});
