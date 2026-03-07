use eframe::egui;

use crate::prefab;
use crate::theme::theme_colors;
use crate::widgets::icons;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MapSizeDialogMode {
    #[default]
    Standard,
    PrefabCanvas,
}

#[derive(Default)]
pub struct MapSizeDialog {
    open: bool,
    width_input: String,
    height_input: String,
    error: Option<String>,
    focus_width_on_open: bool,
}

impl MapSizeDialog {
    pub fn open(&mut self, width: u16, height: u16) {
        self.open = true;
        self.width_input = width.to_string();
        self.height_input = height.to_string();
        self.error = None;
        self.focus_width_on_open = true;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        id: &'static str,
        title: &str,
        confirm_label: &str,
        current_map: Option<&map::Map>,
        mode: MapSizeDialogMode,
    ) -> Option<(u16, u16)> {
        if !self.open {
            return None;
        }

        let viewport = ctx.viewport_rect();
        let screen = ctx.content_rect();
        if !viewport.is_finite() || !screen.is_finite() {
            return None;
        }

        let colors = theme_colors();
        let mut open = self.open;
        let mut close_window = false;
        let mut submitted_size = None;

        // Dim the background and allow click-away dismissal.
        egui::Area::new(egui::Id::new((id, "backdrop")))
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let response = ui.allocate_response(screen.size(), egui::Sense::click());
                ui.painter().rect_filled(
                    screen,
                    0.0,
                    egui::Color32::from_rgba_unmultiplied(0, 0, 0, 140),
                );
                if response.clicked() {
                    close_window = true;
                    self.error = None;
                }
            });

        egui::Window::new("")
            .id(egui::Id::new(id))
            .title_bar(false)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .fixed_size(egui::vec2(360.0, 0.0))
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .frame(
                egui::Frame::NONE
                    .fill(colors.bg_2)
                    .stroke(egui::Stroke::new(1.0, colors.border))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::same(24)),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

                draw_modal_title(ui, &colors, title_icon(title), title);
                ui.add_space(4.0);

                if let Some(map) = current_map {
                    ui.label(
                        egui::RichText::new(format!(
                            "Current: {}x{} ({} tiles)",
                            map.width,
                            map.height,
                            map.tiles.len()
                        ))
                        .size(12.0)
                        .color(colors.muted),
                    );
                    ui.add_space(4.0);
                }

                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    Self::centered_row_label(ui, "Width", colors.muted);
                    let width_response = ui.add_sized(
                        egui::vec2(72.0, 24.0),
                        egui::TextEdit::singleline(&mut self.width_input)
                            .font(egui::FontId::proportional(14.0))
                            .margin(egui::Margin::symmetric(8, 6)),
                    );
                    if self.focus_width_on_open {
                        width_response.request_focus();
                        self.focus_width_on_open = false;
                    }
                    Self::centered_row_label(ui, "x", colors.muted);
                    Self::centered_row_label(ui, "Height", colors.muted);
                    ui.add_sized(
                        egui::vec2(72.0, 24.0),
                        egui::TextEdit::singleline(&mut self.height_input)
                            .font(egui::FontId::proportional(14.0))
                            .margin(egui::Margin::symmetric(8, 6)),
                    );
                });

                if let Some(map) = current_map {
                    let warn_text = match (
                        Self::parse_dim(&self.width_input),
                        Self::parse_dim(&self.height_input),
                    ) {
                        (Ok(width), Ok(height)) => Self::warning_text(mode, map, width, height),
                        _ => None,
                    };

                    if let Some(text) = warn_text {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(text)
                                .size(12.0)
                                .color(egui::Color32::from_rgb(232, 170, 72)),
                        );
                    }
                }

                if let Some(error) = &self.error {
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(error)
                            .size(12.0)
                            .color(egui::Color32::from_rgb(230, 92, 92)),
                    );
                }

                let submit = ui.input(|i| i.key_pressed(egui::Key::Enter));
                let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
                if escape {
                    close_window = true;
                    self.error = None;
                }

                ui.add_space(6.0);
                let (sep_rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 1.0),
                    egui::Sense::hover(),
                );
                ui.painter().rect_filled(sep_rect, 0.0, colors.border);

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);

                        // Swapped order: Cancel is on the right, Create/Apply is on the left.
                        let cancel_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("Cancel").size(14.0).color(colors.text),
                            )
                            .fill(colors.bg_3)
                            .stroke(egui::Stroke::new(1.0, colors.border))
                            .corner_radius(4.0)
                            .min_size(egui::vec2(80.0, 32.0)),
                        );
                        if cancel_btn.clicked() {
                            close_window = true;
                            self.error = None;
                        }

                        let apply_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new(confirm_label)
                                    .size(14.0)
                                    .color(egui::Color32::from_rgb(10, 11, 13)),
                            )
                            .fill(colors.accent)
                            .stroke(egui::Stroke::NONE)
                            .corner_radius(4.0)
                            .min_size(egui::vec2(88.0, 32.0)),
                        );
                        if apply_btn.clicked() || submit {
                            match (
                                Self::parse_dim(&self.width_input),
                                Self::parse_dim(&self.height_input),
                            ) {
                                (Ok(width), Ok(height)) => {
                                    submitted_size = Some((width, height));
                                    close_window = true;
                                    self.error = None;
                                }
                                (Err(width_err), Err(height_err)) => {
                                    self.error =
                                        Some(format!("Width: {width_err}. Height: {height_err}."));
                                }
                                (Err(width_err), _) => {
                                    self.error = Some(format!("Width: {width_err}."));
                                }
                                (_, Err(height_err)) => {
                                    self.error = Some(format!("Height: {height_err}."));
                                }
                            }
                        }
                    });
                });
            });

        if close_window {
            open = false;
        }
        self.open = open;
        if !self.open {
            self.focus_width_on_open = false;
        }
        submitted_size
    }

    fn parse_dim(input: &str) -> Result<u16, &'static str> {
        let trimmed = input.trim();
        let value: u32 = trimmed.parse().map_err(|_| "must be a number")?;
        if !(1..=u16::MAX as u32).contains(&value) {
            return Err("must be between 1 and 65535");
        }
        Ok(value as u16)
    }

    fn truncation_warning(map: &map::Map, width: u16, height: u16) -> Option<String> {
        let new_tiles = width as u64 * height as u64;
        let old_tiles = map.tiles.len() as u64;
        if new_tiles < old_tiles {
            Some(format!(
                "Warning: reducing to {} tiles will truncate {} tile(s) from the end.",
                new_tiles,
                old_tiles - new_tiles
            ))
        } else {
            None
        }
    }

    fn prefab_canvas_warning(map: &map::Map, width: u16, height: u16) -> Option<String> {
        let clipped_tiles = prefab::centered_canvas_tile_loss(map, width, height);
        (clipped_tiles > 0).then(|| {
            format!(
                "Warning: resizing the canvas to {}x{} will clip {} painted tile(s).",
                width, height, clipped_tiles
            )
        })
    }

    fn warning_text(
        mode: MapSizeDialogMode,
        map: &map::Map,
        width: u16,
        height: u16,
    ) -> Option<String> {
        match mode {
            MapSizeDialogMode::Standard => Self::truncation_warning(map, width, height),
            MapSizeDialogMode::PrefabCanvas => Self::prefab_canvas_warning(map, width, height),
        }
    }

    fn centered_row_label(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
        let width = match text {
            "Width" => 40.0,
            "Height" => 44.0,
            "x" => 10.0,
            _ => 40.0,
        };
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 24.0), egui::Sense::hover());
        // Optical correction: "Width" has no descenders, so pure geometric centering
        // looks slightly high compared with the input text and "Height".
        let y_offset = if text == "Width" { 1.0 } else { 0.0 };
        ui.painter().text(
            rect.center() + egui::vec2(0.0, y_offset),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(13.0),
            color,
        );
    }
}

