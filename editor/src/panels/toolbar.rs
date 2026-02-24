use eframe::egui;

use crate::theme::{ThemeColors, theme_colors};

const TOOLBAR_WIDTH: f32 = 48.0;
const TOOL_BUTTON_SIZE: f32 = 34.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToolbarAction {
    None,
    NewFile,
    OpenFile,
    SaveFile,
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
    pub const ALL: &[Tool] = &[
        Tool::Select,
        Tool::Pencil,
        Tool::Eraser,
        Tool::Fill,
        Tool::Eyedropper,
        Tool::Rectangle,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Tool::Select => "V",
            Tool::Pencil => "B",
            Tool::Eraser => "E",
            Tool::Fill => "G",
            Tool::Eyedropper => "I",
            Tool::Rectangle => "R",
        }
    }

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

    pub fn icon_char(self) -> &'static str {
        match self {
            Tool::Select => "\u{25E3}",
            Tool::Pencil => "\u{270E}",
            Tool::Eraser => "\u{2395}",
            Tool::Fill => "\u{25CF}",
            Tool::Eyedropper => "\u{25C9}",
            Tool::Rectangle => "\u{25A1}",
        }
    }
}

pub struct ToolbarPanel;

impl ToolbarPanel {
    pub fn show(ctx: &egui::Context, active_tool: &mut Tool) -> ToolbarAction {
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
                Self::draw(ui, active_tool, &colors, &mut action);
            });

        action
    }

    fn draw(
        ui: &mut egui::Ui,
        active_tool: &mut Tool,
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
            let file_ops: &[(&str, &str, ToolbarAction)] = &[
                ("\u{2B9A}", "New (Cmd+N)", ToolbarAction::NewFile),
                ("\u{21B3}", "Open (Cmd+O)", ToolbarAction::OpenFile),
                ("\u{2193}", "Save (Cmd+S)", ToolbarAction::SaveFile),
            ];

            for &(icon, tooltip, toolbar_action) in file_ops {
                let response = Self::file_button(ui, icon, colors);
                if response.clicked() {
                    *action = toolbar_action;
                }
                response.on_hover_text(tooltip);
            }

            // --- Horizontal divider ---
            ui.add_space(2.0);
            let (divider_rect, _) = ui.allocate_exact_size(
                egui::vec2(TOOL_BUTTON_SIZE, 1.0),
                egui::Sense::hover(),
            );
            ui.painter().rect_filled(divider_rect, 0.0, colors.border);
            ui.add_space(2.0);

            // --- Drawing tools ---
            for &tool in Tool::ALL {
                let is_active = *active_tool == tool;
                let response = Self::tool_button(ui, tool, is_active, colors);
                if response.clicked() {
                    *active_tool = tool;
                }
            }

            // --- Divider ---
            ui.add_space(2.0);
            let (divider_rect2, _) = ui.allocate_exact_size(
                egui::vec2(TOOL_BUTTON_SIZE, 1.0),
                egui::Sense::hover(),
            );
            ui.painter().rect_filled(divider_rect2, 0.0, colors.border);
            ui.add_space(2.0);

            // --- Undo / Redo (placeholder, disabled) ---
            Self::disabled_button(ui, "\u{21B6}", "Undo (Cmd+Z)", colors);
            Self::disabled_button(ui, "\u{21B7}", "Redo (Cmd+Shift+Z)", colors);
        });
    }

    fn disabled_button(ui: &mut egui::Ui, icon: &str, tooltip: &str, colors: &ThemeColors) {
        let size = egui::vec2(TOOL_BUTTON_SIZE, TOOL_BUTTON_SIZE);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
        let dim = colors.border;
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(16.0),
            dim,
        );
        response.on_hover_text(tooltip);
    }

    fn file_button(ui: &mut egui::Ui, icon: &str, colors: &ThemeColors) -> egui::Response {
        let size = egui::vec2(TOOL_BUTTON_SIZE, TOOL_BUTTON_SIZE);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
        let painter = ui.painter();

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

        painter.rect(rect, 4.0, bg, border, egui::StrokeKind::Inside);

        let text_color = if response.hovered() {
            colors.text
        } else {
            colors.muted
        };

        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(16.0),
            text_color,
        );

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

        let text_color = if is_active {
            colors.accent
        } else if response.hovered() {
            colors.text
        } else {
            colors.muted
        };

        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            tool.icon_char(),
            egui::FontId::proportional(16.0),
            text_color,
        );

        response.on_hover_text(tool.tooltip())
    }
}
