use crate::card::Region;
use crate::zone::Location;

#[test]
fn test_locations_are_adjacent() {
    assert!(
        Location::Square(1, Region::Surface).is_adjacent(&Location::Square(2, Region::Surface))
    );
    assert!(
        Location::Square(3, Region::Surface).is_adjacent(&Location::Square(2, Region::Surface))
    );
    assert!(
        Location::Square(3, Region::Surface).is_adjacent(&Location::Square(4, Region::Surface))
    );
    assert!(
        !Location::Square(3, Region::Surface).is_adjacent(&Location::Square(7, Region::Surface))
    );
    assert!(
        !Location::Square(3, Region::Surface).is_adjacent(&Location::Square(9, Region::Surface))
    );
}

#[test]
fn test_locations_are_nearby() {
    assert!(
        Location::Square(1, Region::Surface).is_nearby(&Location::Square(2, Region::Surface))
    );
    assert!(
        Location::Square(3, Region::Surface).is_nearby(&Location::Square(2, Region::Surface))
    );
    assert!(
        Location::Square(3, Region::Surface).is_nearby(&Location::Square(4, Region::Surface))
    );
    assert!(
        Location::Square(3, Region::Surface).is_nearby(&Location::Square(7, Region::Surface))
    );
    assert!(
        Location::Square(3, Region::Surface).is_nearby(&Location::Square(9, Region::Surface))
    );
}

#[test]
fn test_get_adjacent_squares() {
    let adj = Location::Square(8, Region::Surface).get_adjacent();
    assert!(adj.contains(&Location::Square(3, Region::Surface)));
    assert!(adj.contains(&Location::Square(7, Region::Surface)));
    assert!(adj.contains(&Location::Square(9, Region::Surface)));
    assert!(adj.contains(&Location::Square(13, Region::Surface)));
}
