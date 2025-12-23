use crate::{
    card::{Card, from_name},
    deck::Deck,
    game::PlayerId,
};

pub fn fire(player_id: PlayerId) -> (Deck, Vec<Box<dyn Card>>) {
    let spells = vec![
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
    let spells = spells
        .into_iter()
        .flat_map(|(count, name)| (0..count).map(move |_| from_name(name, player_id)))
        .collect::<Vec<_>>();

    let sites = vec![
        (4, "Arid Desert"),
        (1, "Cornerstone"),
        (4, "Red Desert"),
        (4, "Remote Desert"),
        (2, "Shifting Sands"),
        (1, "Vesuvius"),
    ];
    let sites = sites
        .into_iter()
        .flat_map(|(count, name)| (0..count).map(move |_| from_name(name, player_id)))
        .collect::<Vec<_>>();

    let avatar = from_name("Flamecaller", player_id);

    let mut deck = Deck {
        sites: sites.iter().map(|c| c.get_id().clone()).collect(),
        spells: spells.iter().map(|c| c.get_id().clone()).collect(),
        avatar: avatar.get_id().clone(),
    };
    deck.shuffle();

    (deck, vec![avatar].into_iter().chain(spells).chain(sites).collect())
}

pub fn air(player_id: PlayerId) -> (Deck, Vec<Box<dyn Card>>) {
    let spells = vec![
        from_name("Clamor of Harpies", player_id),
        from_name("Clamor of Harpies", player_id),
        from_name("Clamor of Harpies", player_id),
        from_name("Clamor of Harpies", player_id),
        from_name("Clamor of Harpies", player_id),
        from_name("Pit Vipers", player_id),
        from_name("Pit Vipers", player_id),
        from_name("Pit Vipers", player_id),
        from_name("Pit Vipers", player_id),
        from_name("Pit Vipers", player_id),
        from_name("Pit Vipers", player_id),
        from_name("Wayfaring Pilgrim", player_id),
        from_name("Wayfaring Pilgrim", player_id),
        from_name("Wayfaring Pilgrim", player_id),
        from_name("Wayfaring Pilgrim", player_id),
        from_name("Wayfaring Pilgrim", player_id),
        from_name("Sacred Scarabs", player_id),
        from_name("Sacred Scarabs", player_id),
        from_name("Sacred Scarabs", player_id),
        from_name("Sacred Scarabs", player_id),
        from_name("Sacred Scarabs", player_id),
        from_name("Lava Salamander", player_id),
        from_name("Lava Salamander", player_id),
        from_name("Lava Salamander", player_id),
        from_name("Lava Salamander", player_id),
        from_name("Lava Salamander", player_id),
        from_name("Raal Dromedary", player_id),
        from_name("Raal Dromedary", player_id),
        from_name("Raal Dromedary", player_id),
        from_name("Raal Dromedary", player_id),
        from_name("Raal Dromedary", player_id),
    ];
    let sites = vec![
        from_name("Arid Desert", player_id),
        from_name("Arid Desert", player_id),
        from_name("Arid Desert", player_id),
        from_name("Arid Desert", player_id),
        from_name("Arid Desert", player_id),
    ];
    let avatar = from_name("Sparkmage", player_id);

    let mut deck = Deck {
        sites: sites.iter().map(|c| c.get_id().clone()).collect(),
        spells: spells.iter().map(|c| c.get_id().clone()).collect(),
        avatar: avatar.get_id().clone(),
    };
    deck.shuffle();

    (deck, vec![avatar].into_iter().chain(spells).chain(sites).collect())
}
