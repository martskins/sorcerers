use convert_case::{Case, Casing};
use std::io::Read;
use std::io::Write;

const SITE_TEMPLATE: &'static str = r#"use crate::{
    card::{site::SiteBase, CardBase, CardZone, Edition},
    networking::Thresholds,
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
                    edition: Edition::X, // TODO: set edition
                },
                provided_mana: 1,
                provided_threshold: Thresholds::parse(), // TODO: set threshold
            },
        }
    }
}"#;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() != 3 {
        eprintln!("Usage: {} <name>", args[0]);
    }

    let ty = &args[1];
    match ty.as_str() {
        "site" => {
            {
                let name = args[2].clone();
                let content = SITE_TEMPLATE.replace("{Name}", &name.to_case(Case::Pascal));
                let path = format!("src/lib/card/site/{}.rs", name.to_lowercase().to_case(Case::Snake));
                let mut file = std::fs::File::create(path).unwrap();
                file.write_all(content.as_bytes()).unwrap();
            }

            let mut file = std::fs::File::open("src/lib/card/site/mod.rs").unwrap();
            let mut mod_content = String::new();
            file.read_to_string(&mut mod_content).unwrap();
            let name = args[2].clone();
            let mod_line = format!("pub mod {};\n", name.to_lowercase().to_case(Case::Snake));
            let use_line = format!(
                "use {}::{};\n",
                name.to_lowercase().to_case(Case::Snake),
                name.to_case(Case::Pascal)
            );
            if !mod_content.contains(&mod_line) {
                mod_content = format!("{}{}{}", mod_line, use_line, mod_content);
                let mut file = std::fs::File::create("src/lib/card/site/mod.rs").unwrap();
                file.write_all(mod_content.as_bytes()).unwrap();
            }
        }
        _ => {
            eprintln!("Unknown type: {}", ty);
        }
    }
}
