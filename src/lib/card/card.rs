use crate::{
    card::beta,
    effect::{Counter, Effect, ModifierCounter},
    game::{
        Action, AvatarAction, Direction, Element, PlayerId, Thresholds, UnitAction, are_adjacent, are_nearby,
        get_adjacent_zones, get_nearby_zones,
    },
    state::State,
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CardType {
    Site,
    Avatar,
    Token,
    Minion,
    Magic,
    Aura,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Edition {
    Alpha,
    Beta,
    ArthurianLegends,
    Dragonlord,
    Gothic,
}

impl Edition {
    pub fn url_name(&self) -> &str {
        match self {
            Edition::Alpha => "alp",
            Edition::Beta => "bet",
            Edition::ArthurianLegends => "art",
            Edition::Dragonlord => "drg",
            Edition::Gothic => "got",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Plane {
    None,
    Air,
    Surface,
    Burrowed,
    Submerged,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Zone {
    None,
    Hand,
    Spellbook,
    Atlasbook,
    Realm(u8),
    Cemetery,
    Banish,
}

impl Zone {
    pub fn all_realm() -> Vec<Zone> {
        (1..=20).map(|sq| Zone::Realm(sq)).collect()
    }

    pub fn get_square(&self) -> Option<u8> {
        match self {
            Zone::Realm(sq) => Some(*sq),
            _ => None,
        }
    }

    pub fn is_nearby(&self, other: &Zone) -> bool {
        are_nearby(self, other)
    }

    pub fn is_adjacent(&self, other: &Zone) -> bool {
        are_adjacent(self, other)
    }

    pub fn get_nearby(&self) -> Vec<Zone> {
        get_nearby_zones(self)
    }

    pub fn get_units<'a>(&self, state: &'a State, owner_id: Option<&uuid::Uuid>) -> Vec<&'a Box<dyn Card>> {
        state
            .get_cards_in_zone(self)
            .iter()
            .filter(|c| c.is_unit())
            .filter(|c| {
                if let Some(owner_id) = owner_id {
                    c.get_owner_id() == owner_id
                } else {
                    true
                }
            })
            .cloned()
            .collect::<Vec<&Box<dyn Card>>>()
    }

    pub fn get_site<'a>(&self, state: &'a State) -> Option<&'a Box<dyn Card>> {
        state.get_cards_in_zone(self).iter().find(|c| c.is_site()).cloned()
    }

    pub fn get_nearby_units<'a>(&self, state: &'a State, owner_id: Option<&uuid::Uuid>) -> Vec<&'a Box<dyn Card>> {
        get_nearby_zones(self)
            .iter()
            .flat_map(|z| {
                state
                    .get_cards_in_zone(z)
                    .iter()
                    .filter(|c| c.is_unit())
                    .filter(|c| {
                        if let Some(owner_id) = owner_id {
                            c.get_owner_id() == owner_id
                        } else {
                            true
                        }
                    })
                    .cloned()
                    .collect::<Vec<&Box<dyn Card>>>()
            })
            .collect()
    }

    pub fn get_nearby_sites<'a>(&self, state: &'a State, owner_id: Option<&uuid::Uuid>) -> Vec<&'a Box<dyn Card>> {
        get_nearby_zones(self)
            .iter()
            .flat_map(|z| {
                state
                    .get_cards_in_zone(z)
                    .iter()
                    .filter(|c| c.is_site())
                    .filter(|c| {
                        if let Some(owner_id) = owner_id {
                            c.get_owner_id() == owner_id
                        } else {
                            true
                        }
                    })
                    .cloned()
                    .collect::<Vec<&Box<dyn Card>>>()
            })
            .collect()
    }

    pub fn zone_in_direction_and_steps(&self, direction: &Direction, steps: u8) -> Option<Self> {
        let mut current_zone = self.clone();
        for _ in 0..steps {
            match current_zone.zone_in_direction(direction) {
                Some(z) => current_zone = z,
                None => return None,
            }
        }
        Some(current_zone)
    }

    pub fn zone_in_direction(&self, direction: &Direction) -> Option<Self> {
        let square = self.get_square().unwrap();
        let zone = match direction {
            Direction::Up => Zone::Realm(square.saturating_add(5)),
            Direction::Down => Zone::Realm(square.saturating_sub(5)),
            Direction::Left => Zone::Realm(square.saturating_sub(1)),
            Direction::Right => Zone::Realm(square.saturating_add(1)),
            Direction::TopLeft => Zone::Realm(square.saturating_add(4)),
            Direction::TopRight => Zone::Realm(square.saturating_add(6)),
            Direction::BottomLeft => Zone::Realm(square.saturating_sub(6)),
            Direction::BottomRight => Zone::Realm(square.saturating_sub(4)),
        };

        match direction {
            Direction::Up | Direction::Down => {
                if zone.get_square() > Some(20) || zone.get_square() < Some(1) {
                    return None;
                }

                Some(zone)
            }
            _ => Some(zone),
        }
    }

    pub fn get_adjacent(&self) -> Vec<Zone> {
        get_adjacent_zones(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderableCard {
    pub id: uuid::Uuid,
    pub name: String,
    pub owner_id: PlayerId,
    pub tapped: bool,
    pub edition: Edition,
    pub zone: Zone,
    pub plane: Plane,
    pub card_type: CardType,
    pub modifiers: Vec<Modifier>,
}

impl RenderableCard {
    pub fn is_site(&self) -> bool {
        self.card_type == CardType::Site
    }

    pub fn is_spell(&self) -> bool {
        self.card_type == CardType::Minion || self.card_type == CardType::Aura || self.card_type == CardType::Magic
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn is_token(&self) -> bool {
        self.card_type == CardType::Token
    }

    pub fn get_edition(&self) -> &Edition {
        &self.edition
    }
}

pub trait CloneBoxedCard {
    fn clone_box(&self) -> Box<dyn Card>;
}

impl<T> CloneBoxedCard for T
where
    T: 'static + Card + Clone,
{
    fn clone_box(&self) -> Box<dyn Card> {
        Box::new(self.clone())
    }
}

#[async_trait::async_trait]
pub trait Card: Debug + Send + Sync + CloneBoxedCard {
    fn get_name(&self) -> &str;
    fn get_edition(&self) -> Edition;
    fn get_owner_id(&self) -> &PlayerId;
    fn is_tapped(&self) -> bool;
    fn get_id(&self) -> &uuid::Uuid;
    fn get_base(&self) -> &CardBase;
    fn get_base_mut(&mut self) -> &mut CardBase;

    fn remove_counter(&mut self, id: &uuid::Uuid) {
        if let Some(ub) = self.get_unit_base_mut() {
            ub.power_counters.retain(|c| &c.id != id);
        }
    }

    fn remove_modifier_counter(&mut self, id: &uuid::Uuid) {
        if let Some(ub) = self.get_unit_base_mut() {
            ub.modifier_counters.retain(|c| &c.id != id);
        }
    }

    /// on_card_enter is used on sites and it emits the effects that happen when a card enters the
    /// site.
    fn on_card_enter(&self, _state: &State, _card_id: &uuid::Uuid) -> Vec<Effect> {
        vec![]
    }

    fn get_controller_id(&self) -> &PlayerId {
        &self.get_base().controller_id
    }

    fn set_controller_id(&mut self, controller_id: &PlayerId) {
        self.get_base_mut().controller_id = controller_id.clone();
    }

    async fn after_attack(&self, _state: &State) -> Vec<Effect> {
        vec![]
    }

    fn set_data(&mut self, _data: &Box<dyn std::any::Any + Send + Sync>) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("set_data not implemented for {}", self.get_name()))
    }

    fn on_defend(&self, state: &State, attacker_id: &uuid::Uuid) -> Vec<Effect> {
        // Only units can retaliate
        if let Some(power) = self.get_power(&state) {
            return vec![Effect::TakeDamage {
                card_id: attacker_id.clone(),
                from: self.get_id().clone(),
                damage: power,
            }];
        }

        vec![]
    }

    fn get_zones_within_steps(&self, state: &State, steps: u8) -> Vec<Zone> {
        let mut visited = Vec::new();
        let mut to_visit = vec![(self.get_zone().clone(), 0)];

        while let Some((current_zone, current_step)) = to_visit.pop() {
            if current_step > steps {
                continue;
            }

            if !visited.contains(&current_zone) {
                visited.push(current_zone.clone());

                if self.has_modifier(state, &Modifier::Airborne) {
                    for nearby in current_zone.get_nearby() {
                        to_visit.push((nearby, current_step + 1));
                    }
                } else {
                    for adjacent in current_zone.get_adjacent() {
                        to_visit.push((adjacent, current_step + 1));
                    }
                }
            }
        }

        if !self.has_modifier(state, &Modifier::Voidwalk) {
            visited = visited
                .iter()
                .filter(|z| z.get_site(state).is_some())
                .cloned()
                .collect();
        }

        visited
    }

    fn set_status(&mut self, _status: &Box<dyn std::any::Any + Send + Sync>) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("set_status not implemented for {}", self.get_name()))
    }

    fn base_take_damage(&mut self, state: &State, from: &uuid::Uuid, damage: u8) -> Vec<Effect> {
        if self.is_unit() {
            // Avatar is a sub-type of unit
            if self.is_avatar() {
                return vec![Effect::RemoveResources {
                    player_id: self.get_owner_id().clone(),
                    mana: 0,
                    thresholds: Thresholds::new(),
                    health: damage,
                }];
            }

            let ub = self.get_unit_base_mut().unwrap();
            ub.damage += damage;

            let attacker = state.cards.iter().find(|c| c.get_id() == from).unwrap();
            if ub.damage >= self.get_toughness(state).unwrap_or(0) || attacker.has_modifier(state, &Modifier::Lethal) {
                return vec![Effect::bury_card(self.get_id(), self.get_zone())];
            }

            return vec![];
        } else if self.is_site() {
            return vec![Effect::RemoveResources {
                player_id: self.get_owner_id().clone(),
                mana: 0,
                thresholds: Thresholds::new(),
                health: damage,
            }];
        }

        vec![]
    }

    fn base_site_on_summon(&self, _state: &State) -> Vec<Effect> {
        vec![Effect::AddResources {
            player_id: self.get_owner_id().clone(),
            mana: self.get_site_base().unwrap().provided_mana,
            thresholds: self.get_site_base().unwrap().provided_thresholds.clone(),
            health: 0,
        }]
    }

    fn default_get_valid_play_zones(&self, state: &State) -> Vec<Zone> {
        match self.get_card_type() {
            CardType::Minion | CardType::Aura => state
                .cards
                .iter()
                .filter(|c| c.get_owner_id() == self.get_owner_id())
                .filter(|c| c.is_site())
                .filter_map(|c| match c.get_zone() {
                    z @ Zone::Realm(_) => Some(z),
                    _ => None,
                })
                .cloned()
                .collect(),
            CardType::Site => {
                let player_id = self.get_owner_id();
                let has_played_site = state
                    .cards
                    .iter()
                    .any(|c| c.get_owner_id() == player_id && c.is_site() && matches!(c.get_zone(), Zone::Realm(_)));
                if !has_played_site {
                    let avatar = state
                        .cards
                        .iter()
                        .find(|c| c.get_owner_id() == player_id && c.is_avatar())
                        .unwrap();
                    match avatar.get_zone() {
                        z @ Zone::Realm(_) => return vec![z.clone()],
                        _ => panic!("Avatar not in realm"),
                    }
                }

                let sites: Vec<&Zone> = state
                    .cards
                    .iter()
                    .filter(|c| c.is_site())
                    .filter_map(|c| match c.get_zone() {
                        z @ Zone::Realm(_) => Some(z),
                        _ => None,
                    })
                    .collect();

                let occupied_squares: Vec<&Zone> = state
                    .cards
                    .iter()
                    .filter(|c| c.get_owner_id() == player_id)
                    .filter(|c| c.is_site())
                    .filter(|c| matches!(c.get_zone(), Zone::Realm(_)))
                    .flat_map(|c| match c.get_zone() {
                        z @ Zone::Realm(_) => vec![z],
                        _ => vec![],
                    })
                    .collect();

                occupied_squares
                    .iter()
                    .flat_map(|c| get_adjacent_zones(c))
                    .filter(|c| !occupied_squares.contains(&c))
                    .filter(|c| !sites.contains(&c))
                    .collect()
            }
            _ => vec![],
        }
    }

    fn get_plane(&self) -> &Plane {
        &self.get_base().plane
    }

    fn get_card_type(&self) -> CardType {
        if self.is_site() {
            CardType::Site
        } else if self.is_avatar() {
            CardType::Avatar
        } else if self.is_aura() {
            CardType::Aura
        } else if self.is_unit() {
            CardType::Minion
        } else {
            CardType::Magic
        }
    }

    fn get_valid_play_zones(&self, state: &State) -> Vec<Zone> {
        self.default_get_valid_play_zones(state)
    }

    fn has_modifier(&self, _state: &State, modifier: &Modifier) -> bool {
        if self
            .get_unit_base()
            .unwrap_or(&UnitBase::default())
            .modifiers
            .contains(&modifier)
        {
            return true;
        }

        self.get_unit_base()
            .unwrap_or(&UnitBase::default())
            .modifier_counters
            .iter()
            .find(|c| &c.modifier == modifier)
            .is_some()
    }

    fn get_elements(&self, state: &State) -> Vec<Element> {
        let mut elements = Vec::new();
        let thresholds = self.get_required_thresholds(state);
        if thresholds.fire > 0 {
            elements.push(Element::Fire);
        }
        if thresholds.water > 0 {
            elements.push(Element::Water);
        }
        if thresholds.earth > 0 {
            elements.push(Element::Earth);
        }
        if thresholds.air > 0 {
            elements.push(Element::Air);
        }
        elements
    }

    fn get_square(&self) -> Option<u8> {
        match self.get_zone() {
            Zone::Realm(sq) => Some(*sq),
            _ => None,
        }
    }

    fn get_valid_move_zones(&self, state: &State) -> Vec<Zone> {
        let mut steps = 0;
        let movement_mods = self
            .get_modifiers(state)
            .into_iter()
            .filter(|m| matches!(m, Modifier::Movement(_)));
        for mov in movement_mods {
            if let Modifier::Movement(s) = mov {
                if s > steps {
                    steps = s;
                }
            }
        }

        self.get_zones_within_steps(state, steps + 1)
    }

    fn get_valid_attack_targets(&self, state: &State) -> Vec<uuid::Uuid> {
        state
            .cards
            .iter()
            .filter(|c| c.get_owner_id() != self.get_owner_id())
            .filter(|c| c.is_unit() || c.is_site())
            .filter(|c| c.get_zone().is_adjacent(&self.get_zone()))
            .map(|c| c.get_id().clone())
            .collect()
    }

    fn get_toughness(&self, _state: &State) -> Option<u8> {
        let base = self.get_unit_base();
        if base.is_none() {
            return None;
        }

        let base = base.unwrap();
        let mut toughness = base.toughness;
        for counter in &base.power_counters {
            toughness = toughness.saturating_add_signed(counter.toughness);
        }
        Some(toughness)
    }

    fn get_modifiers(&self, state: &State) -> Vec<Modifier> {
        self.base_get_modifiers(state)
    }

    fn base_get_modifiers(&self, state: &State) -> Vec<Modifier> {
        let base = self.get_unit_base();
        if base.is_none() {
            return vec![];
        }

        let base = base.unwrap();
        let mut modifiers = base.modifiers.clone();
        for counter in &base.modifier_counters {
            modifiers.push(counter.modifier.clone());
        }

        let area_mods: Vec<Modifier> = state
            .cards
            .iter()
            .filter(|c| matches!(c.get_zone(), Zone::Realm(_)))
            .flat_map(|c| c.area_modifiers(state))
            .filter_map(|(modif, units)| {
                if units.contains(self.get_id()) {
                    Some(modif)
                } else {
                    None
                }
            })
            .collect();

        modifiers.extend(area_mods);
        modifiers
    }

    fn base_get_power(&self, _state: &State) -> Option<u8> {
        let base = self.get_unit_base();
        if base.is_none() {
            return None;
        }

        let base = base.unwrap();
        let mut power = base.power;
        for counter in &base.power_counters {
            power = power.saturating_add_signed(counter.power);
        }
        Some(power)
    }

    fn get_power(&self, state: &State) -> Option<u8> {
        self.base_get_power(state)
    }

    fn get_required_thresholds(&self, _state: &State) -> &Thresholds {
        &self.get_base().required_thresholds
    }

    fn get_mana_cost(&self, _state: &State) -> u8 {
        self.get_base().mana_cost
    }

    fn get_avatar_base(&self) -> Option<&AvatarBase> {
        None
    }

    fn get_avatar_base_mut(&mut self) -> Option<&mut AvatarBase> {
        None
    }

    fn get_site_base(&self) -> Option<&SiteBase> {
        None
    }

    fn get_site_base_mut(&mut self) -> Option<&mut SiteBase> {
        None
    }

    fn get_aura_base(&self) -> Option<&AuraBase> {
        None
    }

    fn get_unit_base(&self) -> Option<&UnitBase> {
        None
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        None
    }

    fn get_zone(&self) -> &Zone {
        &self.get_base().zone
    }

    fn set_zone(&mut self, zone: Zone) {
        self.get_base_mut().zone = zone;
    }

    async fn genesis(&self, _state: &State) -> Vec<Effect> {
        vec![]
    }

    fn deathrite(&self, _state: &State, _from: &Zone) -> Vec<Effect> {
        if self.is_site() {
            return vec![Effect::RemoveResources {
                player_id: self.get_owner_id().clone(),
                mana: 0,
                thresholds: self.get_site_base().unwrap().provided_thresholds.clone(),
                health: 0,
            }];
        }

        vec![]
    }

    fn is_token(&self) -> bool {
        false
    }

    fn is_site(&self) -> bool {
        self.get_site_base().is_some()
    }

    fn is_avatar(&self) -> bool {
        self.get_avatar_base().is_some()
    }

    fn is_unit(&self) -> bool {
        self.get_unit_base().is_some()
    }

    fn is_minion(&self) -> bool {
        self.is_unit() && !self.is_avatar()
    }

    fn is_aura(&self) -> bool {
        self.get_aura_base().is_some()
    }

    fn can_cast(&self, state: &State, spell: &Box<dyn Card>) -> bool {
        if !matches!(self.get_zone(), Zone::Realm(_)) {
            return false;
        }

        if self.get_owner_id() != spell.get_owner_id() {
            return false;
        }

        if self.is_avatar() {
            return true;
        }

        let elements = spell.get_elements(state);
        for element in elements {
            if self.has_modifier(state, &Modifier::Spellcaster(element)) {
                return true;
            }
        }

        false
    }

    async fn on_move(&self, state: &State, from: &Zone, to: &Zone) -> Vec<Effect> {
        state
            .cards
            .iter()
            .flat_map(|c| c.get_modifiers(state))
            .flat_map(|m| m.on_move(self.get_id(), state, from, to))
            .collect()
    }

    async fn on_visit_zone(&self, _state: &State, _to: &Zone) -> Vec<Effect> {
        vec![]
    }

    fn on_take_damage(&mut self, state: &State, from: &uuid::Uuid, damage: u8) -> Vec<Effect> {
        self.base_take_damage(state, from, damage)
    }

    async fn on_turn_start(&self, _state: &State) -> Vec<Effect> {
        vec![]
    }

    async fn on_turn_end(&self, _state: &State) -> Vec<Effect> {
        vec![]
    }

    fn remove_modifier(&mut self, modifier: Modifier) {
        if let Some(ub) = self.get_unit_base_mut() {
            ub.modifiers.retain(|a| a != &modifier);
        }
    }

    fn add_modifier(&mut self, modifier: Modifier) {
        if let Some(ub) = self.get_unit_base_mut() {
            ub.modifiers.push(modifier);
        }
    }

    fn on_summon(&mut self, state: &State) -> Vec<Effect> {
        if self.is_site() {
            return self.base_site_on_summon(state);
        }

        vec![]
    }

    async fn on_cast(&mut self, _state: &State, _caster_id: &uuid::Uuid) -> Vec<Effect> {
        vec![]
    }

    fn base_avatar_actions(&self, state: &State) -> Vec<Box<dyn Action>> {
        let mut actions: Vec<Box<dyn Action>> = self.base_unit_actions(state);
        actions.push(Box::new(AvatarAction::DrawSite));
        if state
            .cards
            .iter()
            .filter(|c| c.get_owner_id() == self.get_owner_id())
            .filter(|c| matches!(c.get_zone(), Zone::Hand))
            .count()
            > 0
        {
            actions.push(Box::new(AvatarAction::PlaySite));
        }
        actions
    }

    fn base_unit_actions(&self, state: &State) -> Vec<Box<dyn Action>> {
        let mut actions: Vec<Box<dyn Action>> = vec![Box::new(UnitAction::Attack), Box::new(UnitAction::Move)];
        if self.has_modifier(state, &Modifier::Ranged) {
            actions.push(Box::new(UnitAction::RangedAttack));
        }
        actions
    }

    fn get_actions(&self, state: &State) -> Vec<Box<dyn Action>> {
        if self.is_avatar() {
            return self.base_avatar_actions(state);
        } else if self.is_unit() {
            return self.base_unit_actions(state);
        }

        vec![]
    }

    fn area_modifiers(&self, _state: &State) -> Vec<(Modifier, Vec<uuid::Uuid>)> {
        vec![]
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum MinionType {
    Monster,
    Goblin,
    Ogre,
    Mortal,
    Demon,
    Dragon,
    Fairy,
    Beast,
    Spirit,
    Undead,
}

#[derive(Debug, PartialEq, Clone)]
pub enum SiteType {
    Desert,
    Tower,
}

#[derive(Debug, Default, Clone)]
pub struct SiteBase {
    pub provided_mana: u8,
    pub provided_thresholds: Thresholds,
    pub types: Vec<SiteType>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum Modifier {
    Disabled,
    Voidwalk,
    Airborne,
    Ranged,
    Stealth,
    Lethal,
    Movement(u8),
    Burrowing,
    Landbound,
    Submerge,
    Spellcaster(Element),
    Charge,
    SummoningSickness,
    TakesNoDamageFromElement(Element),
    Blaze(u8), // Specific modifier for the Blaze magic
}

impl Modifier {
    fn on_move(&self, card_id: &uuid::Uuid, state: &State, from: &Zone, _to: &Zone) -> Vec<Effect> {
        match self {
            Modifier::Blaze(burn) => {
                let mut effects = vec![];
                let units = state.get_units_in_zone(from);
                for unit in units {
                    let card = state.get_card(card_id).unwrap();
                    effects.push(Effect::take_damage(unit.get_id(), card.get_id(), *burn));
                }

                effects
            }
            _ => vec![],
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct UnitBase {
    pub power: u8,
    pub toughness: u8,
    pub modifiers: Vec<Modifier>,
    pub damage: u8,
    pub power_counters: Vec<Counter>,
    pub modifier_counters: Vec<ModifierCounter>,
    pub types: Vec<MinionType>,
}

#[derive(Debug, Clone)]
pub enum Rarity {
    Ordinary,
    Exceptional,
    Elite,
    Unique,
}

#[derive(Debug)]
pub struct CardBase {
    pub id: uuid::Uuid,
    pub owner_id: PlayerId,
    pub tapped: bool,
    pub zone: Zone,
    pub mana_cost: u8,
    pub required_thresholds: Thresholds,
    pub plane: Plane,
    pub rarity: Rarity,
    pub controller_id: PlayerId,
}

impl Clone for CardBase {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            owner_id: self.owner_id.clone(),
            tapped: self.tapped,
            zone: self.zone.clone(),
            mana_cost: self.mana_cost,
            required_thresholds: self.required_thresholds.clone(),
            plane: self.plane.clone(),
            rarity: self.rarity.clone(),
            controller_id: self.controller_id.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuraBase {}

#[derive(Debug, Clone)]
pub struct AvatarBase {}

pub fn from_name(name: &str, player_id: PlayerId) -> Box<dyn Card> {
    if let Some(card) = beta::from_beta_name(name, player_id) {
        return card;
    }

    panic!("Card with name '{}' not found", name);
}

pub fn from_name_and_zone(name: &str, player_id: PlayerId, zone: Zone) -> Box<dyn Card> {
    let mut card = from_name(name, player_id);
    card.set_zone(zone);
    card
}
