use crate::{
    card::{Card, from_name},
    deck::Deck,
    game::PlayerId,
};

pub fn earth(player_id: &PlayerId) -> (Deck, Vec<Box<dyn Card>>) {
    let spells = vec![
        (2, "Wild Boars"),
        (2, "Land Surveyor"),
        (2, "Scent Hounds"),
        (2, "Autumn Unicorn"),
        (3, "Belmotte Longbowmen"),
        (3, "Cave Trolls"),
        (1, "Slumbering Giantess"),
        (1, "Dalcean Phalanx"),
        (2, "House Arn Bannerman"),
        (1, "Pudge Butcher"),
        (2, "Amazon Warriors"),
        (1, "King of the Realm"),
        (1, "Wraetannis Titan"),
        (1, "Mountain Giant"),
        (1, "Divine Healing"),
        (2, "Overpower"),
        (1, "Border Militia"),
        (2, "Bury"),
        (1, "Cave-In"),
        (1, "Craterize"),
        (1, "Entangle Terrain"),
        (1, "Siege Ballista"),
        (1, "Rolling Boulder"),
        (1, "Payload Trebuchet"),
    ];
    let spells = spells
        .into_iter()
        .flat_map(|(count, name)| (0..count).map(move |_| from_name(name, player_id)))
        .collect::<Vec<_>>();

    let sites = vec![
        (1, "Bedrock"),
        (1, "Holy Ground"),
        (3, "Humble Village"),
        (2, "Quagmire"),
        (3, "Rustic Village"),
        (3, "Simple Village"),
        (1, "Sinkhole"),
        (2, "Vantage Hills"),
    ];
    let sites = sites
        .into_iter()
        .flat_map(|(count, name)| (0..count).map(move |_| from_name(name, player_id)))
        .collect::<Vec<_>>();

    let avatar = from_name("Geomancer", player_id);

    let mut deck = Deck {
        player_id: player_id.clone(),
        sites: sites.iter().map(|c| c.get_id().clone()).collect(),
        spells: spells.iter().map(|c| c.get_id().clone()).collect(),
        avatar: avatar.get_id().clone(),
    };
    deck.shuffle();

    (deck, vec![avatar].into_iter().chain(spells).chain(sites).collect())
}

pub fn fire(player_id: &PlayerId) -> (Deck, Vec<Box<dyn Card>>) {
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
        player_id: player_id.clone(),
        sites: sites.iter().map(|c| c.get_id().clone()).collect(),
        spells: spells.iter().map(|c| c.get_id().clone()).collect(),
        avatar: avatar.get_id().clone(),
    };
    deck.shuffle();

    (deck, vec![avatar].into_iter().chain(spells).chain(sites).collect())
}

pub fn air(player_id: &PlayerId) -> (Deck, Vec<Box<dyn Card>>) {
    let spells = vec![
        (1, "Sling Pixies"),
        (2, "Snow Leopard"),
        (2, "Cloud Spirit"),
        (2, "Dead of Night Demon"),
        (2, "Spectral Stalker"),
        (2, "Apprentice Wizard"),
        (2, "Headless Haunt"),
        (1, "Kite Archer"),
        (2, "Midnight Rogue"),
        (2, "Plumed Pegasus"),
        (1, "Spire Lich"),
        (1, "Gyre Hippogriffs"),
        (1, "Skirmishers of Mu"),
        (1, "Roaming Monster"),
        (1, "Grandmaster Wizard"),
        (1, "Nimbus Jinn"),
        (1, "Highland Clansmen"),
        (2, "Blink"),
        (2, "Chain Lightning"),
        (3, "Lightning Bolt"),
        (1, "Teleport"),
        (1, "Raise Dead"),
    ];
    let spells = spells
        .into_iter()
        .flat_map(|(count, name)| (0..count).map(move |_| from_name(name, player_id)))
        .collect::<Vec<_>>();

    let sites = vec![
        (1, "Cloud City"),
        (3, "Dark Tower"),
        (3, "Gothic Tower"),
        (3, "Lone Tower"),
        (2, "Mountain Pass"),
        (1, "Observatory"),
        (1, "Planar Gate"),
        (2, "Updraft Ridge"),
    ];
    let sites = sites
        .into_iter()
        .flat_map(|(count, name)| (0..count).map(move |_| from_name(name, player_id)))
        .collect::<Vec<_>>();

    let avatar = from_name("Sparkmage", player_id);

    let mut deck = Deck {
        player_id: player_id.clone(),
        sites: sites.iter().map(|c| c.get_id().clone()).collect(),
        spells: spells.iter().map(|c| c.get_id().clone()).collect(),
        avatar: avatar.get_id().clone(),
    };
    deck.shuffle();

    (deck, vec![avatar].into_iter().chain(spells).chain(sites).collect())
}
