use eframe::egui;

use crate::theme::{theme_colors, ThemeColors};
use crate::widgets::icons;

const TOOLBAR_WIDTH: f32 = 48.0;
const TOOL_BUTTON_SIZE: f32 = 34.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToolbarAction {
    None,
    NewFile,
    OpenFile,
    SaveFile,
    Undo,
    Redo,
    Export,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Pencil,
    Eraser,
    Fill,
    Eyedropper,
    Rectangle,
}

impl Tool {
    pub fn tooltip(self) -> &'static str {
        match self {
            Tool::Select => "Select (V)",
            Tool::Pencil => "Pencil (B)",
            Tool::Eraser => "Eraser (E)",
            Tool::Fill => "Fill (G)",
            Tool::Eyedropper => "Eyedropper (I)",
            Tool::Rectangle => "Rectangle (R)",
        }
    }

    pub fn draw_icon(self, painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
        match self {
            Tool::Select => icons::draw_icon_select(painter, rect, color),
            Tool::Pencil => icons::draw_icon_pencil(painter, rect, color),
            Tool::Eraser => icons::draw_icon_eraser(painter, rect, color),
            Tool::Fill => icons::draw_icon_fill(painter, rect, color),
            Tool::Eyedropper => icons::draw_icon_eyedropper(painter, rect, color),
            Tool::Rectangle => icons::draw_icon_rectangle(painter, rect, color),
        }
    }
}

pub struct ToolbarPanel;

impl ToolbarPanel {
    pub fn show(
        ctx: &egui::Context,
        active_tool: &mut Tool,
        can_undo: bool,
        can_redo: bool,
    ) -> ToolbarAction {
        let colors = theme_colors();
        let mut action = ToolbarAction::None;

        egui::SidePanel::left("toolbar")
            .exact_width(TOOLBAR_WIDTH)
            .resizable(false)
            .frame(
                egui::Frame::NONE
                    .fill(colors.bg_2)
                    .inner_margin(egui::Margin {
                        left: 7,
                        right: 7,
                        top: 8,
                        bottom: 8,
                    }),
            )
            .show(ctx, |ui| {
                Self::draw(ui, active_tool, can_undo, can_redo, &colors, &mut action);
            });

        action
    }

