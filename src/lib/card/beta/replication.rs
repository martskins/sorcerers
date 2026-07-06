use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Replication {
    card_base: CardBase,
}

impl Replication {
    pub const NAME: &'static str = "Replication";
    pub const DESCRIPTION: &'static str = "Conjure a copy of an artifact carried by the caster.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "W"),
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
impl Card for Replication {
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
impl Magic for Replication {
    async fn resolve_magic(
        &self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let player_id = self.get_controller_id(state);
        let Some(picked_artifact_id) = CardQuery::new()
            .artifacts()
            .carried_by(caster_id)
            .with_prompt("Pick an artifact to replicate")
            .with_source_card(*self.get_id())
            .pick(&player_id, state)
            .await?
        else {
            return Ok(vec![]);
        };

        Ok(vec![Effect::CopyArtifact {
            player_id,
            artifact_id: picked_artifact_id,
            bearer_id: None,
            caster_id: *caster_id,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Replication::NAME, |owner_id: PlayerId| {
    Box::new(Replication::new(owner_id))
});
