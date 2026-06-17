use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Drown {
    card_base: CardBase,
}

impl Drown {
    pub const NAME: &'static str = "Drown";
    pub const DESCRIPTION: &'static str = "Submerge target minion or artifact, if able.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "W"),
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
impl Card for Drown {
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
impl Magic for Drown {
    async fn resolve_magic(
        &self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let Some(picked_card_id) = CardQuery::new()
            .card_types(vec![CardType::Minion, CardType::Artifact])
            .in_region(Region::Surface)
            .with_prompt("Pick a minion or artifact to submerge")
            .with_source_card(*self.get_id())
            .pick(&controller_id, state)
            .await?
        else {
            return Ok(vec![]);
        };

        Ok(vec![Effect::SetCardRegion {
            card_id: picked_card_id,
            destination: Region::Underwater,
            tap: false,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Drown::NAME, |owner_id: PlayerId| {
    Box::new(Drown::new(owner_id))
});
