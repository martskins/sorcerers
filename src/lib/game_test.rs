use crate::card::Region;
use crate::zone::Zone;

#[test]
fn test_are_adjacent() {
    use crate::game::are_adjacent;

    assert!(are_adjacent(
        &Zone::Location(1, Region::Surface),
        &Zone::Location(2, Region::Surface)
    ));
    assert!(are_adjacent(
        &Zone::Location(3, Region::Surface),
        &Zone::Location(2, Region::Surface)
    ));
    assert!(are_adjacent(
        &Zone::Location(3, Region::Surface),
        &Zone::Location(4, Region::Surface)
    ));
    assert!(!are_adjacent(
        &Zone::Location(3, Region::Surface),
        &Zone::Location(7, Region::Surface)
    ));
    assert!(!are_adjacent(
        &Zone::Location(3, Region::Surface),
        &Zone::Location(9, Region::Surface)
    ));
}

#[test]
fn test_are_nearby() {
    use crate::game::are_nearby;

    assert!(are_nearby(
        &Zone::Location(1, Region::Surface),
        &Zone::Location(2, Region::Surface)
    ));
    assert!(are_nearby(
        &Zone::Location(3, Region::Surface),
        &Zone::Location(2, Region::Surface)
    ));
    assert!(are_nearby(
        &Zone::Location(3, Region::Surface),
        &Zone::Location(4, Region::Surface)
    ));
    assert!(are_nearby(
        &Zone::Location(3, Region::Surface),
        &Zone::Location(7, Region::Surface)
    ));
    assert!(are_nearby(
        &Zone::Location(3, Region::Surface),
        &Zone::Location(9, Region::Surface)
    ));
}

#[test]
fn test_get_adjacent_squares() {
    use crate::game::get_adjacent_zones;

    let adj = get_adjacent_zones(&Zone::Location(8, Region::Surface));
    assert!(adj.contains(&Zone::Location(3, Region::Surface)));
    assert!(adj.contains(&Zone::Location(7, Region::Surface)));
    assert!(adj.contains(&Zone::Location(9, Region::Surface)));
    assert!(adj.contains(&Zone::Location(13, Region::Surface)));
}
