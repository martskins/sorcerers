use crate::{card::Region, state::State, zone::Zone};

#[tokio::test]
async fn test_get_nearby_locations() {
    let state = State::new_mock_state(vec![3, 8, 9, 7, 6]);

    let loc = Zone::Realm(6, Region::Surface);
    let mut nearby = loc.get_nearby_locations(&state);
    nearby.sort();
    assert_eq!(
        vec![
            Zone::Realm(6, Region::Surface),
            Zone::Realm(7, Region::Surface),
        ],
        nearby
    );

    let loc = Zone::Realm(2, Region::Void);
    let mut nearby = loc.get_nearby_locations(&state);
    nearby.sort();
    assert_eq!(
        vec![Zone::Realm(1, Region::Void), Zone::Realm(2, Region::Void),],
        nearby
    );

    let loc = Zone::Realm(2, Region::Void);
    let mut nearby = loc.get_nearby_sites(&state);
    nearby.sort();
    assert_eq!(
        vec![
            Zone::Realm(3, Region::Surface),
            Zone::Realm(6, Region::Surface),
            Zone::Realm(7, Region::Surface),
            Zone::Realm(8, Region::Surface),
        ],
        nearby
    );

    let loc = Zone::Realm(2, Region::Void);
    let mut nearby = loc.get_adjacent_sites(&state);
    nearby.sort();
    assert_eq!(
        vec![
            Zone::Realm(3, Region::Surface),
            Zone::Realm(7, Region::Surface),
        ],
        nearby
    );
}
