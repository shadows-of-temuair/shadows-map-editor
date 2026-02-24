use eframe::egui;

use crate::theme::{ThemeColors, theme_colors};

const INSPECTOR_WIDTH: f32 = 260.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InspectorTab {
    Tileset,
    Properties,
}

pub struct InspectorPanel {
    pub active_tab: InspectorTab,
}

impl Default for InspectorPanel {
    fn default() -> Self {
        Self {
            active_tab: InspectorTab::Tileset,
        }
    }
}

/// Data for the currently selected tile, passed from the app.
pub struct SelectedTileInfo {
    pub col: u16,
    pub row: u16,
    pub ground: u16,
    pub left_wall: u16,
    pub right_wall: u16,
}

impl InspectorPanel {
    pub fn show(&mut self, ctx: &egui::Context, selection: Option<&SelectedTileInfo>) {
        let colors = theme_colors();

        egui::SidePanel::right("inspector")
            .exact_width(INSPECTOR_WIDTH)
            .resizable(false)
            .frame(
                egui::Frame::NONE
                    .fill(colors.bg_2)
                    .inner_margin(egui::Margin {
                        left: 14,
                        right: 14,
                        top: 10,
                        bottom: 10,
                    }),
            )
            .show(ctx, |ui| {
                self.draw(ui, &colors, selection);
            });
    }

    fn draw(&mut self, ui: &mut egui::Ui, colors: &ThemeColors, selection: Option<&SelectedTileInfo>) {
        // Draw left border
        let panel_rect = ui.max_rect();
        ui.painter().line_segment(
            [
                egui::pos2(panel_rect.left() - 14.0, panel_rect.top() - 10.0),
                egui::pos2(panel_rect.left() - 14.0, panel_rect.bottom() + 10.0),
            ],
            egui::Stroke::new(1.0, colors.border),
        );

        // Tab bar
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
            Self::tab_button(
                ui,
                "Tileset",
                &mut self.active_tab,
                InspectorTab::Tileset,
                colors,
            );
            Self::tab_button(
                ui,
                "Properties",
                &mut self.active_tab,
                InspectorTab::Properties,
                colors,
            );
        });

        ui.add_space(10.0);

        // Separator under tabs
        let sep_rect = ui.max_rect();
        ui.painter().line_segment(
            [
                egui::pos2(sep_rect.left(), ui.cursor().top()),
                egui::pos2(sep_rect.right(), ui.cursor().top()),
            ],
            egui::Stroke::new(1.0, colors.border),
        );
        ui.add_space(10.0);

        match self.active_tab {
            InspectorTab::Tileset => self.draw_tileset_tab(ui, colors),
            InspectorTab::Properties => self.draw_properties_tab(ui, colors, selection),
        }
    }

    fn tab_button(
        ui: &mut egui::Ui,
        label: &str,
        active: &mut InspectorTab,
        tab: InspectorTab,
        colors: &ThemeColors,
    ) {
        let is_active = *active == tab;
        let text_color = if is_active {
            colors.accent
        } else {
            colors.muted
        };

        let response = ui.add(
            egui::Button::new(egui::RichText::new(label).size(14.0).color(text_color))
                .fill(egui::Color32::TRANSPARENT)
                .stroke(egui::Stroke::NONE)
                .corner_radius(4.0),
        );

        if is_active {
            let rect = response.rect;
            ui.painter().line_segment(
                [
                    egui::pos2(rect.left() + 4.0, rect.bottom()),
                    egui::pos2(rect.right() - 4.0, rect.bottom()),
                ],
                egui::Stroke::new(2.0, colors.accent),
            );
        }

        if response.clicked() {
            *active = tab;
        }
    }

    fn draw_tileset_tab(&self, ui: &mut egui::Ui, colors: &ThemeColors) {
        // Category selector
        ui.label(
            egui::RichText::new("Category")
                .size(13.0)
                .color(colors.muted),
        );
        ui.add_space(4.0);

        let categories = ["All", "Terrain", "Interior", "Exterior"];
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
            for cat in &categories {
                let selected = *cat == "All";
                let bg = if selected {
                    colors.accent.gamma_multiply(0.15)
                } else {
                    colors.panel_2
                };
                let text_color = if selected {
                    colors.accent
                } else {
                    colors.muted
                };
                ui.add(
                    egui::Button::new(egui::RichText::new(*cat).size(13.0).color(text_color))
                        .fill(bg)
                        .stroke(if selected {
                            egui::Stroke::new(1.0, colors.accent.gamma_multiply(0.4))
                        } else {
                            egui::Stroke::NONE
                        })
                        .corner_radius(4.0),
                );
            }
        });

        ui.add_space(12.0);

        // Tileset grid placeholder
        ui.label(egui::RichText::new("Tiles").size(13.0).color(colors.muted));
        ui.add_space(4.0);

        let available = ui.available_size();
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(available.x, available.y.min(300.0)),
            egui::Sense::hover(),
        );

        // Draw placeholder grid for tile slots
        let tile_size = 48.0;
        let padding = 4.0;
        let cols = ((rect.width() + padding) / (tile_size + padding)).floor() as usize;
        let rows = ((rect.height() + padding) / (tile_size + padding)).floor() as usize;

        for row in 0..rows {
            for col in 0..cols {
                let x = rect.left() + col as f32 * (tile_size + padding);
                let y = rect.top() + row as f32 * (tile_size + padding);
                let tile_rect =
                    egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(tile_size, tile_size));

                if tile_rect.right() <= rect.right() + 1.0
                    && tile_rect.bottom() <= rect.bottom() + 1.0
                {
                    ui.painter().rect(
                        tile_rect,
                        3.0,
                        colors.bg.gamma_multiply(0.8),
                        egui::Stroke::new(1.0, colors.border.gamma_multiply(0.5)),
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }
    }

    fn draw_properties_tab(
        &self,
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        selection: Option<&SelectedTileInfo>,
    ) {
        ui.label(
            egui::RichText::new("Selection")
                .size(13.0)
                .color(colors.muted),
        );
        ui.add_space(6.0);

        match selection {
            Some(info) => {
                let props = [
                    ("Position", format!("{}, {}", info.col, info.row)),
                    ("Ground", format!("{}", info.ground)),
                    ("Left Wall", format!("{}", info.left_wall)),
                    ("Right Wall", format!("{}", info.right_wall)),
                ];

                for (label, value) in &props {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(*label).size(14.0).color(colors.muted),
                        );
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label(
                                    egui::RichText::new(value).size(14.0).color(colors.text),
                                );
                            },
                        );
                    });
                    ui.add_space(2.0);
                }
            }
            None => {
                ui.label(
                    egui::RichText::new("Click a tile to inspect")
                        .size(13.0)
                        .color(colors.muted),
                );
            }
        }
    }
}
