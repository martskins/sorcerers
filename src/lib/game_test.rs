use crate::card::Region;
use crate::zone::{Location, Zone};

#[test]
fn test_are_adjacent() {
    use crate::game::are_adjacent;

    assert!(are_adjacent(
        &Zone::Location(Location::Square(1, Region::Surface)),
        &Zone::Location(Location::Square(2, Region::Surface))
    ));
    assert!(are_adjacent(
        &Zone::Location(Location::Square(3, Region::Surface)),
        &Zone::Location(Location::Square(2, Region::Surface))
    ));
    assert!(are_adjacent(
        &Zone::Location(Location::Square(3, Region::Surface)),
        &Zone::Location(Location::Square(4, Region::Surface))
    ));
    assert!(!are_adjacent(
        &Zone::Location(Location::Square(3, Region::Surface)),
        &Zone::Location(Location::Square(7, Region::Surface))
    ));
    assert!(!are_adjacent(
        &Zone::Location(Location::Square(3, Region::Surface)),
        &Zone::Location(Location::Square(9, Region::Surface))
    ));
}

#[test]
fn test_are_nearby() {
    use crate::game::are_nearby;

    assert!(are_nearby(
        &Zone::Location(Location::Square(1, Region::Surface)),
        &Zone::Location(Location::Square(2, Region::Surface))
    ));
    assert!(are_nearby(
        &Zone::Location(Location::Square(3, Region::Surface)),
        &Zone::Location(Location::Square(2, Region::Surface))
    ));
    assert!(are_nearby(
        &Zone::Location(Location::Square(3, Region::Surface)),
        &Zone::Location(Location::Square(4, Region::Surface))
    ));
    assert!(are_nearby(
        &Zone::Location(Location::Square(3, Region::Surface)),
        &Zone::Location(Location::Square(7, Region::Surface))
    ));
    assert!(are_nearby(
        &Zone::Location(Location::Square(3, Region::Surface)),
        &Zone::Location(Location::Square(9, Region::Surface))
    ));
}

#[test]
fn test_get_adjacent_squares() {
    use crate::game::get_adjacent_zones;

    let adj = get_adjacent_zones(&Zone::Location(Location::Square(8, Region::Surface)));
    assert!(adj.contains(&Zone::Location(Location::Square(3, Region::Surface))));
    assert!(adj.contains(&Zone::Location(Location::Square(7, Region::Surface))));
    assert!(adj.contains(&Zone::Location(Location::Square(9, Region::Surface))));
    assert!(adj.contains(&Zone::Location(Location::Square(13, Region::Surface))));
}
