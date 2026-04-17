use csv::{ReaderBuilder, WriterBuilder};
use scraper::{Html, Selector};
use serde_json::Value;
use std::error::Error;
use std::fs::File;
use std::thread;
use std::time::Duration;

fn get_card_text(url: &str) -> String {
    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
        .unwrap();

    match client.get(url).timeout(Duration::from_secs(10)).send() {
        Ok(response) => {
            if response.status().is_success() {
                let body = response.text().unwrap_or_default();
                let fragment = Html::parse_document(&body);
                let selector = Selector::parse("script#__NEXT_DATA__").unwrap();

                if let Some(element) = fragment.select(&selector).next() {
                    let json_text = element.text().collect::<String>();
                    let v: Value = serde_json::from_str(&json_text).unwrap_or(Value::Null);

                    // Path found in your provided bullfrog.html
                    let path = "/props/pageProps/trpcState/json/queries/0/state/data/rulesText";
                    if let Some(rules) = v.pointer(path)
                        && let Some(text) = rules.as_str() {
                            println!("{}", text);
                            return text.replace("\n", " ").trim().to_string();
                        }
                }
                "Text path not found".to_string()
            } else {
                format!("HTTP Error: {}", response.status())
            }
        }
        Err(e) => format!("Request failed: {}", e),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let input_path = "documents/Sorcery Contested Realm Product Tracker - Beta.csv";
    let output_path = "Sorcery_Tracker_With_Text.csv";

    let file = File::open(input_path)?;
    let mut rdr = ReaderBuilder::new().from_reader(file);
    let headers = rdr.headers()?.clone();

    let mut wtr = WriterBuilder::new().from_writer(File::create(output_path)?);

    let mut new_headers = headers.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    new_headers.push("Card Description".to_string());
    wtr.write_record(&new_headers)?;

    let link_idx = headers
        .iter()
        .position(|h| h == "Curiosa.io link")
        .expect("Link column not found");
    let name_idx = headers.iter().position(|h| h == "Card name").unwrap_or(0);

    for result in rdr.records() {
        let record = result?;
        let card_name = &record[name_idx];
        let url = &record[link_idx];

        if !url.is_empty() && url.starts_with("http") {
            println!("Processing: {}", card_name);
            let description = get_card_text(url);

            let mut new_record = record.iter().map(|s| s.to_string()).collect::<Vec<_>>();
            new_record.push(description);
            wtr.write_record(&new_record)?;

            // Critical: Don't remove this sleep or you may get IP blocked
            thread::sleep(Duration::from_millis(800));
        }
    }

    wtr.flush()?;
    println!("Complete! Created {}", output_path);
    Ok(())
}
