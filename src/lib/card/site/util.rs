#[macro_export]
macro_rules! sites {
    ($($name:ident, $card_name:expr, $provided_mana:literal, $provided_threshold:literal, $edition:expr),+) => {
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        pub enum Site {
            $($name(CardBase),)+
        }

        pub const ALL_SITES: &[&str] = &[
            $($card_name,)+
        ];

        impl Site {
            pub fn from_name(name: &str, owner_id: uuid::Uuid) -> Option<Self> {
                match name {
                    $($card_name => Some(Site::$name(CardBase::new(owner_id, CardZone::Atlasbook))),)+
                    _ => None,
                }
            }

            pub fn get_provided_mana(&self) -> u32 {
                match self {
                    $(Site::$name(_) => $provided_mana,)+
                }
            }

            pub fn get_provided_threshold(&self) -> Thresholds {
                match self {
                    $(Site::$name(_) => Thresholds::parse($provided_threshold),)+
                }
            }

            pub fn get_edition(&self) -> Edition {
                match self {
                    $(Site::$name(_) => $edition,)+
                }
            }

            pub fn get_name(&self) -> &'static str {
                match self {
                    $(Site::$name(_) => $card_name,)+
                }
            }

            /// Returns a reference to the owner's unique identifier (`Uuid`).
            pub fn get_owner_id(&self) -> &uuid::Uuid {
                match self {
                    $(Site::$name(cb) => &cb.owner_id,)+
                }
            }

            /// Returns a reference to the current zone of the spell card.
            pub fn get_zone(&self) -> &CardZone {
                match self {
                    $(Site::$name(cb) => &cb.zone,)+
                }
            }

            pub fn set_zone(&mut self, zone: CardZone) {
                match self {
                    $(Site::$name(cb) => cb.zone = zone,)+
                }
            }


            /// Returns a reference to the underlying `CardBase` of the spell.
            pub fn get_base(&self) -> &CardBase {
                match self {
                    $(Site::$name(cb) => &cb,)+
                }
            }

            /// Returns a mutable reference to the underlying `CardBase` of the spell.
            pub fn get_base_mut(&mut self) -> &mut CardBase {
                match self {
                    $(Site::$name(cb) => cb,)+
                }
            }

            /// Returns a reference to the unique identifier (`Uuid`) of the spell card.
            pub fn get_id(&self) -> &uuid::Uuid {
                match self {
                    $(Site::$name(cb) => &cb.id,)+
                }
            }
        }
    };
}
