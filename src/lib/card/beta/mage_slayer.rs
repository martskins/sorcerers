use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    effect::Effect,
    game::{Element, PlayerId, pick_card},
    state::State,
};

#[derive(Debug, Clone)]
pub struct MageSlayer {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl MageSlayer {
    pub const NAME: &'static str = "Mage Slayer";
    pub const DESCRIPTION: &'static str = "Genesis → Kill target Spellcaster minion nearby.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "E"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for MageSlayer {
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
        let my_zone = self.get_zone().clone();

        let spellcaster_abilities = [
            Ability::Spellcaster(None),
            Ability::Spellcaster(Some(Element::Fire)),
            Ability::Spellcaster(Some(Element::Air)),
            Ability::Spellcaster(Some(Element::Earth)),
            Ability::Spellcaster(Some(Element::Water)),
        ];

        let enemy_spellcasters: Vec<uuid::Uuid> = state
            .cards
            .iter()
            .filter(|c| c.is_minion())
            .filter(|c| c.get_controller_id(state) != controller_id)
            .filter(|c| c.get_zone().is_nearby(&my_zone))
            .filter(|c| {
                spellcaster_abilities
                    .iter()
                    .any(|a| c.has_ability(state, a))
            })
            .map(|c| *c.get_id())
            .collect();

        if enemy_spellcasters.is_empty() {
            return Ok(vec![]);
        }

        let chosen = pick_card(
            &controller_id,
            &enemy_spellcasters,
            state,
            "Mage Slayer: Pick a nearby enemy Spellcaster to kill",
        )
        .await?;

        Ok(vec![Effect::KillMinion {
            card_id: chosen,
            killer_id: *self.get_id(),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (MageSlayer::NAME, |owner_id: PlayerId| {
    Box::new(MageSlayer::new(owner_id))
});
