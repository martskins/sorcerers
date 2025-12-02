use clap::Parser;
use convert_case::{Case, Casing};
use sorcerers::card::Thresholds;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;

const SITE_TEMPLATE: &'static str = r#"use crate::card::{
    site::{site::SiteType, SiteBase},
    CardBase, CardZone, Edition, Thresholds,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct {Name} {
    pub base: SiteBase,
}

impl {Name} {
    pub fn new(owner_id: uuid::Uuid, zone: CardZone) -> Self {
        Self {
            base: SiteBase {
                card_base: CardBase {
                    id: uuid::Uuid::new_v4(),
                    owner_id,
                    zone,
                    tapped: false,
                    edition: Edition::{Edition},
                },
                provided_mana: 1,
                provided_threshold: Thresholds::parse(""),
                site_types: vec![],
            },
        }
    }
}"#;

const SPELL_TEMPLATE: &'static str = r#"use serde::{Deserialize, Serialize};
use crate::{
    card::{
        spell::{Ability, SpellBase, SpellType},
        CardBase, CardType, CardZone, Combat, Edition, Interaction, Lifecycle, Thresholds,
    },
    effect::{Action, Effect},
    game::State,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct {Name} {
    pub spell: SpellBase,
}

impl {Name} {
    pub fn new(owner_id: uuid::Uuid, zone: CardZone) -> Self {
        Self {
            spell: SpellBase {
                card_base: CardBase {
                    id: uuid::Uuid::new_v4(),
                    owner_id,
                    zone,
                    tapped: false,
                    edition: Edition::{Edition},
                },
                damage_taken: 0,
                mana_cost: {ManaCost},
                thresholds: Thresholds::parse("{Thresholds}"),
                power: {Power}
                toughness: {Toughness},
            },
        }
    }

    pub fn get_spell_type(&self) -> &SpellType {
        &SpellType::{SpellType}
    }

    pub fn get_edition(&self) -> &Edition {
        &Edition::{Edition}
    }

    pub fn get_type(&self) -> CardType {
        CardType::Spell
    }

    pub fn get_toughness(&self) -> Option<u8> {
        self.spell.toughness
    }

    pub fn get_power(&self) -> Option<u8> {
        self.spell.power
    }

    pub fn get_abilities(&self) -> Vec<Ability> {
        vec![]
    }

    pub fn get_spell_base(&self) -> &SpellBase {
        &self.spell
    }

    pub fn get_spell_base_mut(&mut self) -> &mut SpellBase {
        &mut self.spell
    }

    pub fn get_cell_id(&self) -> Option<u8> {
        match self.spell.card_base.zone {
            CardZone::Realm(cell_id) => Some(cell_id),
            _ => None,
        }
    }

    pub fn get_name(&self) -> &str {
        "{Name}"
    }

    pub fn get_owner_id(&self) -> &uuid::Uuid {
        &self.spell.card_base.owner_id
    }

    pub fn get_id(&self) -> &uuid::Uuid {
        &self.spell.card_base.id
    }
}

impl Lifecycle for {Name} {}
impl Combat for {Name} {}
impl Interaction for {Name} {}
"#;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    file_path: String,
}

