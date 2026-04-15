use crate::{
    card::{
        Ability, AreaModifiers, Aura, AuraBase, Card, CardBase, Costs, Edition, Rarity, Region,
        Zone,
    },
    effect::Effect,
    game::PlayerId,
    state::State,
};

#[derive(Debug, Clone)]
pub struct EntangleTerrain {
    aura_base: AuraBase,
    card_base: CardBase,
}

impl EntangleTerrain {
    pub const NAME: &'static str = "Entangle Terrain";
    pub const DESCRIPTION: &'static str =
        "Minions occupying affected sites lose Airborne and are Immobile. Lasts 3 of your turns.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "EE"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
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

impl Aura for EntangleTerrain {
    fn should_dispell(&self, state: &State) -> anyhow::Result<bool> {
        let controller_id = self.get_controller_id(state);
        let turns_in_play = state
            .effect_log
            .iter()
            .skip_while(|e| match *e.effect {
                Effect::PlayCard { ref card_id, .. } if card_id == self.get_id() => false,
                _ => true,
            })
            .filter(|e| match *e.effect {
                Effect::EndTurn { ref player_id, .. } if player_id == &controller_id => true,
                _ => false,
            })
            .count();

        Ok(turns_in_play >= 3)
    }
}

#[async_trait::async_trait]
impl Card for EntangleTerrain {
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
        let minions: Vec<uuid::Uuid> = self
            .get_affected_zones(state)
            .iter()
            .filter(|zone| zone.get_site(state).is_some())
            .flat_map(|zone| zone.get_minions(state, None))
            .map(|minion| minion.get_id().clone())
            .collect();
        AreaModifiers {
            grants_abilities: minions
                .iter()
                .map(|id| (id.clone(), vec![Ability::Immobile]))
                .collect(),
            removes_abilities: minions
                .iter()
                .map(|id| (id.clone(), vec![Ability::Airborne]))
                .collect(),
            ..Default::default()
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (EntangleTerrain::NAME, |owner_id: PlayerId| {
        Box::new(EntangleTerrain::new(owner_id))
    });
