use std::path::PathBuf;

use eframe::egui;

use crate::theme::{ThemeColors, theme_colors};

pub struct ExportDialog {
    open: bool,
    filename: String,
    scale_percent: u32,
    bg_enabled: bool,
    bg_color: [u8; 3],
    tab_map_enabled: bool,
    tab_map_scale_percent: u32,
    tab_map_bg_enabled: bool,
    tab_map_bg_color: [u8; 3],
}

pub enum ExportDialogAction {
    None,
    Export {
        path: PathBuf,
        zoom: f32,
        bg_color: Option<[u8; 4]>,
        tab_map: Option<TabMapExport>,
    },
}

pub struct TabMapExport {
    pub zoom: f32,
    pub bg_color: Option<[u8; 4]>,
}

impl Default for ExportDialog {
    fn default() -> Self {
        Self {
            open: false,
            filename: String::new(),
            scale_percent: 100,
            bg_enabled: false,
            bg_color: [11, 12, 14],
            tab_map_enabled: false,
            tab_map_scale_percent: 100,
            tab_map_bg_enabled: false,
            tab_map_bg_color: [11, 12, 14],
        }
    }
}

impl ExportDialog {
    /// Open the dialog, pre-filling the filename from the document name.
    pub fn open_for(&mut self, document_name: &str) {
        self.open = true;
        let base = document_name
            .strip_suffix(".map")
            .or_else(|| document_name.strip_suffix(".MAP"))
            .unwrap_or(document_name);
        self.filename = format!("{}.png", base);
        self.scale_percent = 100;
    }

    pub fn is_open(&self) -> bool {
        self.open
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

        let viewport = ctx.viewport_rect();
        let screen = ctx.content_rect();
        if !viewport.is_finite() || !screen.is_finite() {
            return ExportDialogAction::None;
        }

        let colors = theme_colors();
        let mut action = ExportDialogAction::None;

        // Dim the background
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
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

                // Title
                ui.label(
                    egui::RichText::new("Export as PNG")
                        .size(18.0)
                        .strong()
                        .color(colors.text),
                );

                ui.add_space(4.0);

                // --- Filename ---
                Self::field_label(ui, &colors, "Filename");
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.filename)
                            .desired_width(ui.available_width() - 80.0)
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

                // --- Scale ---
                Self::scale_row(ui, &colors, &mut self.scale_percent);

                ui.add_space(4.0);

                // --- Background color ---
                Self::background_row(ui, &colors, &mut self.bg_enabled, &mut self.bg_color);

                ui.add_space(4.0);

                // Output dimensions
                let zoom = self.scale_percent as f32 / 100.0;
                let (out_w, out_h) =
                    crate::export::compute_output_dimensions(map, zoom, wall_atlas);
                ui.label(
                    egui::RichText::new(format!("Output: {} \u{00D7} {} px", out_w, out_h))
                        .size(12.0)
                        .color(colors.muted),
                );

                ui.add_space(6.0);
                Self::separator(ui, &colors);
                ui.add_space(4.0);

                // --- Tab Map section ---
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
                    ui.add(egui::Checkbox::without_text(&mut self.tab_map_enabled));
                    ui.label(
                        egui::RichText::new("Export Tab Map")
                            .size(14.0)
                            .strong()
                            .color(if self.tab_map_enabled {
                                colors.text
                            } else {
                                colors.muted
                            }),
                    );
                });

                if self.tab_map_enabled {
                    ui.add_space(4.0);

                    // Tab map scale (same layout as map scale, no indent)
                    Self::scale_row(ui, &colors, &mut self.tab_map_scale_percent);

                    ui.add_space(4.0);

                    // Tab map background color (no indent)
                    Self::background_row(
                        ui,
                        &colors,
                        &mut self.tab_map_bg_enabled,
                        &mut self.tab_map_bg_color,
                    );

                    ui.add_space(4.0);

                    // Tab map output dimensions
                    let tm_zoom = self.tab_map_scale_percent as f32 / 100.0;
                    let (tm_w, tm_h) = crate::export::compute_tab_map_dimensions(map, tm_zoom);
                    ui.label(
                        egui::RichText::new(format!("Tab map: {} \u{00D7} {} px", tm_w, tm_h))
                            .size(12.0)
                            .color(colors.muted),
                    );
                }

                ui.add_space(6.0);
                Self::separator(ui, &colors);
                ui.add_space(4.0);

                // --- Buttons (right-aligned) ---
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);

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
                            let bg_color = if self.bg_enabled {
                                Some([self.bg_color[0], self.bg_color[1], self.bg_color[2], 255])
                            } else {
                                None
                            };
                            let tab_map = if self.tab_map_enabled {
                                let tm_bg = if self.tab_map_bg_enabled {
                                    Some([
                                        self.tab_map_bg_color[0],
                                        self.tab_map_bg_color[1],
                                        self.tab_map_bg_color[2],
                                        255,
                                    ])
                                } else {
                                    None
                                };
                                Some(TabMapExport {
                                    zoom: self.tab_map_scale_percent as f32 / 100.0,
                                    bg_color: tm_bg,
                                })
                            } else {
                                None
                            };
                            action = ExportDialogAction::Export {
                                path,
                                zoom,
                                bg_color,
                                tab_map,
                            };
                            self.open = false;
                        }

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
                            self.open = false;
                        }
                    });
                });
            });

        action
    }

    // --- Reusable row helpers ---

    /// Muted field label.
    fn field_label(ui: &mut egui::Ui, colors: &ThemeColors, text: &str) {
        ui.label(egui::RichText::new(text).size(13.0).color(colors.muted));
    }

    /// "Scale" label + slider + percentage on one row.
    fn scale_row(ui: &mut egui::Ui, colors: &ThemeColors, scale_percent: &mut u32) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
            ui.label(egui::RichText::new("Scale").size(13.0).color(colors.muted));
            let mut slot = (*scale_percent / 25) as i32;
            let slider_w = ui.available_width() - 50.0;
            let slider = egui::Slider::new(&mut slot, 1..=16)
                .show_value(false)
                .trailing_fill(true);
            ui.add_sized(egui::vec2(slider_w, 20.0), slider);
            *scale_percent = (slot as u32) * 25;
            ui.label(
                egui::RichText::new(format!("{}%", *scale_percent))
                    .size(14.0)
                    .strong()
                    .color(colors.text),
            );
        });
    }

    /// Checkbox + "Background" label + color picker swatch with visible border.
    fn background_row(
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        enabled: &mut bool,
        color: &mut [u8; 3],
    ) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
            ui.add(egui::Checkbox::without_text(enabled));
            ui.label(
                egui::RichText::new("Background")
                    .size(13.0)
                    .color(if *enabled { colors.text } else { colors.muted }),
            );
            if *enabled {
                let mut c = egui::Color32::from_rgb(color[0], color[1], color[2]);
                let resp = ui.color_edit_button_srgba(&mut c);
                *color = [c.r(), c.g(), c.b()];
                // Draw a visible border around the swatch so dark colors don't disappear
                ui.painter().rect_stroke(
                    resp.rect,
                    2.0,
                    egui::Stroke::new(1.0, colors.border),
                    egui::StrokeKind::Outside,
                );
            }
        });
    }

    /// Full-width horizontal separator line.
    fn separator(ui: &mut egui::Ui, colors: &ThemeColors) {
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 0.0, colors.border);
    }
}
