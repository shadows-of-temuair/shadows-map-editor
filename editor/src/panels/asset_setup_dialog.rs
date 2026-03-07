use eframe::egui;

use crate::theme::theme_colors;
use crate::widgets::icons;

const ASSET_SETUP_ICON: &str = "\u{EB02}";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetSetupDialogAction {
    None,
    SelectFolder,
    NotNow,
}

#[derive(Default)]
pub struct AssetSetupDialog {
    open: bool,
}

impl AssetSetupDialog {
    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn show(&mut self, ctx: &egui::Context) -> AssetSetupDialogAction {
        if !self.open {
            return AssetSetupDialogAction::None;
        }

        let viewport = ctx.viewport_rect();
        let screen = ctx.content_rect();
        if !viewport.is_finite() || !screen.is_finite() {
            return AssetSetupDialogAction::None;
        }

        let colors = theme_colors();
        let mut open = self.open;
        let mut action = AssetSetupDialogAction::None;

        egui::Area::new(egui::Id::new("asset_setup_backdrop"))
            .order(egui::Order::Middle)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let _ = ui.allocate_response(screen.size(), egui::Sense::click());
                ui.painter().rect_filled(
                    screen,
                    0.0,
                    egui::Color32::from_rgba_unmultiplied(0, 0, 0, 140),
                );
            });

        egui::Window::new("")
            .order(egui::Order::Foreground)
            .id(egui::Id::new("asset_setup_dialog"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .fixed_size(egui::vec2(460.0, 0.0))
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .frame(
                egui::Frame::NONE
                    .fill(colors.bg_2)
                    .stroke(egui::Stroke::new(1.0, colors.border))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::same(24)),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

                draw_modal_title(ui, &colors, ASSET_SETUP_ICON, "Asset Setup");
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(
                        "The editor needs the original Dark Ages .dat archives to render tiles and walls properly.",
                    )
                    .size(13.0)
                    .color(colors.text),
                );
                ui.label(
                    egui::RichText::new(
                        "Select your Dark Ages install folder and the editor will copy its .dat files into the local assets/ folder. This keeps the editor self-contained and avoids depending on your install path every time it starts.",
                    )
                    .size(12.0)
                    .color(colors.muted),
                );

                let submit = ui.input(|i| i.key_pressed(egui::Key::Enter));
                let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
                if escape {
                    action = AssetSetupDialogAction::NotNow;
                }

                ui.add_space(6.0);
                let (sep_rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 1.0),
                    egui::Sense::hover(),
                );
                ui.painter().rect_filled(sep_rect, 0.0, colors.border);

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);

                        let not_now_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("Not Now").size(14.0).color(colors.text),
                            )
                            .fill(colors.bg_3)
                            .stroke(egui::Stroke::new(1.0, colors.border))
                            .corner_radius(4.0)
                            .min_size(egui::vec2(92.0, 32.0)),
                        );
                        if not_now_btn.clicked() {
                            action = AssetSetupDialogAction::NotNow;
                        }

                        let select_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("Select Dark Ages Folder")
                                    .size(14.0)
                                    .color(egui::Color32::from_rgb(10, 11, 13)),
                            )
                            .fill(colors.accent)
                            .stroke(egui::Stroke::NONE)
                            .corner_radius(4.0)
                            .min_size(egui::vec2(188.0, 32.0)),
                        );
                        if select_btn.clicked() || submit {
                            action = AssetSetupDialogAction::SelectFolder;
                        }
                    });
                });
            });

        if !matches!(action, AssetSetupDialogAction::None) {
            open = false;
        }
        self.open = open;
        action
    }
}

fn draw_modal_title(
    ui: &mut egui::Ui,
    colors: &crate::theme::ThemeColors,
    icon: &str,
    title: &str,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        ui.label(
            egui::RichText::new(icon)
                .font(icons::symbol_icon_font_id(18.0))
                .color(colors.text),
        );
        ui.label(
            egui::RichText::new(title)
                .size(18.0)
                .strong()
                .color(colors.text),
        );
    });
}
