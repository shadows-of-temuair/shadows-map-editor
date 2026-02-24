use eframe::egui;

use crate::document::MapDocument;
use crate::theme::{ThemeColors, theme_colors};

const TAB_BAR_HEIGHT: f32 = 32.0;

pub enum TabBarAction {
    None,
    NewTab,
    CloseTab(usize),
    SwitchTab(usize),
}

pub struct TabBarPanel;

impl TabBarPanel {
    pub fn show(ctx: &egui::Context, documents: &[MapDocument], active_tab: usize) -> TabBarAction {
        let colors = theme_colors();
        let mut action = TabBarAction::None;

        egui::TopBottomPanel::top("tab_bar")
            .exact_height(TAB_BAR_HEIGHT)
            .frame(
                egui::Frame::NONE
                    .fill(colors.bg_2)
                    .inner_margin(egui::Margin::ZERO),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                    for (i, doc) in documents.iter().enumerate() {
                        let is_active = i == active_tab;
                        let tab_action = Self::tab_button(ui, doc, i, is_active, &colors);
                        if !matches!(tab_action, TabBarAction::None) {
                            action = tab_action;
                        }
                    }

                    // + button (flat, square)
                    let (plus_rect, plus_response) = ui.allocate_exact_size(
                        egui::vec2(TAB_BAR_HEIGHT, TAB_BAR_HEIGHT),
                        egui::Sense::click(),
                    );
                    let plus_bg = if plus_response.hovered() {
                        colors.bg_3
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    ui.painter().rect_filled(plus_rect, 0.0, plus_bg);
                    ui.painter().text(
                        plus_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "+",
                        egui::FontId::proportional(14.0),
                        colors.muted,
                    );
                    if plus_response.clicked() {
                        action = TabBarAction::NewTab;
                    }
                    plus_response.on_hover_text("New map (Cmd+N)");
                });
            });

        action
    }

    fn tab_button(
        ui: &mut egui::Ui,
        doc: &MapDocument,
        index: usize,
        is_active: bool,
        colors: &ThemeColors,
    ) -> TabBarAction {
        let mut action = TabBarAction::None;

        let name = doc.display_name();
        let label = if doc.dirty {
            format!("{name} *")
        } else {
            name
        };

        let bg = if is_active {
            colors.bg // seamless blend with viewport
        } else {
            colors.bg_2 // recessed
        };
        let text_color = if is_active {
            colors.text
        } else {
            colors.muted
        };

        // Measure text to compute tab width
        let font = egui::FontId::proportional(13.0);
        let close_width: f32 = 20.0;
        let h_pad: f32 = 12.0;
        let galley = ui.painter().layout_no_wrap(label.clone(), font.clone(), text_color);
        let text_width = galley.size().x;
        let tab_width = h_pad + text_width + 6.0 + close_width + 6.0;

        // Allocate the tab rect
        let (tab_rect, tab_response) = ui.allocate_exact_size(
            egui::vec2(tab_width, TAB_BAR_HEIGHT),
            egui::Sense::click(),
        );

        let painter = ui.painter();

        // Background fill (no corner radius)
        painter.rect_filled(tab_rect, 0.0, bg);

        // Label text
        painter.text(
            egui::pos2(tab_rect.left() + h_pad, tab_rect.center().y),
            egui::Align2::LEFT_CENTER,
            &label,
            font,
            text_color,
        );

        // Close button (x) — sub-interaction within tab rect
        let close_rect = egui::Rect::from_center_size(
            egui::pos2(tab_rect.right() - 6.0 - close_width / 2.0, tab_rect.center().y),
            egui::vec2(close_width, close_width),
        );
        let close_response = ui.interact(
            close_rect,
            ui.id().with(("tab_close", index)),
            egui::Sense::click(),
        );

        let close_color = if close_response.hovered() {
            colors.text
        } else if is_active {
            colors.muted
        } else {
            colors.border
        };
        painter.text(
            close_rect.center(),
            egui::Align2::CENTER_CENTER,
            "\u{00D7}",
            egui::FontId::proportional(14.0),
            close_color,
        );

        // 2px accent underline on active tab (full width)
        if is_active {
            painter.line_segment(
                [
                    egui::pos2(tab_rect.left(), tab_rect.bottom() - 1.0),
                    egui::pos2(tab_rect.right(), tab_rect.bottom() - 1.0),
                ],
                egui::Stroke::new(2.0, colors.accent),
            );
        }

        // 1px vertical separator on right edge
        painter.line_segment(
            [
                egui::pos2(tab_rect.right(), tab_rect.top() + 6.0),
                egui::pos2(tab_rect.right(), tab_rect.bottom() - 6.0),
            ],
            egui::Stroke::new(1.0, colors.border),
        );

        // Close takes priority over tab switch
        if close_response.clicked() {
            action = TabBarAction::CloseTab(index);
        } else if tab_response.clicked() {
            action = TabBarAction::SwitchTab(index);
        }

        action
    }
}
