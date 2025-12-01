#[macro_export]
macro_rules! sites {
    ($($variant:ident, $card_name:expr),+) => {
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        pub enum Site {
            $($variant($variant),)+
        }

        pub const ALL_SITES: &[&str] = &[
            $($card_name,)+
        ];

        impl Site {
            pub fn from_name(name: &str, owner_id: uuid::Uuid) -> Option<Self> {
                match name {
                    $($card_name => Some(Site::$variant($variant::new(owner_id, CardZone::Atlasbook))),)+
                    _ => None,
                }
            }

            pub fn get_provided_mana(&self) -> u8 {
                match self {
                    $(Site::$variant(c) => c.base.provided_mana,)+
                }
            }

            pub fn get_provided_threshold(&self) -> Thresholds {
                match self {
                    $(Site::$variant(c) => c.base.provided_threshold.clone(),)+
                }
            }

            pub fn get_edition(&self) -> Edition {
                match self {
                    $(Site::$variant(c) => c.base.card_base.edition.clone(),)+
                }
            }

            pub fn get_name(&self) -> &str {
                match self {
                    $(Site::$variant(_) => $card_name,)+
                }
            }

            /// Returns a reference to the owner's unique identifier (`Uuid`).
            pub fn get_owner_id(&self) -> &uuid::Uuid {
                match self {
                    $(Site::$variant(cb) => &cb.base.card_base.owner_id,)+
                }
            }

            /// Returns a reference to the current zone of the spell card.
            pub fn get_zone(&self) -> &CardZone {
                match self {
                    $(Site::$variant(cb) => &cb.base.card_base.zone,)+
                }
            }

            pub fn set_zone(&mut self, zone: CardZone) {
                match self {
                    $(Site::$variant(cb) => cb.base.card_base.zone = zone,)+
                }
            }


            /// Returns a reference to the underlying `CardBase` of the spell.
            pub fn get_base(&self) -> &CardBase {
                match self {
                    $(Site::$variant(cb) => &cb.base.card_base,)+
                }
            }

            /// Returns a mutable reference to the underlying `CardBase` of the spell.
            pub fn get_base_mut(&mut self) -> &mut CardBase {
                match self {
                    $(Site::$variant(cb) => &mut cb.base.card_base,)+
                }
            }

            /// Returns a reference to the unique identifier (`Uuid`) of the spell card.
            pub fn get_id(&self) -> &uuid::Uuid {
                match self {
                    $(Site::$variant(cb) => &cb.base.card_base.id,)+
                }
            }
        }
    };
}
