mod artifact_template;
mod aura_template;
mod magic_template;
mod minion_template;
mod site_template;

use artifact_template::ARTIFACT_TEMPLATE;
use aura_template::AURA_TEMPLATE;
use clap::Parser;
use convert_case::{Case, Casing};
use magic_template::MAGIC_TEMPLATE;
use minion_template::MINION_TEMPLATE;
use site_template::SITE_TEMPLATE;
use std::io::Write;

#[derive(Parser, Clone, Debug, clap::ValueEnum)]
enum CardType {
    Minion,
    Magic,
    Site,
    Avatar,
    Aura,
    Artifact,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    path: String,

    #[arg(long)]
    card_type: CardType,
}

#[derive(Debug)]
struct ArtifactRecord {
    name: String,
    edition: String,
    mana_cost: u8,
    required_thresholds: String,
    rarity: String,
}

#[derive(Debug)]
struct AuraRecord {
    name: String,
    edition: String,
    mana_cost: u8,
    required_thresholds: String,
    rarity: String,
}

#[derive(Debug)]
struct MagicRecord {
    name: String,
    edition: String,
    mana_cost: u8,
    required_thresholds: String,
    rarity: String,
}

#[derive(Debug)]
struct SiteRecord {
    name: String,
    edition: String,
    provided_mana: u8,
    provided_thresholds: String,
    types: Vec<String>,
    rarity: String,
}

