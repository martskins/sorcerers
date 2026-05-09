use crate::card::{Region, Zone};

#[test]
fn test_are_adjacent() {
    use crate::game::are_adjacent;

    assert!(are_adjacent(
        &Zone::Realm(1, Region::Surface),
        &Zone::Realm(2, Region::Surface)
    ));
    assert!(are_adjacent(
        &Zone::Realm(3, Region::Surface),
        &Zone::Realm(2, Region::Surface)
    ));
    assert!(are_adjacent(
        &Zone::Realm(3, Region::Surface),
        &Zone::Realm(4, Region::Surface)
    ));
    assert!(!are_adjacent(
        &Zone::Realm(3, Region::Surface),
        &Zone::Realm(7, Region::Surface)
    ));
    assert!(!are_adjacent(
        &Zone::Realm(3, Region::Surface),
        &Zone::Realm(9, Region::Surface)
    ));
}

#[test]
fn test_are_nearby() {
    use crate::game::are_nearby;

    assert!(are_nearby(
        &Zone::Realm(1, Region::Surface),
        &Zone::Realm(2, Region::Surface)
    ));
    assert!(are_nearby(
        &Zone::Realm(3, Region::Surface),
        &Zone::Realm(2, Region::Surface)
    ));
    assert!(are_nearby(
        &Zone::Realm(3, Region::Surface),
        &Zone::Realm(4, Region::Surface)
    ));
    assert!(are_nearby(
        &Zone::Realm(3, Region::Surface),
        &Zone::Realm(7, Region::Surface)
    ));
    assert!(are_nearby(
        &Zone::Realm(3, Region::Surface),
        &Zone::Realm(9, Region::Surface)
    ));
}

#[test]
fn test_get_adjacent_squares() {
    use crate::game::get_adjacent_zones;

    let adj = get_adjacent_zones(&Zone::Realm(8, Region::Surface));
    assert!(adj.contains(&Zone::Realm(3, Region::Surface)));
    assert!(adj.contains(&Zone::Realm(7, Region::Surface)));
    assert!(adj.contains(&Zone::Realm(9, Region::Surface)));
    assert!(adj.contains(&Zone::Realm(13, Region::Surface)));
}
