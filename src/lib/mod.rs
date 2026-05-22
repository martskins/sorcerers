#[allow(clippy::needless_update)]
pub mod card;
pub mod deck;
pub mod effect;
pub mod error;
pub mod evaluation;
pub mod game;
pub mod networking;
pub mod query;
pub mod state;
pub mod zone;

#[cfg(test)]
mod effect_test;
#[cfg(test)]
mod game_test;
#[cfg(test)]
mod state_test;
#[cfg(test)]
mod zone_test;

pub(crate) mod prelude {
    pub use crate::card::{
        Ability, AdditionalCost, AreaModifiers, Artifact, ArtifactBase, ArtifactType, Aura,
        AuraBase, AvatarBase, Card, CardBase, CardBaseMethods, CardConstructor, CardType, Cost,
        CostType, Costs, Damage, Edition, MinionType, Rarity, Region, ResourceProvider,
        ResourceProviderBaseMethods, Rubble, Site, SiteBase, SiteType, UnitBase,
    };
    pub use crate::effect::{AbilityCounter, Counter, Effect, TokenType};
    pub use crate::game::{
        ActivatedAbility, AvatarAction, BaseAction, BaseOption, CARDINAL_DIRECTIONS, Direction,
        Element, NO_CONTROLLER, PlayerId, Thresholds, UnitAction, force_sync, get_knight_move_zones,
        pick_action_source, pick_card, pick_card_source, pick_card_with_options,
        pick_card_with_preview, pick_cards, pick_direction_source, pick_option,
        pick_option_source, pick_zone, pick_zone_group, pick_zone_group_source, pick_zone_near,
        pick_zone_source, reveal_cards, take_action, yes_or_no,
    };
    pub use crate::query::{CardQuery, EffectQuery, ZoneQuery};
    pub use crate::state::{
        ContinuousEffect, DeferredEffect, LoggedEffect, State, TemporaryEffect,
    };
    pub use crate::zone::Zone;
}
