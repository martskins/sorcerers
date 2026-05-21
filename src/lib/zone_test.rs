use crate::{card::Region, state::State, zone::Zone};

#[tokio::test]
async fn test_get_nearby_locations() {
    let state = State::new_mock_state(vec![3, 8, 9, 7, 6]);

    let loc = Zone::Location(6, Region::Surface);
    let mut nearby = loc.get_nearby_locations(&state);
    nearby.sort();
    assert_eq!(
        vec![
            Zone::Location(6, Region::Surface),
            Zone::Location(7, Region::Surface),
        ],
        nearby
    );

    let loc = Zone::Location(2, Region::Void);
    let mut nearby = loc.get_nearby_locations(&state);
    nearby.sort();
    assert_eq!(
        vec![
            Zone::Location(1, Region::Void),
            Zone::Location(2, Region::Void),
        ],
        nearby
    );

    let loc = Zone::Location(2, Region::Void);
    let mut nearby = loc.get_nearby_sites(&state);
    nearby.sort();
    assert_eq!(
        vec![
            Zone::Location(3, Region::Surface),
            Zone::Location(6, Region::Surface),
            Zone::Location(7, Region::Surface),
            Zone::Location(8, Region::Surface),
        ],
        nearby
    );

    let loc = Zone::Location(2, Region::Void);
    let mut nearby = loc.get_adjacent_sites(&state);
    nearby.sort();
    assert_eq!(
        vec![
            Zone::Location(3, Region::Surface),
            Zone::Location(7, Region::Surface),
        ],
        nearby
    );
}
