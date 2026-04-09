mod client;
pub mod components;
mod config;
pub mod element_icon;
pub mod input;
mod render;
mod scene;
mod texture_cache;

use eframe::egui;

fn main() -> eframe::Result {
    let server_url = std::env::var("SORCERERS_SERVER_URL").unwrap_or_else(|_| "127.0.0.1:5000".to_string());

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Sorcerers")
            .with_fullscreen(true),
        ..Default::default()
    };

    eframe::run_native(
        "Sorcerers",
        native_options,
        Box::new(|cc| {
            Ok(Box::new(
                client::SorcerersApp::new(cc, &server_url).expect("client init failed"),
            ))
        }),
    )
}
