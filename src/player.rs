use macroquad::math::Rect;

use crate::card::{Card, CardType};
use crate::deck::Deck;
use crate::window::{CARD_OFFSET_X, HAND_RECT};

pub struct Player {
    pub id: uuid::Uuid,
    pub name: String,
    pub deck: Deck,
    pub cards_in_hand: Vec<Card>,
}

impl Player {
    pub fn new(name: String, deck: Deck) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            name,
            deck,
            cards_in_hand: Vec::new(),
        }
    }

    pub fn draw_site(&mut self) {
        if let Some(mut site) = self.deck.draw_site() {
            site.zone = crate::card::CardZone::Hand;
            site.rect = Some(Rect::new(
                HAND_RECT.x + (self.cards_in_hand.len() as f32 * CARD_OFFSET_X),
                HAND_RECT.y,
                CardType::Spell.get_dimensions().x,
                CardType::Spell.get_dimensions().y,
            ));
            self.cards_in_hand.push(Card::Site(site));
        }
    }

    pub fn draw_spell(&mut self) {
        if let Some(mut spell) = self.deck.draw_spell() {
            spell.zone = crate::card::CardZone::Hand;
            spell.rect = Some(Rect::new(
                HAND_RECT.x + (self.cards_in_hand.len() as f32 * CARD_OFFSET_X),
                HAND_RECT.y,
                CardType::Spell.get_dimensions().x,
                CardType::Spell.get_dimensions().y,
            ));
            self.cards_in_hand.push(Card::Spell(spell));
        }
    }
}
