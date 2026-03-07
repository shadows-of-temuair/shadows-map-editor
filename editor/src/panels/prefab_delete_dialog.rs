use eframe::egui;

use crate::theme::theme_colors;
use crate::widgets::icons;

const PREFAB_DELETE_ICON: &str = "\u{26A0}";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrefabDeleteDialogAction {
    None,
    Delete,
    Cancel,
}

#[derive(Default)]
pub struct PrefabDeleteDialog {
    open: bool,
    prefab_name: String,
}

impl PrefabDeleteDialog {
    pub fn open_for(&mut self, prefab_name: &str) {
        self.open = true;
        self.prefab_name = prefab_name.to_string();
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn show(&mut self, ctx: &egui::Context) -> PrefabDeleteDialogAction {
        if !self.open {
            return PrefabDeleteDialogAction::None;
        }

        let viewport = ctx.viewport_rect();
        let screen = ctx.content_rect();
        if !viewport.is_finite() || !screen.is_finite() {
            return PrefabDeleteDialogAction::None;
        }

        let colors = theme_colors();
        let mut open = self.open;
        let mut action = PrefabDeleteDialogAction::None;

        egui::Area::new(egui::Id::new("prefab_delete_backdrop"))
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let response = ui.allocate_response(screen.size(), egui::Sense::click());
                ui.painter().rect_filled(
                    screen,
                    0.0,
                    egui::Color32::from_rgba_unmultiplied(0, 0, 0, 140),
                );
                if response.clicked() {
                    action = PrefabDeleteDialogAction::Cancel;
                }
            });

        egui::Window::new("")
            .id(egui::Id::new("prefab_delete_dialog"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .fixed_size(egui::vec2(400.0, 0.0))
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

                draw_modal_title(ui, &colors, PREFAB_DELETE_ICON, "Delete Prefab");
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!("Remove prefab \"{}\"?", self.prefab_name))
                        .size(13.0)
                        .color(colors.text),
                );
                ui.label(
                    egui::RichText::new("This will remove it and cannot be undone.")
                        .size(12.0)
                        .color(colors.muted),
                );

                let submit = ui.input(|i| i.key_pressed(egui::Key::Enter));
                let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
                if escape {
                    action = PrefabDeleteDialogAction::Cancel;
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

                        let cancel_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("Cancel").size(14.0).color(colors.text),
                            )
                            .fill(colors.bg_3)
                            .stroke(egui::Stroke::new(1.0, colors.border))
                            .corner_radius(4.0)
                            .min_size(egui::vec2(80.0, 32.0)),
                        );
                        if cancel_btn.clicked() {
                            action = PrefabDeleteDialogAction::Cancel;
                        }

                        let delete_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("Delete")
                                    .size(14.0)
                                    .color(egui::Color32::from_rgb(10, 11, 13)),
                            )
                            .fill(colors.accent)
                            .stroke(egui::Stroke::NONE)
                            .corner_radius(4.0)
                            .min_size(egui::vec2(88.0, 32.0)),
                        );
                        if delete_btn.clicked() || submit {
                            action = PrefabDeleteDialogAction::Delete;
                        }
                    });
                });
            });

        if !matches!(action, PrefabDeleteDialogAction::None) {
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
