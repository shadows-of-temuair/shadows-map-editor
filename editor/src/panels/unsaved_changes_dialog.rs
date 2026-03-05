use eframe::egui;

use crate::theme::theme_colors;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsavedChangesDialogAction {
    None,
    Save,
    Discard,
    Cancel,
}

#[derive(Default)]
pub struct UnsavedChangesDialog {
    open: bool,
    document_name: String,
}

impl UnsavedChangesDialog {
    pub fn open_for(&mut self, document_name: &str) {
        self.open = true;
        self.document_name = document_name.to_string();
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn show(&mut self, ctx: &egui::Context) -> UnsavedChangesDialogAction {
        if !self.open {
            return UnsavedChangesDialogAction::None;
        }

        let colors = theme_colors();
        let mut open = self.open;
        let mut action = UnsavedChangesDialogAction::None;

        let screen = ctx.input(|i| i.viewport_rect());
        egui::Area::new(egui::Id::new("unsaved_changes_backdrop"))
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let response = ui.allocate_response(screen.size(), egui::Sense::click());
                ui.painter().rect_filled(
                    screen,
                    0.0,
                    egui::Color32::from_rgba_unmultiplied(0, 0, 0, 140),
                );
                if response.clicked() {
                    action = UnsavedChangesDialogAction::Cancel;
                }
            });

        egui::Window::new("")
            .id(egui::Id::new("unsaved_changes_dialog"))
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

                ui.label(
                    egui::RichText::new("Unsaved Changes")
                        .size(18.0)
                        .strong()
                        .color(colors.text),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!(
                        "Save changes to \"{}\" before discarding them?",
                        self.document_name
                    ))
                    .size(13.0)
                    .color(colors.text),
                );
                ui.label(
                    egui::RichText::new("Discarding will permanently lose the current edits.")
                        .size(12.0)
                        .color(colors.muted),
                );

                let submit = ui.input(|i| i.key_pressed(egui::Key::Enter));
                let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
                if escape {
                    action = UnsavedChangesDialogAction::Cancel;
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
                            action = UnsavedChangesDialogAction::Cancel;
                        }

                        let discard_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("Discard")
                                    .size(14.0)
                                    .color(colors.text),
                            )
                            .fill(colors.bg_3)
                            .stroke(egui::Stroke::new(1.0, colors.border))
                            .corner_radius(4.0)
                            .min_size(egui::vec2(88.0, 32.0)),
                        );
                        if discard_btn.clicked() {
                            action = UnsavedChangesDialogAction::Discard;
                        }

                        let save_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("Save")
                                    .size(14.0)
                                    .color(egui::Color32::from_rgb(10, 11, 13)),
                            )
                            .fill(colors.accent)
                            .stroke(egui::Stroke::NONE)
                            .corner_radius(4.0)
                            .min_size(egui::vec2(80.0, 32.0)),
                        );
                        if save_btn.clicked() || submit {
                            action = UnsavedChangesDialogAction::Save;
                        }
                    });
                });
            });

        if !matches!(action, UnsavedChangesDialogAction::None) {
            open = false;
        }
        self.open = open;
        action
    }
}
