use eframe::egui;

use crate::panels::Tool;
use crate::theme::{ThemeColors, theme_colors};

const STATUS_BAR_HEIGHT: f32 = 28.0;

#[derive(Clone, Copy, PartialEq)]
pub enum StatusBarAction {
    None,
    ZoomIn,
    ZoomOut,
    SetDimensions(u16, u16),
}

pub struct StatusBarPanel;

impl StatusBarPanel {
    pub fn show(
        ctx: &egui::Context,
        map: &map::Map,
        active_tool: Tool,
        hover_tile: (u16, u16),
        zoom: f32,
    ) -> StatusBarAction {
        let colors = theme_colors();
        let mut action = StatusBarAction::None;

        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(STATUS_BAR_HEIGHT)
            .frame(
                egui::Frame::NONE
                    .fill(colors.bg_2)
                    .inner_margin(egui::Margin {
                        left: 12,
                        right: 12,
                        top: 0,
                        bottom: 0,
                    }),
            )
            .show(ctx, |ui| {
                // Top border
                let rect = ui.max_rect();
                ui.painter().line_segment(
                    [
                        egui::pos2(rect.left() - 12.0, rect.top()),
                        egui::pos2(rect.right() + 12.0, rect.top()),
                    ],
                    egui::Stroke::new(1.0, colors.border),
                );

                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(16.0, 0.0);

                    // Tool indicator (left)
                    ui.label(
                        egui::RichText::new(active_tool.tooltip())
                            .size(13.0)
                            .color(colors.muted),
                    );

                    Self::separator(ui, &colors);

                    // Status (left, accent)
                    ui.label(egui::RichText::new("Ready").size(13.0).color(colors.accent));

                    // Right-aligned section
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);

                        // Position (rightmost)
                        let pos_text = format!("Pos: {}, {}", hover_tile.0, hover_tile.1);
                        ui.label(
                            egui::RichText::new(pos_text).size(13.0).color(colors.muted),
                        );

                        Self::separator(ui, &colors);

                        // Dimensions dropdown
                        ui.label(
                            egui::RichText::new("Size").size(13.0).color(colors.muted),
                        );
                        let dim_text = format!("{}×{} ▾", map.width, map.height);
                        let dim_response = ui.add(
                            egui::Button::new(
                                egui::RichText::new(&dim_text).size(13.0).color(colors.text),
                            )
                            .fill(egui::Color32::TRANSPARENT)
                            .stroke(egui::Stroke::NONE)
                            .corner_radius(0.0),
                        );

                        egui::Popup::from_toggle_button_response(&dim_response)
                            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                            .width(100.0)
                            .show(|ui| {
                                let tile_count = map.tiles.len();
                                let dims = map::all_dimensions(tile_count);
                                egui::ScrollArea::vertical()
                                    .max_height(200.0)
                                    .show(ui, |ui| {
                                        for (w, h) in dims {
                                            let is_current =
                                                w == map.width && h == map.height;
                                            let label = format!("{}×{}", w, h);
                                            let text_color = if is_current {
                                                colors.accent
                                            } else {
                                                colors.text
                                            };
                                            if ui
                                                .add(
                                                    egui::Button::new(
                                                        egui::RichText::new(&label)
                                                            .size(13.0)
                                                            .color(text_color),
                                                    )
                                                    .fill(egui::Color32::TRANSPARENT)
                                                    .stroke(egui::Stroke::NONE)
                                                    .min_size(egui::vec2(
                                                        ui.available_width(),
                                                        0.0,
                                                    )),
                                                )
                                                .clicked()
                                            {
                                                action =
                                                    StatusBarAction::SetDimensions(w, h);
                                                egui::Popup::close_id(
                                                    ui.ctx(),
                                                    egui::Popup::default_response_id(
                                                        &dim_response,
                                                    ),
                                                );
                                            }
                                        }
                                    });
                            });

                        Self::separator(ui, &colors);

                        // Zoom controls: [+] "Zoom N%" [-] (RTL order)
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("+").size(14.0).color(colors.muted),
                                )
                                .fill(egui::Color32::TRANSPARENT)
                                .stroke(egui::Stroke::NONE)
                                .min_size(egui::vec2(18.0, 18.0)),
                            )
                            .clicked()
                        {
                            action = StatusBarAction::ZoomIn;
                        }

                        let zoom_text =
                            format!("Zoom {}%", (zoom * 100.0).round() as i32);
                        ui.label(
                            egui::RichText::new(zoom_text).size(13.0).color(colors.text),
                        );

                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("-").size(14.0).color(colors.muted),
                                )
                                .fill(egui::Color32::TRANSPARENT)
                                .stroke(egui::Stroke::NONE)
                                .min_size(egui::vec2(18.0, 18.0)),
                            )
                            .clicked()
                        {
                            action = StatusBarAction::ZoomOut;
                        }
                    });
                });
            });

        action
    }

    fn separator(ui: &mut egui::Ui, colors: &ThemeColors) {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, 14.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 0.0, colors.border);
    }
}
