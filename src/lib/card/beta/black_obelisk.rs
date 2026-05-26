use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct BlackObelisk {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl BlackObelisk {
    pub const NAME: &'static str = "Black Obelisk";
    pub const DESCRIPTION: &'static str =
        "Black Obelisk's site has “At the start of your turn, lose 2 life and gain ② this turn.”";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                types: vec![ArtifactType::Monument],
                tapped: false,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(3),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for BlackObelisk {}

#[async_trait::async_trait]
impl Card for BlackObelisk {
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

    fn get_artifact_base(&self) -> Option<&ArtifactBase> {
        Some(&self.artifact_base)
    }

    fn get_artifact_base_mut(&mut self) -> Option<&mut ArtifactBase> {
        Some(&mut self.artifact_base)
    }

    fn get_artifact(&self) -> Option<&dyn Artifact> {
        Some(self)
    }

    async fn get_continuous_effects(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<ContinuousEffect>> {
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        Ok(vec![ContinuousEffect::ModifyProvidedMana {
            mana_diff: 2,
            affected_cards: CardQuery::new().in_zone_of_card(self.get_id()).sites(),
        }])
    }

    async fn on_turn_start(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let Some(site): Option<&dyn Site> = self.get_zone().get_site(state) else {
            return Ok(vec![]);
        };

        let controller_id = site.get_controller_id(state);
        if controller_id != state.current_player() {
            return Ok(vec![]);
        }

        if site.has_status(state, &CardStatus::Disabled) {
            return Ok(vec![]);
        }

        let avatar_id = state.get_player_avatar_id(&controller_id)?;
        Ok(vec![Effect::TakeDamage {
            card_id: avatar_id,
            from: *site.get_id(),
            damage: Damage::basic(2),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (BlackObelisk::NAME, |owner_id: PlayerId| {
    Box::new(BlackObelisk::new(owner_id))
});
