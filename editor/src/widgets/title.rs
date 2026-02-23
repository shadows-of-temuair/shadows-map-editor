use eframe::egui;

use crate::theme::ThemeColors;

pub const TITLE_BAR_BUTTON_WIDTH: f32 = 46.0;

pub enum TitleBarIcon {
    Minimize,
    Maximize,
    Close,
}

pub fn title_bar_icon_button(
    ui: &mut egui::Ui,
    icon: TitleBarIcon,
    bar_height: f32,
    colors: &ThemeColors,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(TITLE_BAR_BUTTON_WIDTH, bar_height),
        egui::Sense::click(),
    );
    let painter = ui.painter();

    if response.hovered() {
        let hover_bg = match icon {
            TitleBarIcon::Close => egui::Color32::from_rgb(196, 43, 28),
            _ => colors.panel_2,
        };
        painter.rect_filled(rect, 0.0, hover_bg);
    }
    if response.is_pointer_button_down_on() {
        painter.rect_filled(rect, 0.0, colors.accent);
    }

    let idle_icon = egui::Color32::from_rgb(186, 190, 196);
    let hover_icon = egui::Color32::from_rgb(245, 245, 245);
    let pressed_icon = egui::Color32::from_rgb(10, 11, 13);
    let icon_color = if response.is_pointer_button_down_on() {
        pressed_icon
    } else if response.hovered() {
        hover_icon
    } else {
        idle_icon
    };

    let stroke = egui::Stroke::new(1.5, icon_color);
    let center = rect.center();
    let size = 5.0;

    match icon {
        TitleBarIcon::Minimize => {
            painter.line_segment(
                [
                    egui::pos2(center.x - size, center.y),
                    egui::pos2(center.x + size, center.y),
                ],
                stroke,
            );
        }
        TitleBarIcon::Maximize => {
            let r = egui::Rect::from_center_size(center, egui::vec2(size * 2.0, size * 2.0));
            painter.rect_stroke(r, 0.0, stroke, egui::StrokeKind::Inside);
        }
        TitleBarIcon::Close => {
            painter.line_segment(
                [
                    egui::pos2(center.x - size, center.y - size),
                    egui::pos2(center.x + size, center.y + size),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(center.x - size, center.y + size),
                    egui::pos2(center.x + size, center.y - size),
                ],
                stroke,
            );
        }
    }

    response
}
