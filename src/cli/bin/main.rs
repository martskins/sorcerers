mod minion_template;

use crate::minion_template::MINION_TEMPLATE;
use clap::Parser;
use convert_case::{Case, Casing};
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

#[derive(Debug, serde::Deserialize)]
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
                if modifiers_str.is_empty() {
                    minion.modifiers = modifiers_str.split(",").map(|s| s.to_string()).collect();
                }

                let types_str = record[8].to_string();
                if types_str.is_empty() {
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

                let mut mod_file = std::fs::File::create(format!("src/lib/card/{}/mod.rs", edition))?;
                mod_file.write_all(format!("pub mod {};\n", filename).as_bytes())?;
                mod_file.write_all(format!("pub use {}::*;\n", filename).as_bytes())?;
            }

            Ok(())
        }
        _ => Err(anyhow::anyhow!("Unknown card type {:?}", args.card_type)),
    }
}
