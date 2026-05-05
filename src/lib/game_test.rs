use crate::card::Zone;

#[test]
fn test_are_adjacent() {
    use crate::game::are_adjacent;

    assert!(are_adjacent(&Zone::Realm(1), &Zone::Realm(2)));
    assert!(are_adjacent(&Zone::Realm(3), &Zone::Realm(2)));
    assert!(are_adjacent(&Zone::Realm(3), &Zone::Realm(4)));
    assert!(!are_adjacent(&Zone::Realm(3), &Zone::Realm(7)));
    assert!(!are_adjacent(&Zone::Realm(3), &Zone::Realm(9)));
}

#[test]
fn test_are_nearby() {
    use crate::game::are_nearby;

    assert!(are_nearby(&Zone::Realm(1), &Zone::Realm(2)));
    assert!(are_nearby(&Zone::Realm(3), &Zone::Realm(2)));
    assert!(are_nearby(&Zone::Realm(3), &Zone::Realm(4)));
    assert!(are_nearby(&Zone::Realm(3), &Zone::Realm(7)));
    assert!(are_nearby(&Zone::Realm(3), &Zone::Realm(9)));
}

#[test]
fn test_get_adjacent_squares() {
    use crate::game::get_adjacent_zones;

    let adj = get_adjacent_zones(&Zone::Realm(8));
    assert!(adj.contains(&Zone::Realm(3)));
    assert!(adj.contains(&Zone::Realm(7)));
    assert!(adj.contains(&Zone::Realm(9)));
    assert!(adj.contains(&Zone::Realm(13)));
}
