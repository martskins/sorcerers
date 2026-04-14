use crate::{
    card::{AvatarBase, Card, CardBase, Cost, Costs, Edition, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::{ActivatedAbility, AvatarAction, PlayerId},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
struct DeathspeakerAbility;

#[async_trait::async_trait]
impl ActivatedAbility for DeathspeakerAbility {
    fn get_name(&self) -> String {
        "Banish a dead minion".to_string()
    }

    async fn on_select(
        &self,
        _card_id: &uuid::Uuid,
        _player_id: &PlayerId,
        _state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        // TODO: Implement the banish and copy effect.
        //  - Add a copy card effect that creates a new instance of the given card, except it's a token.
        //  - Have the player decide where to summon the copy.
        //  - Check if the player is on Death's Door, and if so, make the cost of the copy 0.
        //  - Banish the original card.
        //  - Banish the copy after it enters the realm and uses its Genesis ability.
        Ok(vec![])
    }

    fn can_activate(
        &self,
        _card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<bool> {
        Ok(CardQuery::new()
            .dead()
            .minions()
            .controlled_by(player_id)
            .all(state)
            .len()
            > 0)
    }

    fn get_cost(&self, _card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        // TODO: Implement this
        Ok(Cost::ZERO)
    }
}

#[derive(Debug, Clone)]
pub struct Deathspeaker {
    pub card_base: CardBase,
    pub unit_base: UnitBase,
    pub avatar_base: AvatarBase,
}

impl Deathspeaker {
    pub const NAME: &'static str = "Deathspeaker";
    pub const DESCRIPTION: &'static str = "Tap → Play or draw a site.\r \r You may banish a dead minion each turn to cast a copy of it, and for (0) if you're on Death's Door. The copy enters the realm, uses its Genesis, then is banished.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 20,
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::ZERO,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
            avatar_base: AvatarBase {
                ..Default::default()
            },
        }
    }
}

impl Card for Deathspeaker {
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

    fn get_avatar_base(&self) -> Option<&AvatarBase> {
        Some(&self.avatar_base)
    }

    fn get_avatar_base_mut(&mut self) -> Option<&mut AvatarBase> {
        Some(&mut self.avatar_base)
    }

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(DeathspeakerAbility)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Deathspeaker::NAME, |owner_id: PlayerId| {
        Box::new(Deathspeaker::new(owner_id))
    });
