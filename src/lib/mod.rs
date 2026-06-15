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
        Ability, AdditionalCost, Artifact, ArtifactBase, ArtifactType, Aura, AuraBase, Avatar,
        AvatarBase, Card, CardBase, CardBaseMethods, CardConstructor, CardStatus, CardType, Cost,
        Costs, DEATHRITE_HOOK_ID, Damage, Edition, GENESIS_HOOK_ID, Hook, HookId, HookSourceZones,
        HookTiming, Magic, MinionType, Rarity, Region, ResourceProvider,
        ResourceProviderBaseMethods, Rubble, Site, SiteBase, SiteType, UnitBase,
    };
    pub use crate::effect::{
        AbilityCounter, Counter, DrawKind, Effect, StatusCounter, SummonCard, TokenType,
    };
    pub use crate::game::{
        ActivatedAbility, AvatarAction, BaseOption, CARDINAL_DIRECTIONS, CardId, Direction,
        Element, NO_CONTROLLER, PlayerId, Thresholds, UnitAction, force_sync,
        get_knight_move_locations, pick_action_source, pick_card, pick_card_source,
        pick_card_with_options, pick_card_with_preview, pick_cards, pick_direction_source,
        pick_location, pick_location_near, pick_location_near_source, pick_location_source,
        pick_option, pick_zone_group, pick_zone_group_source, reveal_cards, take_action, yes_or_no,
    };
    pub use crate::query::{CardQuery, EffectQuery, LocationQuery, ZoneQuery};
    pub use crate::state::{
        AbilityRemoval, AffinityModifier, DeferredEffect, LoggedEffect, OngoingEffect, State,
        TemporaryEffect,
    };
    pub use crate::zone::{Location, Zone};
    pub use std::sync::Arc;
}
