use crate::{
    card::Region,
    state::State,
    zone::Location,
};

#[tokio::test]
async fn test_get_nearby_locations() {
    let state = State::new_mock_state(vec![3, 8, 9, 7, 6]);

    let loc = Location::Square(6, Region::Surface);
    let mut nearby = loc.get_nearby_locations(&state);
    nearby.sort();
    assert_eq!(
        vec![
            Location::Square(6, Region::Surface),
            Location::Square(7, Region::Surface),
        ],
        nearby
    );

    let loc = Location::Square(2, Region::Void);
    let mut nearby = loc.get_nearby_locations(&state);
    nearby.sort();
    assert_eq!(
        vec![
            Location::Square(1, Region::Void),
            Location::Square(2, Region::Void),
        ],
        nearby
    );

    let loc = Location::Square(2, Region::Void);
    let mut nearby = loc.get_nearby_sites(&state);
    nearby.sort();
    assert_eq!(
        vec![
            Location::Square(3, Region::Surface),
            Location::Square(6, Region::Surface),
            Location::Square(7, Region::Surface),
            Location::Square(8, Region::Surface),
        ],
        nearby
    );

    let loc = Location::Square(2, Region::Void);
    let mut nearby = loc.get_adjacent_sites(&state);
    nearby.sort();
    assert_eq!(
        vec![
            Location::Square(3, Region::Surface),
            Location::Square(7, Region::Surface),
        ],
        nearby
    );
}
