#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod app;
mod document;
pub mod export;
mod map_list;
mod palette_lookup;
mod panels;
mod prefab;
mod shape;
mod theme;
mod widgets;

use app::EditorApp;
use eframe::egui;
use tracing::warn;

const APP_TITLE: &str = "Shadows Map Editor";

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .init();

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([960.0, 600.0])
        .with_decorations(false)
        .with_title(APP_TITLE);

    if let Some(icon) = load_app_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(|cc| Ok(Box::new(EditorApp::new(cc)))),
    )
}

fn load_app_icon() -> Option<egui::IconData> {
    match eframe::icon_data::from_png_bytes(include_bytes!("../app-icon.png")) {
        Ok(icon) => Some(icon),
        Err(error) => {
            warn!(error = ?error, "failed to decode app-icon.png for viewport icon");
            None
        }
    }
}
