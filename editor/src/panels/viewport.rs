use eframe::egui;

use crate::map::{self, Map};
use crate::theme::theme_colors;

pub struct ViewportPanel;

impl ViewportPanel {
    pub fn show(ctx: &egui::Context, map: &Map) {
        let colors = theme_colors();

        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(colors.bg)
                    .inner_margin(egui::Margin::ZERO),
            )
            .show(ctx, |ui| {
                Self::draw(ui, map, &colors);
            });
    }

    fn draw(ui: &mut egui::Ui, map: &Map, colors: &crate::theme::ThemeColors) {
        let rect = ui.max_rect();
        let painter = ui.painter_at(rect);

        // Subtle grid dot pattern on the background
        let grid_spacing = 24.0;
        let dot_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 8);
        let mut x = rect.left() + grid_spacing;
        while x < rect.right() {
            let mut y = rect.top() + grid_spacing;
            while y < rect.bottom() {
                painter.circle_filled(egui::pos2(x, y), 0.5, dot_color);
                y += grid_spacing;
            }
            x += grid_spacing;
        }

        // Draw the isometric grid preview centered in the viewport
        let center = rect.center();
        let tw = map::TILE_WIDTH;
        let th = map::TILE_HEIGHT;
        let half_w = tw / 2.0;
        let half_h = th / 2.0;

        let grid_w = map.width as f32;
        let grid_h = map.height as f32;

        // Origin: top-center of the isometric diamond
        let origin_x = center.x;
        let origin_y = center.y - (grid_w + grid_h) * half_h / 2.0;

        let grid_line = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 18);
        let grid_stroke = egui::Stroke::new(0.5, grid_line);

        // Draw horizontal grid lines (along x axis in iso space)
        for row in 0..=map.height {
            let r = row as f32;
            let start_x = origin_x - r * half_w;
            let start_y = origin_y + r * half_h;
            let end_x = start_x + grid_w * half_w;
            let end_y = start_y + grid_w * half_h;
            painter.line_segment(
                [egui::pos2(start_x, start_y), egui::pos2(end_x, end_y)],
                grid_stroke,
            );
        }

        // Draw vertical grid lines (along y axis in iso space)
        for col in 0..=map.width {
            let c = col as f32;
            let start_x = origin_x + c * half_w;
            let start_y = origin_y + c * half_h;
            let end_x = start_x - grid_h * half_w;
            let end_y = start_y + grid_h * half_h;
            painter.line_segment(
                [egui::pos2(start_x, start_y), egui::pos2(end_x, end_y)],
                grid_stroke,
            );
        }

        // Label the grid center
        painter.text(
            egui::pos2(center.x, origin_y - 14.0),
            egui::Align2::CENTER_BOTTOM,
            format!("{}x{}", map.width, map.height),
            egui::FontId::proportional(11.0),
            colors.muted,
        );
    }
}
