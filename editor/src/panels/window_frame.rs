use eframe::egui;

use crate::theme::theme_colors;

const RESIZE_BORDER: f32 = 5.0;

pub struct WindowFrame;

impl WindowFrame {
    /// Handle window edge resize interaction and draw a 1px border.
    pub fn show(ctx: &egui::Context) {
        let is_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
        if is_fullscreen {
            return;
        }

        Self::handle_resize(ctx);
        Self::draw_border(ctx);
    }

    fn handle_resize(ctx: &egui::Context) {
        let rect = ctx.viewport_rect();
        if !rect.is_finite() {
            return;
        }
        let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) else {
            return;
        };

        let left = pos.x - rect.left() < RESIZE_BORDER;
        let right = rect.right() - pos.x < RESIZE_BORDER;
        let top = pos.y - rect.top() < RESIZE_BORDER;
        let bottom = rect.bottom() - pos.y < RESIZE_BORDER;

        let direction = match (left, right, top, bottom) {
            (true, _, true, _) => Some(egui::ResizeDirection::NorthWest),
            (true, _, _, true) => Some(egui::ResizeDirection::SouthWest),
            (_, true, true, _) => Some(egui::ResizeDirection::NorthEast),
            (_, true, _, true) => Some(egui::ResizeDirection::SouthEast),
            (true, _, _, _) => Some(egui::ResizeDirection::West),
            (_, true, _, _) => Some(egui::ResizeDirection::East),
            (_, _, true, _) => Some(egui::ResizeDirection::North),
            (_, _, _, true) => Some(egui::ResizeDirection::South),
            _ => None,
        };

        if let Some(dir) = direction {
            let cursor = match dir {
                egui::ResizeDirection::North | egui::ResizeDirection::South => {
                    egui::CursorIcon::ResizeVertical
                }
                egui::ResizeDirection::East | egui::ResizeDirection::West => {
                    egui::CursorIcon::ResizeHorizontal
                }
                egui::ResizeDirection::NorthWest | egui::ResizeDirection::SouthEast => {
                    egui::CursorIcon::ResizeNwSe
                }
                egui::ResizeDirection::NorthEast | egui::ResizeDirection::SouthWest => {
                    egui::CursorIcon::ResizeNeSw
                }
            };
            ctx.set_cursor_icon(cursor);

            if ctx.input(|i| i.pointer.any_pressed()) {
                ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(dir));
            }
        }
    }

    fn draw_border(ctx: &egui::Context) {
        let colors = theme_colors();
        let rect = ctx.viewport_rect();
        if !rect.is_finite() {
            return;
        }
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("window_border"),
        ));
        painter.rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(1.0, colors.border),
            egui::StrokeKind::Inside,
        );
    }
}
