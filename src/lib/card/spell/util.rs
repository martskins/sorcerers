#[macro_export]
macro_rules! spells {
    ($($name:ident, $card_name:expr, $mana_cost:literal, $threshold:literal, $spell_type:expr, $power:expr, $toughness:expr, $abilities:expr, $edition:expr),+) => {
        /// Represents the different spell cards in the game.
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        pub enum Spell {
            $($name(SpellBase),)+
        }

        pub const ALL_SPELLS: &[&str] = &[
            $($card_name,)+
        ];

        impl Spell {
            pub fn get_abilities(&self) -> Vec<Ability> {
                match self {
                    $(Spell::$name(_) => $abilities.clone(),)+
                }
            }
            pub fn from_name(name: &str, owner_id: uuid::Uuid) -> Option<Self> {
                match name {
                    $($card_name => Some(Spell::$name(SpellBase::new(owner_id, CardZone::Spellbook))),)+
                    _ => None,
                }
            }
            pub fn get_edition(&self) -> Edition {
                match self {
                    $(Spell::$name(_) => $edition,)+
                }
            }

            pub fn get_name(&self) -> &'static str {
                match self {
                    $(Spell::$name(_) => $card_name,)+
                }
            }

            pub fn get_mana_cost(&self) -> u8 {
                match self {
                    $(Spell::$name(_) => $mana_cost,)+
                }
            }

            pub fn get_power(&self) -> Option<u8> {
                match self {
                    $(Spell::$name(_) => $power,)+
                }
            }

            pub fn get_toughness(&self) -> Option<u8> {
                match self {
                    $(Spell::$name(_) => $toughness,)+
                }
            }

            /// Returns a reference to the owner's unique identifier (`Uuid`).
            pub fn get_owner_id(&self) -> &uuid::Uuid {
                match self {
                    $(Spell::$name(cb) => &cb.card_base.owner_id,)+
                }
            }

            /// Returns a reference to the current zone of the spell card.
            pub fn get_zone(&self) -> &CardZone {
                match self {
                    $(Spell::$name(cb) => &cb.card_base.zone,)+
                }
            }

            pub fn set_zone(&mut self, zone: CardZone) {
                match self {
                    $(Spell::$name(cb) => cb.card_base.zone = zone,)+
                }
            }


            /// Returns a reference to the underlying `CardBase` of the spell.
            pub fn get_spell_base_mut(&mut self) -> &mut SpellBase {
                match self {
                    $(Spell::$name(cb) => cb,)+
                }
            }

            /// Returns a reference to the underlying `CardBase` of the spell.
            pub fn get_spell_base(&self) -> &SpellBase {
                match self {
                    $(Spell::$name(cb) => cb,)+
                }
            }

            /// Returns a reference to the underlying `CardBase` of the spell.
            pub fn get_base(&self) -> &CardBase {
                match self {
                    $(Spell::$name(cb) => &cb.card_base,)+
                }
            }

            /// Returns a mutable reference to the underlying `CardBase` of the spell.
            pub fn get_base_mut(&mut self) -> &mut CardBase {
                match self {
                    $(Spell::$name(cb) => &mut cb.card_base,)+
                }
            }

            /// Returns a reference to the unique identifier (`Uuid`) of the spell card.
            pub fn get_id(&self) -> &uuid::Uuid {
                match self {
                    $(Spell::$name(cb) => &cb.card_base.id,)+
                }
            }

            pub fn reset_damage(&mut self) {
                match self {
                    $(Spell::$name(cb) => cb.damage_taken = 0,)+
                }
            }

            /// Returns the required thresholds to play the spell.
            pub fn get_required_threshold(&self) -> Thresholds {
                match self {
                    $(Spell::$name(_) => Thresholds::parse($threshold),)+
                }
            }

            pub fn get_spell_type(&self) -> SpellType {
                match self {
                    $(Spell::$name(_) => $spell_type,)+
                }
            }
        }
    };
}
