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
        status_message: &str,
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
                    ui.label(egui::RichText::new(status_message).size(13.0).color(colors.accent));

                    // Right-aligned section
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);

                        // Position (rightmost) — fixed width to prevent layout jitter
                        let pos_text = format!("Pos: {:>3}, {:>3}", hover_tile.0, hover_tile.1);
                        ui.label(
                            egui::RichText::new(pos_text)
                                .size(13.0)
                                .color(colors.muted)
                                .family(egui::FontFamily::Monospace),
                        );

                        Self::separator(ui, &colors);

                        // Dimensions dropdown — "caret | NxN | Size" (RTL order)
                        // Caret triangle (allocated in RTL flow, appears rightmost)
                        let (caret_rect, _) = ui.allocate_exact_size(
                            egui::vec2(14.0, 14.0),
                            egui::Sense::hover(),
                        );
                        {
                            let cx = caret_rect.center().x;
                            let cy = caret_rect.center().y;
                            let s = 4.5;
                            ui.painter().add(egui::Shape::convex_polygon(
                                vec![
                                    egui::pos2(cx - s, cy - s * 0.5),
                                    egui::pos2(cx + s, cy - s * 0.5),
                                    egui::pos2(cx, cy + s * 0.5),
                                ],
                                colors.muted,
                                egui::Stroke::NONE,
                            ));
                        }
                        let dim_text = format!("{}x{}", map.width, map.height);
                        let dim_response = ui.add(
                            egui::Button::new(
                                egui::RichText::new(&dim_text).size(13.0).color(colors.text),
                            )
                            .fill(egui::Color32::TRANSPARENT)
                            .stroke(egui::Stroke::NONE)
                            .corner_radius(0.0),
                        );
                        ui.label(
                            egui::RichText::new("Size").size(13.0).color(colors.muted),
                        );

                        let popup_width = dim_response.rect.width().max(120.0);
                        egui::Popup::from_toggle_button_response(&dim_response)
                            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                            .width(popup_width)
                            .show(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(0.0, 2.0);
                                let tile_count = map.tiles.len();
                                let dims = map::all_dimensions(tile_count);
                                egui::ScrollArea::vertical()
                                    .max_height(200.0)
                                    .show(ui, |ui| {
                                        for (w, h) in dims {
                                            let is_current =
                                                w == map.width && h == map.height;
                                            let label = format!("{}x{}", w, h);
                                            let text_color = if is_current {
                                                colors.accent
                                            } else {
                                                colors.text
                                            };
                                            let btn = ui.add(
                                                egui::Button::new(
                                                    egui::RichText::new(&label)
                                                        .size(13.0)
                                                        .color(text_color),
                                                )
                                                .fill(egui::Color32::TRANSPARENT)
                                                .stroke(egui::Stroke::NONE)
                                                .min_size(egui::vec2(
                                                    ui.available_width(),
                                                    24.0,
                                                ))
                                                .corner_radius(4.0),
                                            );
                                            if btn.clicked() {
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

                        // Zoom controls: [+] "Zoom N%" [-] (RTL order: + is rightmost)
                        if Self::zoom_button(ui, "+", &colors, "Zoom in (Cmd+Plus)") {
                            action = StatusBarAction::ZoomIn;
                        }

                        let zoom_text =
                            format!("Zoom {}%", (zoom * 100.0).round() as i32);
                        ui.label(
                            egui::RichText::new(zoom_text).size(13.0).color(colors.text),
                        );

                        if Self::zoom_button(ui, "-", &colors, "Zoom out (Cmd+Minus)") {
                            action = StatusBarAction::ZoomOut;
                        }

                        Self::separator(ui, &colors);
                    });
                });
            });

        action
    }

    fn zoom_button(ui: &mut egui::Ui, label: &str, colors: &ThemeColors, tooltip: &str) -> bool {
        let size = egui::vec2(20.0, 20.0);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

        let bg = if response.hovered() {
            colors.panel_2
        } else {
            egui::Color32::TRANSPARENT
        };
        let border = if response.hovered() {
            egui::Stroke::new(1.0, colors.border)
        } else {
            egui::Stroke::NONE
        };

        ui.painter()
            .rect(rect, 4.0, bg, border, egui::StrokeKind::Inside);

        let text_color = if response.hovered() {
            colors.text
        } else {
            colors.muted
        };
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(14.0),
            text_color,
        );

        response.clone().on_hover_text(tooltip);
        response.clicked()
    }

    fn separator(ui: &mut egui::Ui, colors: &ThemeColors) {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, 14.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 0.0, colors.border);
    }
}
