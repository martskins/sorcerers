use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct GreatWall {
    site_base: SiteBase,
    card_base: CardBase,
}

impl GreatWall {
    pub const NAME: &'static str = "Great Wall";
    pub const DESCRIPTION: &'static str =
        "Enemy units can’t move through this site’s top border on the ground.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::ZERO,
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }

    fn is_enemy_ground_unit_crossing_top_border(
        &self,
        card_id: &CardId,
        other_side: &Zone,
        region: &Region,
        state: &State,
    ) -> bool {
        let card = state.get_card(card_id);
        if !card.is_unit()
            || card.get_controller_id(state) == self.get_controller_id(state)
            || card.has_ability(state, &Ability::Airborne)
            || card.get_region(state) != &Region::Surface
            || region != &Region::Surface
        {
            return false;
        }

        self.get_zone()
            .zone_in_direction(&Direction::Up, 1)
            .is_some_and(|top_zone| &top_zone == other_side)
    }
}

#[async_trait::async_trait]
impl Site for GreatWall {
    fn can_be_entered_by(
        &self,
        card_id: &CardId,
        from: &Zone,
        region: &Region,
        state: &State,
    ) -> anyhow::Result<bool> {
        Ok(
            !self.is_enemy_ground_unit_crossing_top_border(card_id, from, region, state)
                && self.base_can_be_entered_by(card_id, from, region, state)?,
        )
    }

    fn can_be_exited_by(
        &self,
        card_id: &CardId,
        to: &Zone,
        region: &Region,
        state: &State,
    ) -> anyhow::Result<bool> {
        Ok(!self.is_enemy_ground_unit_crossing_top_border(card_id, to, region, state))
    }
}

impl ResourceProvider for GreatWall {}

#[async_trait::async_trait]
impl Card for GreatWall {
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

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (GreatWall::NAME, |owner_id: PlayerId| {
    Box::new(GreatWall::new(owner_id))
});
