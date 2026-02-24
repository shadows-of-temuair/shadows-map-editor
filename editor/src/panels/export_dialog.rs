use std::path::PathBuf;

use eframe::egui;

use crate::theme::theme_colors;

pub struct ExportDialog {
    open: bool,
    filename: String,
    scale_percent: u32,
}

pub enum ExportDialogAction {
    None,
    Export { path: PathBuf, zoom: f32 },
}

impl Default for ExportDialog {
    fn default() -> Self {
        Self {
            open: false,
            filename: String::new(),
            scale_percent: 100,
        }
    }
}

impl ExportDialog {
    /// Open the dialog, pre-filling the filename from the document name.
    pub fn open_for(&mut self, document_name: &str) {
        self.open = true;
        // Strip .map extension (case-insensitive) then add .png
        let base = document_name
            .strip_suffix(".map")
            .or_else(|| document_name.strip_suffix(".MAP"))
            .unwrap_or(document_name);
        self.filename = format!("{}.png", base);
        self.scale_percent = 100;
    }

    /// Show the modal dialog. Returns the action taken.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        map: &map::Map,
        wall_atlas: Option<&render::SpriteAtlas>,
    ) -> ExportDialogAction {
        if !self.open {
            return ExportDialogAction::None;
        }

        let colors = theme_colors();
        let mut action = ExportDialogAction::None;

        // Dim the background
        let screen = ctx.input(|i| i.viewport_rect());
        egui::Area::new(egui::Id::new("export_backdrop"))
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let response = ui.allocate_response(screen.size(), egui::Sense::click());
                ui.painter().rect_filled(
                    screen,
                    0.0,
                    egui::Color32::from_rgba_unmultiplied(0, 0, 0, 140),
                );
                if response.clicked() {
                    self.open = false;
                }
            });

        egui::Window::new("")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .fixed_size(egui::vec2(440.0, 0.0))
            .frame(
                egui::Frame::NONE
                    .fill(colors.bg_2)
                    .stroke(egui::Stroke::new(1.0, colors.border))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::same(24)),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 12.0);

                // Title
                ui.label(
                    egui::RichText::new("Export as PNG")
                        .size(18.0)
                        .strong()
                        .color(colors.text),
                );
                ui.add_space(4.0);

                // Filename
                ui.label(
                    egui::RichText::new("Filename")
                        .size(13.0)
                        .color(colors.muted),
                );
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.filename)
                            .desired_width(340.0)
                            .font(egui::FontId::proportional(14.0))
                            .margin(egui::Margin::symmetric(8, 6)),
                    );
                    let browse_btn = ui.add(
                        egui::Button::new(
                            egui::RichText::new("Browse\u{2026}")
                                .size(13.0)
                                .color(colors.text),
                        )
                        .fill(colors.bg_3)
                        .stroke(egui::Stroke::new(1.0, colors.border))
                        .corner_radius(4.0),
                    );
                    if browse_btn.clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("PNG", &["png"])
                            .set_file_name(&self.filename)
                            .save_file()
                        {
                            self.filename = path.to_string_lossy().into_owned();
                        }
                    }
                });

                ui.add_space(4.0);

                // Scale
                ui.label(
                    egui::RichText::new("Scale")
                        .size(13.0)
                        .color(colors.muted),
                );
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
                    let mut scale_slot = (self.scale_percent / 25) as i32;
                    let slider_w = ui.available_width() - 50.0;
                    let slider = egui::Slider::new(&mut scale_slot, 1..=16)
                        .show_value(false)
                        .trailing_fill(true);
                    ui.add_sized(egui::vec2(slider_w, 20.0), slider);
                    self.scale_percent = (scale_slot as u32) * 25;
                    ui.label(
                        egui::RichText::new(format!("{}%", self.scale_percent))
                            .size(14.0)
                            .strong()
                            .color(colors.text),
                    );
                });

                ui.add_space(4.0);

                // Output dimensions
                let zoom = self.scale_percent as f32 / 100.0;
                let (out_w, out_h) =
                    crate::export::compute_output_dimensions(map, zoom, wall_atlas);
                ui.label(
                    egui::RichText::new(format!("Output size: {} \u{00D7} {} px", out_w, out_h))
                        .size(13.0)
                        .color(colors.muted),
                );

                ui.add_space(8.0);

                // Separator
                let (sep_rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 1.0),
                    egui::Sense::hover(),
                );
                ui.painter().rect_filled(sep_rect, 0.0, colors.border);

                ui.add_space(4.0);

                // Buttons (right-aligned)
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);

                        // Export (primary action)
                        let export_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("  Export  ")
                                    .size(14.0)
                                    .color(egui::Color32::from_rgb(10, 11, 13)),
                            )
                            .fill(colors.accent)
                            .stroke(egui::Stroke::NONE)
                            .corner_radius(4.0)
                            .min_size(egui::vec2(80.0, 32.0)),
                        );
                        if export_btn.clicked() && !self.filename.is_empty() {
                            let path = PathBuf::from(&self.filename);
                            let zoom = self.scale_percent as f32 / 100.0;
                            action = ExportDialogAction::Export { path, zoom };
                            self.open = false;
                        }

                        // Cancel (secondary)
                        let cancel_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("Cancel")
                                    .size(14.0)
                                    .color(colors.text),
                            )
                            .fill(colors.bg_3)
                            .stroke(egui::Stroke::new(1.0, colors.border))
                            .corner_radius(4.0)
                            .min_size(egui::vec2(80.0, 32.0)),
                        );
                        if cancel_btn.clicked() {
                            self.open = false;
                        }
                    });
                });
            });

        action
    }
}
