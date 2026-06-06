use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Boneyard {
    site_base: SiteBase,
    card_base: CardBase,
}

impl Boneyard {
    pub const NAME: &'static str = "Boneyard";
    pub const DESCRIPTION: &'static str =
        "Genesis → Each player may summon a minion from their cemetery here.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::ZERO,
                types: vec![],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
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
impl Site for Boneyard {}

impl ResourceProvider for Boneyard {}

#[async_trait::async_trait]
impl Card for Boneyard {
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

    fn get_site_base(&self) -> Option<&SiteBase> {
        Some(&self.site_base)
    }

    fn get_site_base_mut(&mut self) -> Option<&mut SiteBase> {
        Some(&mut self.site_base)
    }

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }

    async fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook::genesis(self.get_id())])
    }

    async fn resolve_hook(
        &self,
        hook: HookId,
        state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            GENESIS_HOOK_ID => {
        let mut cards = vec![];
        for player in &state.players {
            let player_id = player.id;
            let state = state.clone();
            let zone = self.get_zone().clone();

            let all_cards = &CardQuery::new()
                .in_zone(&Zone::Cemetery)
                .controlled_by(&player_id)
                .all(&state);
            let minions = &CardQuery::new()
                .in_zone(&Zone::Cemetery)
                .minions()
                .controlled_by(&player_id)
                .all(&state);
            let picked_minion_id = pick_card_with_options(
                &player_id,
                minions,
                all_cards,
                true,
                &state,
                "Pick a minion in your cemetery to summon to Boneyard",
            )
            .await?;

            cards.push((
                player_id,
                picked_minion_id,
                Zone::Cemetery,
                zone.into_location()
                    .expect("Boneyard summon target must be a location"),
            ));
        }

        Ok(vec![Effect::SummonCards { cards }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Boneyard::NAME, |owner_id: PlayerId| {
    Box::new(Boneyard::new(owner_id))
});
