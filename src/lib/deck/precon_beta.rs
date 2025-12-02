use crate::{
    card::{avatar::Avatar, site::Site, spell::Spell},
    deck::Deck,
};

pub fn fire(player_id: uuid::Uuid) -> Deck {
    let avatar = "Flamecaller";
    let spells = [
        (1, "Wildfire"),
        (2, "Pit Vipers"),
        (2, "Raal Dromedary"),
        (1, "Lava Salamander"),
        (2, "Rimland Nomads"),
        (2, "Sacred Scarabs"),
        (2, "Wayfaring Pilgrim"),
        (1, "Colicky Dragonettes"),
        (3, "Ogre Goons"),
        (1, "Quarrelsome Kobolds"),
        (1, "Clamor of Harpies"),
        (1, "Hillock Basilisk"),
        (1, "Petrosian Cavalry"),
        (2, "Sand Worm"),
        (1, "Askelon Phoenix"),
        (1, "Escyllion Cyclops"),
        (1, "Infernal Legion"),
        (2, "Firebolts"),
        (1, "Mad Dash"),
        (1, "Blaze"),
        (1, "Heat Ray"),
        (2, "Minor Explosion"),
        (1, "Fireball"),
        (1, "Incinerate"),
        (1, "Cone of Flame"),
        (1, "Major Explosion"),
    ];
    let sites = [
        (4, "Arid Desert"),
        (1, "Cornerstone"),
        (4, "Red Desert"),
        (4, "Remote Desert"),
        (2, "Shifting Sands"),
        (1, "Vesuvius"),
    ];

    let mut deck = Deck::empty(player_id);
    deck.avatar = Avatar::from_name(avatar, player_id).unwrap();

    let spells = spells
        .iter()
        .flat_map(|(count, name)| {
            vec![Spell::from_name(name, player_id).expect(format!("{} to exist", name).as_str()); *count]
        })
        .collect();
    deck.spells = spells;

    let sites = sites
        .iter()
        .flat_map(|(count, name)| {
            vec![Site::from_name(name, player_id).expect(format!("{} to exist", name).as_str()); *count]
        })
        .collect();
    deck.sites = sites;

    deck
}