fn create_file_with_content(path: &str, content: &str) -> anyhow::Result<()> {
    let path = std::path::Path::new(path);
    if path.exists() {
        return Ok(());
    }

    let mut file = std::fs::File::create(path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    Ok(())
}

fn main() {
    let args = Args::parse();
    // if args.name.is_some() {
    //     for name in args.name.unwrap() {
    //         let variant = name.to_case(Case::Pascal);
    //         let module = name.to_lowercase().to_case(Case::Snake);
    //         let edition_mod = args.edition.to_lowercase().to_case(Case::Snake);
    //         let edition_variant = args.edition.to_case(Case::Pascal);
    //
    //         match args.card_type.as_str() {
    //             "site" => {
    //                 let content = SITE_TEMPLATE
    //                     .replace("{Name}", &variant)
    //                     .replace("{Edition}", &edition_variant);
    //                 let path = format!("src/lib/card/{}/site/{}.rs", edition_mod, module);
    //                 create_file_with_content(&path, &content).unwrap();
    //
    //                 let mut mod_file =
    //                     std::fs::File::open(format!("src/lib/card/{}/site/mod.rs", edition_mod)).unwrap();
    //                 let mut mod_content = String::new();
    //                 mod_file.read_to_string(&mut mod_content).unwrap();
    //                 let mod_line = format!("pub mod {};\n", module);
    //                 let use_line = format!("pub use {}::{};\n", module, variant);
    //                 if !mod_content.contains(&mod_line) {
    //                     mod_content = format!("{}{}{}", mod_line, use_line, mod_content);
    //                     mod_file.write_all(mod_content.as_bytes()).unwrap();
    //                 }
    //             }
    //             "magic" => {
    //                 let content = SPELL_TEMPLATE
    //                     .replace("{Name}", &variant)
    //                     .replace("{Edition}", &edition_variant)
    //                     .replace("{SpellType}", "Magic");
    //                 let path = format!("src/lib/card/{}/spell/{}.rs", edition_mod, module);
    //                 create_file_with_content(&path, &content).unwrap();
    //
    //                 let mut mod_file = std::fs::File::open("src/lib/card/{}/spell/mod.rs").unwrap();
    //                 let mut mod_content = String::new();
    //                 mod_file.read_to_string(&mut mod_content).unwrap();
    //                 let mod_line = format!("pub mod {};\n", module);
    //                 let use_line = format!("pub use {}::{};\n", module, variant,);
    //                 if !mod_content.contains(&mod_line) {
    //                     mod_content = format!("{}{}{}", mod_line, use_line, mod_content);
    //                     mod_file.write_all(mod_content.as_bytes()).unwrap();
    //                 }
    //             }
    //             "aura" => {
    //                 let content = SPELL_TEMPLATE
    //                     .replace("{Name}", &variant)
    //                     .replace("{Edition}", &edition_variant)
    //                     .replace("{SpellType}", "Aura");
    //                 let path = format!("src/lib/card/{}/spell/{}.rs", edition_mod, module);
    //                 create_file_with_content(&path, &content).unwrap();
    //
    //                 let mut mod_file = std::fs::File::open("src/lib/card/{}/spell/mod.rs").unwrap();
    //                 let mut mod_content = String::new();
    //                 mod_file.read_to_string(&mut mod_content).unwrap();
    //                 let mod_line = format!("pub mod {};\n", module);
    //                 let use_line = format!("pub use {}::{};\n", module, variant,);
    //                 if !mod_content.contains(&mod_line) {
    //                     mod_content = format!("{}{}{}", mod_line, use_line, mod_content);
    //                     mod_file.write_all(mod_content.as_bytes()).unwrap();
    //                 }
    //             }
    //             "minion" => {}
    //             _ => {
    //                 eprintln!("Unknown type: {}", args.card_type);
    //             }
    //         }
    //     }
    // }

    let file = std::fs::File::open(args.file_path).unwrap();
    let reader = BufReader::new(file);
    for line in reader.lines().skip(1) {
        let line = line.unwrap();
        let parts = line.split(',').collect::<Vec<&str>>();
        create_minion(parts[1], parts[2], parts[3], parts[4], parts[5], parts[6]);
    }
}

fn create_minion(name: &str, mana_cost: &str, thresholds: &str, power: &str, toughness: &str, edition: &str) {
    let variant = name.to_case(Case::Pascal);
    let module = name.to_lowercase().to_case(Case::Snake);
    let edition_mod = edition.to_lowercase().to_case(Case::Snake);
    let edition_variant = edition.to_case(Case::Pascal);

    let content = SPELL_TEMPLATE
        .replace("{Name}", &variant)
        .replace("{Edition}", &edition_variant)
        .replace("{Power}", power)
        .replace("{Toughness}", toughness)
        .replace("{Thresholds}", thresholds)
        .replace("{ManaCost}", mana_cost)
        .replace("{SpellType}", "Minion");
    let path = format!("src/lib/card/{}/spell/{}.rs", edition_mod, module);
    create_file_with_content(&path, &content).unwrap();

    let mut mod_file = std::fs::File::open("src/lib/card/{}/spell/mod.rs").unwrap();
    let mut mod_content = String::new();
    mod_file.read_to_string(&mut mod_content).unwrap();
    let mod_line = format!("pub mod {};\n", module);
    let use_line = format!("pub use {}::{};\n", module, variant,);
    if !mod_content.contains(&mod_line) {
        mod_content = format!("{}{}{}", mod_line, use_line, mod_content);
        mod_file.write_all(mod_content.as_bytes()).unwrap();
    }
}
