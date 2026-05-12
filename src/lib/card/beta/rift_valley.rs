use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct RiftValley {
    site_base: SiteBase,
    card_base: CardBase,
}

impl RiftValley {
    pub const NAME: &'static str = "Rift Valley";
    pub const DESCRIPTION: &'static str =
        "You may pull apart a partial row or column to make a void in which to play this.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("E"),
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }

    fn same_row(square: u8, other: u8) -> bool {
        (square - 1) / 5 == (other - 1) / 5
    }

    fn same_column(square: u8, other: u8) -> bool {
        (square - 1) % 5 == (other - 1) % 5
    }

    fn row_is_partial(square: u8, occupied_squares: &[u8]) -> bool {
        let row_start = ((square - 1) / 5) * 5 + 1;
        (row_start..=row_start + 4).any(|sq| !occupied_squares.contains(&sq))
    }

    fn column_is_partial(square: u8, occupied_squares: &[u8]) -> bool {
        let column = (square - 1) % 5;
        (0..4)
            .map(|row| row * 5 + column + 1)
            .any(|sq| !occupied_squares.contains(&sq))
    }
}

#[async_trait::async_trait]
impl Site for RiftValley {}

impl ResourceProvider for RiftValley {}

#[async_trait::async_trait]
impl Card for RiftValley {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_description(&self) -> &str {
        Self::DESCRIPTION
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn get_site_base(&self) -> Option<&SiteBase> {
        Some(&self.site_base)
    }

    fn get_site_base_mut(&mut self) -> Option<&mut SiteBase> {
        Some(&mut self.site_base)
    }

    fn get_valid_play_zones(
        &self,
        state: &State,
        player_id: &PlayerId,
        caster_id: &uuid::Uuid,
    ) -> anyhow::Result<Vec<Zone>> {
        let mut valid_zones = self.base_get_valid_play_zones(state, player_id, caster_id)?;
        let occupied_squares = CardQuery::new()
            .sites()
            .in_play()
            .all(state)
            .into_iter()
            .filter_map(|card_id| state.get_card(&card_id).get_zone().get_square())
            .collect::<Vec<_>>();
        let controlled_squares = CardQuery::new()
            .sites()
            .in_play()
            .controlled_by(player_id)
            .not_named(Rubble::NAME)
            .all(state)
            .into_iter()
            .filter_map(|card_id| state.get_card(&card_id).get_zone().get_square())
            .collect::<Vec<_>>();

        let rift_valley_zones = Zone::all_in_surface()
            .into_iter()
            .filter(|zone| zone.get_site(state).is_none())
            .filter_map(|zone| {
                let square = zone.get_square()?;
                let costs = state
                    .get_effective_costs(self.get_id(), Some(&zone), player_id)
                    .ok()?;
                if !costs.can_afford(state, player_id).unwrap_or_default() {
                    return None;
                }

                let can_pull_apart_row = Self::row_is_partial(square, &occupied_squares)
                    && controlled_squares
                        .iter()
                        .any(|controlled_square| Self::same_row(square, *controlled_square));
                let can_pull_apart_column = Self::column_is_partial(square, &occupied_squares)
                    && controlled_squares
                        .iter()
                        .any(|controlled_square| Self::same_column(square, *controlled_square));

                (can_pull_apart_row || can_pull_apart_column).then_some(zone)
            });

        valid_zones.extend(rift_valley_zones);
        valid_zones.sort();
        valid_zones.dedup();
        Ok(valid_zones)
    }

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (RiftValley::NAME, |owner_id: PlayerId| {
    Box::new(RiftValley::new(owner_id))
});
