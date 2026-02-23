use eframe::egui;

use crate::theme::{ThemeColors, draw_grid_icon, theme_colors};
use crate::widgets::{TitleBarIcon, title_bar_icon_button};

const TITLE_BAR_HEIGHT: f32 = 36.0;

pub struct TitleBarPanel;

impl TitleBarPanel {
    pub fn show(ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let colors = theme_colors();

        egui::TopBottomPanel::top("title_bar")
            .exact_height(TITLE_BAR_HEIGHT)
            .frame(
                egui::Frame::NONE
                    .fill(colors.bg_2)
                    .inner_margin(egui::Margin::ZERO),
            )
            .show(ctx, |ui| {
                Self::draw(ui, &colors);
            });
    }

    fn draw(ui: &mut egui::Ui, colors: &ThemeColors) {
        let rect = ui.max_rect();

        // Bottom border (paint before layout so it doesn't conflict)
        ui.painter().line_segment(
            [
                egui::pos2(rect.left(), rect.bottom()),
                egui::pos2(rect.right(), rect.bottom()),
            ],
            egui::Stroke::new(1.0, colors.border),
        );

        // Icon + title text
        let icon_size = 22.0;
        let icon_left = rect.left() + 12.0;
        let icon_rect = egui::Rect::from_center_size(
            egui::pos2(icon_left + icon_size / 2.0, rect.center().y),
            egui::vec2(icon_size, icon_size),
        );
        draw_grid_icon(ui.painter(), icon_rect, colors);

        let text_left = icon_rect.right() + 12.0;
        ui.painter().text(
            egui::pos2(text_left, rect.center().y),
            egui::Align2::LEFT_CENTER,
            "Shadows Map Editor",
            egui::FontId::proportional(14.0),
            colors.muted,
        );

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

            // Draggable title area
            let title_rect = egui::Rect::from_min_size(
                rect.min,
                egui::vec2(rect.width() - 46.0 * 3.0, TITLE_BAR_HEIGHT),
            );
            let title_response = ui.interact(
                title_rect,
                ui.id().with("title_bar_drag"),
                egui::Sense::click_and_drag(),
            );
            if title_response.dragged() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if title_response.double_clicked() {
                let is_fullscreen = ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fullscreen));
            }

            // Window control buttons (right-aligned)
            let buttons_x = rect.right() - 46.0 * 3.0;
            ui.allocate_space(egui::vec2(buttons_x - rect.left(), 0.0));

            if title_bar_icon_button(ui, TitleBarIcon::Minimize, TITLE_BAR_HEIGHT, colors).clicked()
            {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
            if title_bar_icon_button(ui, TitleBarIcon::Maximize, TITLE_BAR_HEIGHT, colors).clicked()
            {
                let is_fullscreen = ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fullscreen));
            }
            if title_bar_icon_button(ui, TitleBarIcon::Close, TITLE_BAR_HEIGHT, colors).clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }
}
