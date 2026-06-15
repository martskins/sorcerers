use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct SimpleVillage {
    site_base: SiteBase,
    card_base: CardBase,
}

impl SimpleVillage {
    pub const NAME: &'static str = "Simple Village";
    pub const DESCRIPTION: &'static str =
        "Genesis → You may pay ① to summon a Foot Soldier token here.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("E"),
                types: vec![SiteType::Village],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
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
impl Site for SimpleVillage {}

impl ResourceProvider for SimpleVillage {}

#[async_trait::async_trait]
impl Card for SimpleVillage {
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

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
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
                let options = [BaseOption::Yes, BaseOption::No];
                let option_labels: Vec<String> = options.iter().map(|o| o.to_string()).collect();
                let picked_option = pick_option(
                    self.get_controller_id(state),
                    &option_labels,
                    state,
                    "Humble Village: Pay 1 to summon a foot soldier?",
                    false,
                )
                .await?;
                if options[picked_option] == BaseOption::No {
                    return Ok(vec![]);
                }

                Ok(vec![
                    Effect::AdjustMana {
                        player_id: self.get_controller_id(state),
                        amount: -1,
                    },
                    Effect::SummonToken {
                        player_id: self.get_controller_id(state),
                        token_type: TokenType::FootSoldier,
                        location: self.get_location().clone(),
                    },
                ])
            }
            _ => Ok(vec![]),
        }
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (SimpleVillage::NAME, |owner_id: PlayerId| {
        Box::new(SimpleVillage::new(owner_id))
    });