fn title_icon(title: &str) -> &'static str {
    match title {
        "New Map" => "\u{EC01}",
        "New Prefab" => "\u{E5D8}",
        "Resize Canvas" => "\u{EE04}",
        "Map Size" => "\u{1F4CF}",
        "Resize" => "\u{1F4CF}",
        _ => "\u{1F4CF}",
    }
}

fn draw_modal_title(
    ui: &mut egui::Ui,
    colors: &crate::theme::ThemeColors,
    icon: &str,
    title: &str,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        ui.label(
            egui::RichText::new(icon)
                .font(icons::symbol_icon_font_id(18.0))
                .color(colors.text),
        );
        ui.label(
            egui::RichText::new(title)
                .size(18.0)
                .strong()
                .color(colors.text),
        );
    });
}

#[cfg(test)]
mod tests {
    use super::{MapSizeDialog, MapSizeDialogMode};

    #[test]
    fn truncation_warning_only_when_new_tile_count_is_smaller() {
        let map = map::Map::new(10, 10); // 100 tiles
        assert!(MapSizeDialog::truncation_warning(&map, 10, 10).is_none());
        assert!(MapSizeDialog::truncation_warning(&map, 12, 10).is_none());
        assert!(MapSizeDialog::truncation_warning(&map, 5, 5).is_some());
    }

    #[test]
    fn prefab_canvas_warning_only_when_painted_tiles_would_clip() {
        let mut map = map::Map::new(4, 4);
        map.tiles[0].ground = 7;
        map.tiles[1 * 4 + 1].left_wall = 9;
        map.tiles[2 * 4 + 2].right_wall = 11;

        assert!(MapSizeDialog::warning_text(MapSizeDialogMode::PrefabCanvas, &map, 6, 6).is_none());
        assert!(MapSizeDialog::warning_text(MapSizeDialogMode::PrefabCanvas, &map, 2, 2).is_some());
    }
}
