#[macro_export]
macro_rules! avatars {
    ($($name:ident, $card_name:expr, $edition:expr),+) => {
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        pub enum Avatar {
            $($name(CardBase),)+
        }

        pub const ALL_AVATARS: &[&str] = &[
            $($card_name,)+
        ];

        impl Avatar {
            pub fn from_name(name: &str, owner_id: uuid::Uuid) -> Option<Self> {
                match name {
                    $($card_name => Some(Avatar::$name(CardBase::new(owner_id, CardZone::Spellbook))),)+
                    _ => None,
                }
            }

            pub fn get_edition(&self) -> Edition {
                match self {
                    $(Avatar::$name(_) => $edition,)+
                }
            }

            pub fn get_name(&self) -> &'static str {
                match self {
                    $(Avatar::$name(_) => $card_name,)+
                }
            }

            /// Returns a reference to the owner's unique identifier (`Uuid`).
            pub fn get_owner_id(&self) -> &uuid::Uuid {
                match self {
                    $(Avatar::$name(cb) => &cb.owner_id,)+
                }
            }

            /// Returns a reference to the current zone of the spell card.
            pub fn get_zone(&self) -> &CardZone {
                match self {
                    $(Avatar::$name(cb) => &cb.zone,)+
                }
            }

            pub fn set_zone(&mut self, zone: CardZone) {
                match self {
                    $(Avatar::$name(cb) => cb.zone = zone,)+
                }
            }


            /// Returns a reference to the underlying `CardBase` of the spell.
            pub fn get_base(&self) -> &CardBase {
                match self {
                    $(Avatar::$name(cb) => &cb,)+
                }
            }

            /// Returns a mutable reference to the underlying `CardBase` of the spell.
            pub fn get_base_mut(&mut self) -> &mut CardBase {
                match self {
                    $(Avatar::$name(cb) => cb,)+
                }
            }

            /// Returns a reference to the unique identifier (`Uuid`) of the spell card.
            pub fn get_id(&self) -> &uuid::Uuid {
                match self {
                    $(Avatar::$name(cb) => &cb.id,)+
                }
            }
        }
    };
}
