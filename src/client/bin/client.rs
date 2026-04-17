use crate::config::SCREEN_RECT;
use crate::scene::Scene;
use crate::scene::menu::Menu;
use crate::texture_cache::TextureCache;
use eframe::egui;
use sorcerers::networking;
use sorcerers::networking::message::{Message, ServerMessage};
use std::sync::RwLock;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

// Embedded fonts — compiled into the binary so they're always available.
static NOTO_SANS: &[u8] = include_bytes!("../../../assets/fonts/NotoSans-Regular.ttf");
static NOTO_SYMBOLS: &[u8] = include_bytes!("../../../assets/fonts/NotoSansSymbols2-Regular.ttf");

pub struct SorcerersApp {
    scene: Scene,
    _runtime: Runtime,
    rx: mpsc::UnboundedReceiver<ServerMessage>,
}

impl SorcerersApp {
    pub fn new(cc: &eframe::CreationContext, server_url: &str) -> anyhow::Result<Self> {
        TextureCache::init();
        Self::setup_style(&cc.egui_ctx);

        let client = networking::client::Client::connect(server_url)?;
        let (tx, rx) = mpsc::unbounded_channel::<ServerMessage>();

        let rt = Runtime::new()?;

        let receiver = client.clone();
        rt.spawn(async move {
            loop {
                if let Some(Message::ServerMessage(msg)) =
                    receiver.recv().expect("message should be received")
                {
                    let _ = tx.send(msg);
                }
            }
        });

        let scene = Scene::Menu(Menu::new(client));

        Ok(SorcerersApp {
            scene,
            _runtime: rt,
            rx,
        })
    }

    fn setup_style(ctx: &egui::Context) {
        use egui::epaint::CornerRadius;
        use egui::{Color32, FontId, Stroke, TextStyle, style::WidgetVisuals};

        // ── Embedded fonts ───────────────────────────────────────────────────
        // Start from egui's defaults (which include its own compact Latin font),
        // then append Noto Sans and Noto Sans Symbols 2 as fallbacks so every
        // Unicode character we use in the UI renders correctly.
        {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "NotoSans".to_owned(),
                std::sync::Arc::new(egui::FontData::from_static(NOTO_SANS)),
            );
            fonts.font_data.insert(
                "NotoSymbols2".to_owned(),
                std::sync::Arc::new(egui::FontData::from_static(NOTO_SYMBOLS)),
            );
            // Append after the built-in font so basic Latin keeps the default look;
            // missing glyphs (symbols, dingbats, etc.) fall through to these fonts.
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                let list = fonts.families.entry(family).or_default();
                list.push("NotoSans".to_owned());
                list.push("NotoSymbols2".to_owned());
            }
            ctx.set_fonts(fonts);
        }

        // ── Visuals ─────────────────────────────────────────────────────────
        let mut visuals = egui::Visuals::dark();

        // Overall background / window chrome
        visuals.window_corner_radius = CornerRadius::same(8);
        visuals.window_shadow = egui::Shadow::NONE;
        visuals.window_fill = Color32::from_rgb(18, 18, 24);
        visuals.window_stroke = Stroke::new(1.0, Color32::from_rgb(60, 60, 80));
        visuals.panel_fill = Color32::TRANSPARENT;
        visuals.extreme_bg_color = Color32::from_rgb(12, 12, 18);

        let blue = Color32::from_rgb(30, 144, 255); // dodger blue
        let blue_hovered = Color32::from_rgb(65, 105, 225); // royal blue
        let blue_active = Color32::from_rgb(25, 25, 112); // midnight blue
        let btn_text = Stroke::new(1.0, Color32::WHITE);
        let cr = CornerRadius::same(1);

        visuals.widgets.inactive = WidgetVisuals {
            bg_fill: blue,
            weak_bg_fill: blue,
            bg_stroke: Stroke::NONE,
            fg_stroke: btn_text,
            corner_radius: cr,
            expansion: 0.0,
        };
        visuals.widgets.hovered = WidgetVisuals {
            bg_fill: blue_hovered,
            weak_bg_fill: blue_hovered,
            bg_stroke: Stroke::NONE,
            fg_stroke: btn_text,
            corner_radius: cr,
            expansion: 1.0,
        };
        visuals.widgets.active = WidgetVisuals {
            bg_fill: blue_active,
            weak_bg_fill: blue_active,
            bg_stroke: Stroke::NONE,
            fg_stroke: btn_text,
            corner_radius: cr,
            expansion: 0.0,
        };
        // Open (non-interactive) widgets — used for text inputs, labels inside frames
        visuals.widgets.open = WidgetVisuals {
            bg_fill: Color32::from_rgb(30, 30, 42),
            weak_bg_fill: Color32::from_rgb(30, 30, 42),
            bg_stroke: Stroke::new(1.0, Color32::from_rgb(80, 80, 110)),
            fg_stroke: Stroke::new(1.0, Color32::WHITE),
            corner_radius: cr,
            expansion: 0.0,
        };
        visuals.widgets.noninteractive = WidgetVisuals {
            bg_fill: Color32::from_rgb(30, 30, 42),
            weak_bg_fill: Color32::from_rgb(22, 22, 32),
            bg_stroke: Stroke::new(1.0, Color32::from_rgb(60, 60, 80)),
            fg_stroke: Stroke::new(1.0, Color32::from_rgb(200, 200, 220)),
            corner_radius: cr,
            expansion: 0.0,
        };
        visuals.selection.bg_fill = blue;

        ctx.set_visuals(visuals);

        // ── Spacing / style ──────────────────────────────────────────────────
        let mut style = (*ctx.global_style()).clone();
        style.spacing.button_padding = egui::vec2(14.0, 8.0);
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.text_edit_width = 300.0;

        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(18.0));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(22.0));
        style
            .text_styles
            .insert(TextStyle::Heading, FontId::proportional(24.0));
        style
            .text_styles
            .insert(TextStyle::Small, FontId::proportional(14.0));

        ctx.set_global_style(style);
    }
}

impl eframe::App for SorcerersApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update screen rect
        let screen = ctx.content_rect();
        {
            let rect_lock = SCREEN_RECT.get_or_init(|| RwLock::new(screen));
            if let Ok(mut r) = rect_lock.write() {
                *r = screen;
            }
        }

        // Flush loaded textures
        TextureCache::flush_blocking(ctx);

        // Drain incoming server messages
        let mut received_message = false;
        while let Ok(msg) = self.rx.try_recv() {
            if let Some(new_scene) = self.scene.process_message(&msg) {
                self.scene = new_scene;
            }
            received_message = true;
        }
        if received_message {
            ctx.request_repaint();
        }

        // Update game state
        self.scene.update(ctx);
    }

    /// When the window is closed the tokio networking task is blocked on
    /// `receiver.recv()` and will never wake up, causing the process to hang
    /// on drop.  Force-exit immediately instead.
    fn on_exit(&mut self) {
        std::process::exit(0);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
            .show_inside(ui, |ui| {
                if let Some(new_scene) = self.scene.render(ui) {
                    self.scene = new_scene;
                }
            });
    }
}
