use crate::{
    card::{
        Ability, AreaModifiers, Aura, AuraBase, Card, CardBase, CardConstructor, Costs, Edition,
        Rarity, Region, Zone,
    },
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Blizzard {
    aura_base: AuraBase,
    card_base: CardBase,
}

impl Blizzard {
    pub const NAME: &'static str = "Blizzard";
    pub const DESCRIPTION: &'static str = "Affected sites and units atop them can't be attacked or intercepted.\r \r At the start of your turn, dispel Blizzard.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(1, "W"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            aura_base: AuraBase {
                tapped: false,
                region: Region::Surface,
            },
        }
    }
}

impl Aura for Blizzard {
    fn should_dispell(&self, state: &State) -> anyhow::Result<bool> {
        let controller_id = self.get_controller_id(state);
        let turns_in_play = state
            .effect_log
            .iter()
            .skip_while(|e| !matches!(*e.effect, Effect::PlayCard { ref card_id, .. } if card_id == self.get_id()))
            .filter(|e| matches!(*e.effect, Effect::StartTurn { ref player_id, .. } if player_id == &controller_id))
            .count();

        Ok(turns_in_play >= 1)
    }

    fn get_affected_zones(&self, state: &State) -> Vec<Zone> {
        let affected_zones = self.base_get_affected_zones(state);
        affected_zones
            .into_iter()
            .filter(|z| z.get_site(state).is_some())
            .collect()
    }
}

#[async_trait::async_trait]
impl Card for Blizzard {
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

    fn get_aura_base(&self) -> Option<&AuraBase> {
        Some(&self.aura_base)
    }
    fn get_aura_base_mut(&mut self) -> Option<&mut AuraBase> {
        Some(&mut self.aura_base)
    }

    fn get_aura(&self) -> Option<&dyn Aura> {
        Some(self)
    }

    fn area_modifiers(&self, state: &State) -> AreaModifiers {
        let affected_sites = self.get_affected_zones(state);
        let minions = CardQuery::new()
            .minions()
            .in_zones(&affected_sites)
            .all(state);

        AreaModifiers {
            grants_abilities: minions
                .iter()
                .map(|id| (*id, vec![Ability::Unattackable, Ability::Uninterceptable]))
                .collect(),
            ..Default::default()
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Blizzard::NAME, |owner_id: PlayerId| {
    Box::new(Blizzard::new(owner_id))
});
