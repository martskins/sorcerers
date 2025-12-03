#[macro_export]
macro_rules! spells {
    ($($variant:ident),+) => {
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        pub enum Spell {
            $($variant($variant),)+
        }

        pub const ALL_SPELLS: &[&str] = &[$($variant::NAME),+];

        impl Spell {
            pub fn get_abilities(&self) -> Vec<Ability> {
                match self {
                    $(Spell::$variant(cb) => cb.get_abilities(),)+
                }
            }

            pub fn on_damage_taken(&self, from: &uuid::Uuid, amount: u8, state: &State) -> Vec<Effect> {
                match self {
                    $(Spell::$variant(cb) => cb.on_damage_taken(from, amount, state),)+
                }
            }

            pub fn get_damage_taken(&self) -> u8 {
                self.get_spell_base().damage_taken
            }

            pub fn genesis(&self, state: &State) -> Vec<Effect> {
                match self {
                    $(Spell::$variant(cb) => cb.genesis(state),)+
                }
            }

            pub fn from_name(name: &str, owner_id: uuid::Uuid) -> Option<Self> {
                match name {
                    $($variant::NAME => Some(Spell::$variant($variant::new(owner_id, CardZone::Spellbook))),)+
                    _ => None,
                }
            }

            pub fn get_edition(&self) -> Edition {
                self.get_base().edition.clone()
            }

            pub fn get_name(&self) -> &str {
                match self {
                    $(Spell::$variant(c) => c.get_name(),)+
                }
            }

            pub fn get_mana_cost(&self) -> u8 {
                self.get_spell_base().mana_cost
            }

            pub fn get_power(&self) -> Option<u8> {
                self.get_spell_base().power
            }

            pub fn get_toughness(&self) -> Option<u8> {
                self.get_spell_base().toughness
            }

            /// Returns a reference to the owner's unique identifier (`Uuid`).
            pub fn get_owner_id(&self) -> &uuid::Uuid {
                match self {
                    $(Spell::$variant(c) => c.get_owner_id(),)+
                }
            }

            /// Returns a reference to the current zone of the spell card.
            pub fn get_zone(&self) -> &CardZone {
                &self.get_base().zone
            }

            pub fn set_zone(&mut self, zone: CardZone) {
                self.get_base_mut().zone = zone;
            }


            /// Returns a reference to the underlying `CardBase` of the spell.
            pub fn get_spell_base_mut(&mut self) -> &mut SpellBase {
                match self {
                    $(Spell::$variant(cb) => &mut cb.spell,)+
                }
            }

            pub fn deathrite(&self, state: &State) -> Vec<Effect> {
                match self {
                    $(Spell::$variant(cb) => cb.deathrite(state),)+
                }
            }

            /// Returns a reference to the underlying `CardBase` of the spell.
            pub fn get_spell_base(&self) -> &SpellBase {
                match self {
                    $(Spell::$variant(cb) => &cb.spell,)+
                }
            }

            /// Returns a reference to the underlying `CardBase` of the spell.
            pub fn get_base(&self) -> &CardBase {
                match self {
                    $(Spell::$variant(cb) => &cb.spell.card_base,)+
                }
            }

            /// Returns a mutable reference to the underlying `CardBase` of the spell.
            pub fn get_base_mut(&mut self) -> &mut CardBase {
                match self {
                    $(Spell::$variant(cb) => &mut cb.spell.card_base,)+
                }
            }

            /// Returns a reference to the unique identifier (`Uuid`) of the spell card.
            pub fn get_id(&self) -> &uuid::Uuid {
                match self {
                    $(Spell::$variant(c) => c.get_id(),)+
                }
            }

            pub fn reset_damage(&mut self) {
                match self {
                    $(Spell::$variant(cb) => cb.get_spell_base_mut().damage_taken = 0,)+
                }
            }

            /// Returns the required thresholds to play the spell.
            pub fn get_required_threshold(&self) -> Thresholds {
                self.get_spell_base().thresholds.clone()
            }

            pub fn get_spell_type(&self) -> &SpellType {
                match self {
                    $(Spell::$variant(c) => c.get_spell_type(),)+
                }
            }

            pub fn on_select_in_realm_actions(&self, state: &State) -> Vec<Action> {
                match self {
                    $(Spell::$variant(c) => c.on_select_in_realm_actions(state),)+
                }
            }
        }
    };
}
