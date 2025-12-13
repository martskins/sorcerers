use crate::{
    card::{from_name, Card},
    deck::Deck,
    game::PlayerId,
};

pub fn fire(player_id: PlayerId) -> (Deck, Vec<Box<dyn Card>>) {
    let spells = vec![
        from_name("Clamor of Harpies", player_id),
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
    ];
    let sites = vec![
        from_name("Arid Desert", player_id),
        from_name("Arid Desert", player_id),
        from_name("Arid Desert", player_id),
        from_name("Arid Desert", player_id),
        from_name("Arid Desert", player_id),
    ];
    let avatar = from_name("Flamecaller", player_id);

    let mut deck = Deck {
        sites: sites.iter().map(|c| c.get_id().clone()).collect(),
        spells: spells.iter().map(|c| c.get_id().clone()).collect(),
        avatar: avatar.get_id().clone(),
    };
    deck.shuffle();

    (deck, vec![avatar].into_iter().chain(spells).chain(sites).collect())
}
