use crate::card::avatar::Avatar;
use crate::card::site::Site;
use crate::card::spell::Spell;

pub struct Deck {
    sites: Vec<Site>,
    spells: Vec<Spell>,
    avatar: Avatar,
}

impl Clone for Deck {
    fn clone(&self) -> Self {
        Self {
            sites: self.sites.clone(),
            spells: self.spells.clone(),
            avatar: self.avatar.clone(),
        }
    }
}

impl Deck {
    pub fn new(avatar: Avatar, sites: Vec<Site>, spells: Vec<Spell>) -> Self {
        Self {
            avatar,
            sites,
            spells,
        }
    }

    pub async fn test_deck() -> Self {
        let avatar = Avatar::from_name("Battlemage").await.unwrap();
        let sites = vec![Site::from_name("Spring River").await.unwrap(); 20];
        let spells = vec![Spell::from_name("Battlemage").await.unwrap(); 40];

        Self::new(avatar, sites, spells)
    }

    pub fn draw_site(&mut self) -> Option<Site> {
        self.sites.pop()
    }

    pub fn draw_spell(&mut self) -> Option<Spell> {
        self.spells.pop()
    }
}

impl From<&Deck> for Deck {
    fn from(deck: &Deck) -> Self {
        let mut sites = vec![];
        for it in &deck.sites {
            let mut site = it.clone();
            site.id = uuid::Uuid::new_v4();
            sites.push(site);
        }

        let mut spells = vec![];
        for it in &deck.spells {
            let mut spell = it.clone();
            spell.id = uuid::Uuid::new_v4();
            spells.push(spell);
        }

        let mut avatar = deck.avatar.clone();
        avatar.id = uuid::Uuid::new_v4();
        Deck::new(avatar, sites, spells)
    }
}
