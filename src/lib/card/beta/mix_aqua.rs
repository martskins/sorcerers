use crate::card::CardType;
use crate::{
    card::{
        Artifact, ArtifactBase, ArtifactType, Card, CardBase, CardConstructor, Costs, Edition,
        Rarity, Region, Zone,
    },
    effect::Effect,
    game::{ActivatedAbility, Element, PlayerId},
    query::EffectQuery,
    state::{CardQuery, State, TemporaryEffect},
};

#[derive(Debug, Clone)]
struct SacrificeForWaterSpell;

#[async_trait::async_trait]
impl ActivatedAbility for SacrificeForWaterSpell {
    fn get_name(&self) -> String {
        "Sacrifice Mix Aqua".to_string()
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        _state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let water_spells = CardQuery::new()
            .with_element(Element::Water)
            .card_types(vec![CardType::Magic])
            .including_not_in_play();

        Ok(vec![
            Effect::BuryCard { card_id: *card_id },
            Effect::AddTemporaryEffect {
                effect: TemporaryEffect::IgnoreCostThresholds {
                    affected_cards: water_spells,
                    expires_on_effect: EffectQuery::PlayCard {
                        card: CardQuery::new()
                            .with_element(Element::Water)
                            .card_types(vec![CardType::Magic])
                            .including_not_in_play(),
                    },
                    for_player: *player_id,
                },
            },
        ])
    }
}

#[derive(Debug, Clone)]
pub struct MixAqua {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl MixAqua {
    pub const NAME: &'static str = "Mix Aqua";
    pub const DESCRIPTION: &'static str = "Sacrifice Mix Aqua → This turn, bearer's next Water spell requires no threshold and costs ③ less to cast.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                needs_bearer: true,
                types: vec![ArtifactType::Relic],
                tapped: false,
                region: Region::Surface,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(1, ""),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for MixAqua {}

#[async_trait::async_trait]
impl Card for MixAqua {
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

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(SacrificeForWaterSpell)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (MixAqua::NAME, |owner_id: PlayerId| {
    Box::new(MixAqua::new(owner_id))
});
