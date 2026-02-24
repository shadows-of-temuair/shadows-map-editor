use eframe::egui;

use crate::document::MapDocument;
use crate::theme::{ThemeColors, theme_colors};

const TAB_BAR_HEIGHT: f32 = 32.0;
const NAV_BUTTON_WIDTH: f32 = 24.0;

pub enum TabBarAction {
    None,
    CloseTab(usize),
    SwitchTab(usize),
}

pub struct TabBarPanel {
    scroll_offset: f32,
}

impl Default for TabBarPanel {
    fn default() -> Self {
        Self {
            scroll_offset: 0.0,
        }
    }
}

impl TabBarPanel {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        documents: &[MapDocument],
        active_tab: usize,
    ) -> TabBarAction {
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

                    let available_width = ui.available_width();

                    // Measure total tab width
                    let total_tabs_width: f32 = documents
                        .iter()
                        .enumerate()
                        .map(|(i, doc)| Self::measure_tab_width(ui, doc, i == active_tab))
                        .sum();

                    let needs_scroll = total_tabs_width > available_width;
                    let tabs_area_width = if needs_scroll {
                        available_width - NAV_BUTTON_WIDTH * 2.0
                    } else {
                        available_width
                    };

                    // Left scroll button
                    if needs_scroll {
                        let can_scroll_left = self.scroll_offset > 0.0;
                        if Self::nav_button(ui, "<", can_scroll_left, &colors) {
                            self.scroll_offset = (self.scroll_offset - 120.0).max(0.0);
                        }
                    }

                    // Clipped tab area
                    let (clip_rect, _) = ui.allocate_exact_size(
                        egui::vec2(tabs_area_width, TAB_BAR_HEIGHT),
                        egui::Sense::hover(),
                    );

                    // Render tabs in a clipped child UI
                    let inner_width = total_tabs_width.max(tabs_area_width);
                    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(
                        egui::Rect::from_min_size(
                            egui::pos2(clip_rect.left() - self.scroll_offset, clip_rect.top()),
                            egui::vec2(inner_width, TAB_BAR_HEIGHT),
                        ),
                    ));
                    child_ui.set_clip_rect(clip_rect);
                    child_ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                    child_ui.horizontal_centered(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                        for (i, doc) in documents.iter().enumerate() {
                            let is_active = i == active_tab;
                            let tab_action =
                                Self::tab_button(ui, doc, i, is_active, &colors);
                            if !matches!(tab_action, TabBarAction::None) {
                                action = tab_action;
                            }
                        }
                    });

                    // Right scroll button
                    if needs_scroll {
                        let max_scroll = (total_tabs_width - tabs_area_width).max(0.0);
                        self.scroll_offset = self.scroll_offset.min(max_scroll);
                        let can_scroll_right = self.scroll_offset < max_scroll;
                        if Self::nav_button(ui, ">", can_scroll_right, &colors) {
                            let max_s = (total_tabs_width - tabs_area_width).max(0.0);
                            self.scroll_offset =
                                (self.scroll_offset + 120.0).min(max_s);
                        }
                    }

                });
            });

        action
    }

    /// Ensure the active tab is within the visible scroll window.
    pub fn ensure_tab_visible(&mut self, documents: &[MapDocument], active_tab: usize) {
        // We need a rough measure of tab widths without a UI context.
        // Use the same estimation formula as measure_tab_width but with a fixed
        // approximate character width since we don't have a painter here.
        let mut offset = 0.0f32;
        for (i, doc) in documents.iter().enumerate() {
            let w = Self::estimate_tab_width(doc, i == active_tab);
            if i == active_tab {
                // Just ensure scroll_offset isn't past this tab
                if offset < self.scroll_offset {
                    self.scroll_offset = offset;
                }
                // We don't know the exact tabs_area_width here, so use a reasonable default
                break;
            }
            offset += w;
        }
    }

    /// Measure the width a tab would occupy.
    fn measure_tab_width(ui: &egui::Ui, doc: &MapDocument, is_active: bool) -> f32 {
        let name = doc.display_name();
        let label = if doc.dirty {
            format!("{name} *")
        } else {
            name
        };

        let text_color = if is_active {
            egui::Color32::WHITE
        } else {
            egui::Color32::GRAY
        };

        let font = egui::FontId::proportional(13.0);
        let close_width: f32 = 20.0;
        let h_pad: f32 = 12.0;
        let galley = ui.painter().layout_no_wrap(label, font, text_color);
        let text_width = galley.size().x;
        h_pad + text_width + 6.0 + close_width + 6.0
    }

    /// Rough tab width estimate without painter access (for ensure_tab_visible).
    fn estimate_tab_width(doc: &MapDocument, _is_active: bool) -> f32 {
        let name = doc.display_name();
        let chars = if doc.dirty { name.len() + 2 } else { name.len() };
        let close_width: f32 = 20.0;
        let h_pad: f32 = 12.0;
        let approx_char_width = 7.5;
        h_pad + chars as f32 * approx_char_width + 6.0 + close_width + 6.0
    }

    fn nav_button(
        ui: &mut egui::Ui,
        label: &str,
        enabled: bool,
        colors: &ThemeColors,
    ) -> bool {
        let size = egui::vec2(NAV_BUTTON_WIDTH, TAB_BAR_HEIGHT);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
        let painter = ui.painter();

        let bg = if response.hovered() && enabled {
            colors.bg_3
        } else {
            egui::Color32::TRANSPARENT
        };
        painter.rect_filled(rect, 0.0, bg);

        let text_color = if enabled {
            if response.hovered() {
                colors.text
            } else {
                colors.muted
            }
        } else {
            colors.border
        };
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(14.0),
            text_color,
        );

        enabled && response.clicked()
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