    fn draw(
        ui: &mut egui::Ui,
        active_tool: &mut Tool,
        can_undo: bool,
        can_redo: bool,
        colors: &ThemeColors,
        action: &mut ToolbarAction,
    ) {
        // Draw right border
        let panel_rect = ui.max_rect();
        ui.painter().line_segment(
            [
                egui::pos2(panel_rect.right() + 7.0, panel_rect.top() - 8.0),
                egui::pos2(panel_rect.right() + 7.0, panel_rect.bottom() + 8.0),
            ],
            egui::Stroke::new(1.0, colors.border),
        );

        ui.vertical_centered(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 4.0);

            // --- File operation buttons ---
            let file_ops: &[(
                fn(&egui::Painter, egui::Rect, egui::Color32),
                &str,
                ToolbarAction,
            )] = &[
                (icons::draw_icon_new, "New (Cmd+N)", ToolbarAction::NewFile),
                (
                    icons::draw_icon_open,
                    "Open (Cmd+O)",
                    ToolbarAction::OpenFile,
                ),
                (
                    icons::draw_icon_save,
                    "Save (Cmd+S)",
                    ToolbarAction::SaveFile,
                ),
            ];

            for &(draw_fn, tooltip, toolbar_action) in file_ops {
                let response = Self::file_icon_button(ui, draw_fn, colors);
                if response.clicked() {
                    *action = toolbar_action;
                }
                response.on_hover_text(tooltip);
            }

            let export_response = Self::file_icon_button(ui, icons::draw_icon_export, colors);
            if export_response.clicked() {
                *action = ToolbarAction::Export;
            }
            export_response.on_hover_text("Export as PNG (Cmd+E)");

            // --- Horizontal divider ---
            ui.add_space(2.0);
            let (divider_rect, _) =
                ui.allocate_exact_size(egui::vec2(TOOL_BUTTON_SIZE, 1.0), egui::Sense::hover());
            ui.painter().rect_filled(divider_rect, 0.0, colors.border);
            ui.add_space(2.0);

            // --- Drawing tools ---
            for &tool in &[Tool::Select, Tool::Pencil] {
                let is_active = *active_tool == tool;
                let response = Self::tool_button(ui, tool, is_active, colors);
                if response.clicked() {
                    *active_tool = tool;
                }
            }
            for &tool in &[Tool::Eraser, Tool::Fill, Tool::Eyedropper, Tool::Rectangle] {
                Self::disabled_tool_button(ui, tool, colors);
            }

            // --- Divider ---
            ui.add_space(2.0);
            let (divider_rect2, _) =
                ui.allocate_exact_size(egui::vec2(TOOL_BUTTON_SIZE, 1.0), egui::Sense::hover());
            ui.painter().rect_filled(divider_rect2, 0.0, colors.border);
            ui.add_space(2.0);

            // --- Undo / Redo ---
            let undo_response = Self::icon_button(ui, icons::draw_icon_undo, colors, can_undo);
            if can_undo && undo_response.clicked() {
                *action = ToolbarAction::Undo;
            }
            undo_response.on_hover_text("Undo (Cmd+Z)");

            let redo_response = Self::icon_button(ui, icons::draw_icon_redo, colors, can_redo);
            if can_redo && redo_response.clicked() {
                *action = ToolbarAction::Redo;
            }
            redo_response.on_hover_text("Redo (Cmd+Shift+Z)");
        });
    }

    fn disabled_tool_button(ui: &mut egui::Ui, tool: Tool, colors: &ThemeColors) {
        let size = egui::vec2(TOOL_BUTTON_SIZE, TOOL_BUTTON_SIZE);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
        tool.draw_icon(ui.painter(), rect, colors.border);
        response.on_hover_text(format!("{} (coming soon)", tool.tooltip()));
    }

    fn file_icon_button(
        ui: &mut egui::Ui,
        draw_fn: fn(&egui::Painter, egui::Rect, egui::Color32),
        colors: &ThemeColors,
    ) -> egui::Response {
        Self::icon_button(ui, draw_fn, colors, true)
    }

    fn icon_button(
        ui: &mut egui::Ui,
        draw_fn: fn(&egui::Painter, egui::Rect, egui::Color32),
        colors: &ThemeColors,
        enabled: bool,
    ) -> egui::Response {
        let size = egui::vec2(TOOL_BUTTON_SIZE, TOOL_BUTTON_SIZE);
        let sense = if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(size, sense);
        let painter = ui.painter();

        let bg = if enabled && response.hovered() {
            colors.panel_2
        } else {
            egui::Color32::TRANSPARENT
        };

        let border = if enabled && response.hovered() {
            egui::Stroke::new(1.0, colors.border)
        } else {
            egui::Stroke::NONE
        };

        painter.rect(rect, 4.0, bg, border, egui::StrokeKind::Inside);

        let icon_color = if enabled && response.hovered() {
            colors.text
        } else if enabled {
            colors.muted
        } else {
            colors.border
        };

        draw_fn(painter, rect, icon_color);

        response
    }

    fn tool_button(
        ui: &mut egui::Ui,
        tool: Tool,
        is_active: bool,
        colors: &ThemeColors,
    ) -> egui::Response {
        let size = egui::vec2(TOOL_BUTTON_SIZE, TOOL_BUTTON_SIZE);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
        let painter = ui.painter();

        let bg = if is_active {
            colors.accent.gamma_multiply(0.2)
        } else if response.hovered() {
            colors.panel_2
        } else {
            egui::Color32::TRANSPARENT
        };

        let border = if is_active {
            egui::Stroke::new(1.0, colors.accent)
        } else if response.hovered() {
            egui::Stroke::new(1.0, colors.border)
        } else {
            egui::Stroke::NONE
        };

        painter.rect(rect, 4.0, bg, border, egui::StrokeKind::Inside);

        let icon_color = if is_active {
            colors.accent
        } else if response.hovered() {
            colors.text
        } else {
            colors.muted
        };

        tool.draw_icon(painter, rect, icon_color);

        response.on_hover_text(tool.tooltip())
    }
}