#[derive(Debug)]
struct MinionRecord {
    name: String,
    edition: String,
    mana_cost: u8,
    required_threshold: String,
    power: u8,
    toughness: u8,
    types: Vec<String>,
    rarity: String,
    modifiers: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let file = std::fs::File::open(&args.path)?;
    match args.card_type {
        CardType::Minion => {
            let mut rdr = csv::Reader::from_reader(file);
            for record in rdr.records() {
                let record = record?;
                let mut minion: MinionRecord = MinionRecord {
                    name: record[0].to_string(),
                    edition: record[1].to_string(),
                    mana_cost: record[2].parse().unwrap(),
                    required_threshold: record[3].to_string(),
                    power: record[4].parse().unwrap(),
                    toughness: record[5].parse().unwrap(),
                    types: vec![],
                    rarity: record[7].to_string(),
                    modifiers: vec![],
                };

                let modifiers_str = record[8].to_string();
                if !modifiers_str.is_empty() {
                    minion.modifiers = modifiers_str.split(",").map(|s| s.to_string()).collect();
                }

                let types_str = record[6].to_string();
                if !types_str.is_empty() {
                    minion.types = types_str.split(",").map(|s| s.to_string()).collect();
                }

                let edition = minion.edition.to_lowercase();
                let filename = minion.name.to_case(Case::Snake);
                let path = format!("src/lib/card/{}/{}.rs", edition, filename);
                if std::fs::exists(&path)? {
                    continue;
                }

                let struct_name = minion.name.to_case(Case::Pascal);
                let modifiers = minion
                    .modifiers
                    .iter()
                    .map(|m| format!("Modifier::{}", m))
                    .collect::<Vec<String>>()
                    .join(", ");
                let minion_types = minion
                    .types
                    .iter()
                    .map(|m| format!("MinionType::{}", m))
                    .collect::<Vec<String>>()
                    .join(", ");

                let contents = MINION_TEMPLATE
                    .replace("{CardName}", &minion.name)
                    .replace("{StructName}", &struct_name)
                    .replace("{Power}", &minion.power.to_string())
                    .replace("{Toughness}", &minion.toughness.to_string())
                    .replace("{Modifiers}", &modifiers)
                    .replace("{MinionTypes}", &minion_types)
                    .replace("{ManaCost}", &minion.mana_cost.to_string())
                    .replace("{RequiredThresholds}", &minion.required_threshold)
                    .replace("{Rarity}", &minion.rarity)
                    .replace("{Edition}", &minion.edition);

                let mut file = std::fs::File::create(path)?;
                file.write_all(contents.as_bytes())?;

                let mod_path = format!("src/lib/card/{}/mod.rs", edition);
                let mut mod_file = std::fs::File::options().append(true).open(mod_path)?;
                mod_file.write_all(format!("pub mod {};\n", filename).as_bytes())?;
                mod_file.write_all(format!("pub use {}::*;\n", filename).as_bytes())?;
            }

            Ok(())
        }
        CardType::Site => {
            let mut rdr = csv::Reader::from_reader(file);
            for record in rdr.records() {
                let record = record?;
                let mut site: SiteRecord = SiteRecord {
                    name: record[0].to_string(),
                    edition: record[1].to_string(),
                    provided_mana: record[2].parse().unwrap(),
                    provided_thresholds: record[3].to_string(),
                    types: vec![],
                    rarity: record[5].to_string(),
                };

                let types_str = record[4].to_string();
                if !types_str.is_empty() {
                    site.types = types_str.split(",").map(|s| s.to_string()).collect();
                }

                let edition = site.edition.to_lowercase();
                let filename = site.name.to_case(Case::Snake);
                let path = format!("src/lib/card/{}/{}.rs", edition, filename);
                if std::fs::exists(&path)? {
                    continue;
                }

                let struct_name = site.name.to_case(Case::Pascal);
                let site_types = site
                    .types
                    .iter()
                    .map(|m| format!("MinionType::{}", m))
                    .collect::<Vec<String>>()
                    .join(", ");

                let contents = SITE_TEMPLATE
                    .replace("{CardName}", &site.name)
                    .replace("{StructName}", &struct_name)
                    .replace("{SiteTypes}", &site_types)
                    .replace("{ProvidedMana}", &site.provided_mana.to_string())
                    .replace("{ProvidedThresholds}", &site.provided_thresholds)
                    .replace("{Rarity}", &site.rarity)
                    .replace("{Edition}", &site.edition);

                let mut file = std::fs::File::create(path)?;
                file.write_all(contents.as_bytes())?;

                let mod_path = format!("src/lib/card/{}/mod.rs", edition);
                let mut mod_file = std::fs::File::options().append(true).open(mod_path)?;
                mod_file.write_all(format!("pub mod {};\n", filename).as_bytes())?;
                mod_file.write_all(format!("pub use {}::*;\n", filename).as_bytes())?;
            }

            Ok(())
        }
        CardType::Magic => {
            let mut rdr = csv::Reader::from_reader(file);
            for record in rdr.records() {
                let record = record?;
                let card: MagicRecord = MagicRecord {
                    name: record[0].to_string(),
                    edition: record[1].to_string(),
                    mana_cost: record[2].parse().unwrap(),
                    required_thresholds: record[3].to_string(),
                    rarity: record[4].to_string(),
                };

                let edition = card.edition.to_lowercase();
                let filename = card.name.to_case(Case::Snake);
                let path = format!("src/lib/card/{}/{}.rs", edition, filename);
                if std::fs::exists(&path)? {
                    continue;
                }

                let struct_name = card.name.to_case(Case::Pascal);
                let contents = MAGIC_TEMPLATE
                    .replace("{CardName}", &card.name)
                    .replace("{StructName}", &struct_name)
                    .replace("{ManaCost}", &card.mana_cost.to_string())
                    .replace("{RequiredThresholds}", &card.required_thresholds)
                    .replace("{Rarity}", &card.rarity)
                    .replace("{Edition}", &card.edition);

                let mut file = std::fs::File::create(path)?;
                file.write_all(contents.as_bytes())?;

                let mod_path = format!("src/lib/card/{}/mod.rs", edition);
                let mut mod_file = std::fs::File::options().append(true).open(mod_path)?;
                mod_file.write_all(format!("pub mod {};\n", filename).as_bytes())?;
                mod_file.write_all(format!("pub use {}::*;\n", filename).as_bytes())?;
            }

            Ok(())
        }
        CardType::Artifact => {
            let mut rdr = csv::Reader::from_reader(file);
            for record in rdr.records() {
                let record = record?;
                let card: ArtifactRecord = ArtifactRecord {
                    name: record[0].to_string(),
                    edition: record[1].to_string(),
                    mana_cost: record[2].parse().unwrap(),
                    required_thresholds: record[3].to_string(),
                    rarity: record[4].to_string(),
                };

                let edition = card.edition.to_lowercase();
                let filename = card.name.to_case(Case::Snake);
                let path = format!("src/lib/card/{}/{}.rs", edition, filename);
                if std::fs::exists(&path)? {
                    continue;
                }

                let struct_name = card.name.to_case(Case::Pascal);
                let contents = ARTIFACT_TEMPLATE
                    .replace("{CardName}", &card.name)
                    .replace("{StructName}", &struct_name)
                    .replace("{ManaCost}", &card.mana_cost.to_string())
                    .replace("{RequiredThresholds}", &card.required_thresholds)
                    .replace("{Rarity}", &card.rarity)
                    .replace("{Edition}", &card.edition);

                let mut file = std::fs::File::create(path)?;
                file.write_all(contents.as_bytes())?;

                let mod_path = format!("src/lib/card/{}/mod.rs", edition);
                let mut mod_file = std::fs::File::options().append(true).open(mod_path)?;
                mod_file.write_all(format!("pub mod {};\n", filename).as_bytes())?;
                mod_file.write_all(format!("pub use {}::*;\n", filename).as_bytes())?;
            }

            Ok(())
        }
        CardType::Aura => {
            let mut rdr = csv::Reader::from_reader(file);
            for record in rdr.records() {
                let record = record?;
                let card: AuraRecord = AuraRecord {
                    name: record[0].to_string(),
                    edition: record[1].to_string(),
                    mana_cost: record[2].parse().unwrap(),
                    required_thresholds: record[3].to_string(),
                    rarity: record[4].to_string(),
                };

                let edition = card.edition.to_lowercase();
                let filename = card.name.to_case(Case::Snake);
                let path = format!("src/lib/card/{}/{}.rs", edition, filename);
                if std::fs::exists(&path)? {
                    continue;
                }

                let struct_name = card.name.to_case(Case::Pascal);
                let contents = AURA_TEMPLATE
                    .replace("{CardName}", &card.name)
                    .replace("{StructName}", &struct_name)
                    .replace("{ManaCost}", &card.mana_cost.to_string())
                    .replace("{RequiredThresholds}", &card.required_thresholds)
                    .replace("{Rarity}", &card.rarity)
                    .replace("{Edition}", &card.edition);

                let mut file = std::fs::File::create(path)?;
                file.write_all(contents.as_bytes())?;

                let mod_path = format!("src/lib/card/{}/mod.rs", edition);
                let mut mod_file = std::fs::File::options().append(true).open(mod_path)?;
                mod_file.write_all(format!("pub mod {};\n", filename).as_bytes())?;
                mod_file.write_all(format!("pub use {}::*;\n", filename).as_bytes())?;
            }

            Ok(())
        }

        _ => Err(anyhow::anyhow!("Unknown card type {:?}", args.card_type)),
    }
}
