use eframe::egui;

use crate::map::Map;
use crate::panels::Tool;
use crate::theme::theme_colors;

const STATUS_BAR_HEIGHT: f32 = 28.0;

pub struct StatusBarPanel;

impl StatusBarPanel {
    pub fn show(ctx: &egui::Context, map: &Map, active_tool: Tool) {
        let colors = theme_colors();

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

                    // Tool indicator
                    ui.label(
                        egui::RichText::new(active_tool.tooltip())
                            .size(11.0)
                            .color(colors.muted),
                    );

                    Self::separator(ui, &colors);

                    // Map dimensions
                    ui.label(
                        egui::RichText::new(format!("{}x{}", map.width, map.height))
                            .size(11.0)
                            .color(colors.text),
                    );

                    Self::separator(ui, &colors);

                    // Cursor position placeholder
                    ui.label(
                        egui::RichText::new("Pos: -")
                            .size(11.0)
                            .color(colors.muted),
                    );

                    // Right-aligned status
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("Ready")
                                .size(11.0)
                                .color(colors.accent),
                        );
                    });
                });
            });
    }

    fn separator(ui: &mut egui::Ui, colors: &crate::theme::ThemeColors) {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, 14.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 0.0, colors.border);
    }
}
