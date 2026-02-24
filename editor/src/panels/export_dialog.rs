use std::path::PathBuf;

use eframe::egui;

use crate::theme::theme_colors;

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

                // Background color
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
                    ui.add(egui::Checkbox::without_text(&mut self.bg_enabled));
                    ui.label(
                        egui::RichText::new("Background")
                            .size(13.0)
                            .color(if self.bg_enabled { colors.text } else { colors.muted }),
                    );
                    if self.bg_enabled {
                        let mut c = egui::Color32::from_rgb(
                            self.bg_color[0],
                            self.bg_color[1],
                            self.bg_color[2],
                        );
                        ui.color_edit_button_srgba(&mut c);
                        self.bg_color = [c.r(), c.g(), c.b()];
                    }
                });

                ui.add_space(2.0);

                // Output dimensions
                let zoom = self.scale_percent as f32 / 100.0;
                let (out_w, out_h) =
                    crate::export::compute_output_dimensions(map, zoom, wall_atlas);
                ui.label(
                    egui::RichText::new(format!("Output size: {} \u{00D7} {} px", out_w, out_h))
                        .size(13.0)
                        .color(colors.muted),
                );

                ui.add_space(4.0);

                // --- Tab Map section ---
                let (sep_rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 1.0),
                    egui::Sense::hover(),
                );
                ui.painter().rect_filled(sep_rect, 0.0, colors.border);
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
                    ui.add(egui::Checkbox::without_text(&mut self.tab_map_enabled));
                    ui.label(
                        egui::RichText::new("Export Tab Map")
                            .size(14.0)
                            .color(if self.tab_map_enabled {
                                colors.text
                            } else {
                                colors.muted
                            }),
                    );
                });

                if self.tab_map_enabled {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
                        ui.add_space(24.0);
                        ui.label(
                            egui::RichText::new("Scale")
                                .size(13.0)
                                .color(colors.muted),
                        );
                        let mut tm_slot = (self.tab_map_scale_percent / 25) as i32;
                        let slider_w = ui.available_width() - 50.0;
                        let slider = egui::Slider::new(&mut tm_slot, 1..=16)
                            .show_value(false)
                            .trailing_fill(true);
                        ui.add_sized(egui::vec2(slider_w, 20.0), slider);
                        self.tab_map_scale_percent = (tm_slot as u32) * 25;
                        ui.label(
                            egui::RichText::new(format!("{}%", self.tab_map_scale_percent))
                                .size(14.0)
                                .strong()
                                .color(colors.text),
                        );
                    });

                    // Tab map background color
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
                        ui.add_space(24.0);
                        ui.add(egui::Checkbox::without_text(&mut self.tab_map_bg_enabled));
                        ui.label(
                            egui::RichText::new("Background")
                                .size(13.0)
                                .color(if self.tab_map_bg_enabled {
                                    colors.text
                                } else {
                                    colors.muted
                                }),
                        );
                        if self.tab_map_bg_enabled {
                            let mut c = egui::Color32::from_rgb(
                                self.tab_map_bg_color[0],
                                self.tab_map_bg_color[1],
                                self.tab_map_bg_color[2],
                            );
                            ui.color_edit_button_srgba(&mut c);
                            self.tab_map_bg_color = [c.r(), c.g(), c.b()];
                        }
                    });

                    // Tab map output dimensions
                    let tm_zoom = self.tab_map_scale_percent as f32 / 100.0;
                    let (tm_w, tm_h) =
                        crate::export::compute_tab_map_dimensions(map, tm_zoom);
                    ui.horizontal(|ui| {
                        ui.add_space(24.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "Tab map size: {} \u{00D7} {} px",
                                tm_w, tm_h
                            ))
                            .size(13.0)
                            .color(colors.muted),
                        );
                    });
                }

                ui.add_space(4.0);

                // Separator
                let (sep_rect2, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 1.0),
                    egui::Sense::hover(),
                );
                ui.painter().rect_filled(sep_rect2, 0.0, colors.border);

                ui.add_space(4.0);

                // Buttons (right-aligned)
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
