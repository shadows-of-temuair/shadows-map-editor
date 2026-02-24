use eframe::egui;

use crate::theme::{ThemeColors, theme_colors};

const INSPECTOR_MIN_WIDTH: f32 = 260.0;
const INSPECTOR_MAX_WIDTH: f32 = 364.0;

pub struct InspectorPanel;

impl InspectorPanel {
    pub fn show(ctx: &egui::Context, map: &map::Map, sotp: Option<&[u8]>) {
        let colors = theme_colors();

        egui::SidePanel::right("inspector")
            .width_range(INSPECTOR_MIN_WIDTH..=INSPECTOR_MAX_WIDTH)
            .resizable(true)
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
                Self::draw(ui, &colors, map, sotp);
            });
    }

    fn draw(
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        map: &map::Map,
        sotp: Option<&[u8]>,
    ) {
        // Draw left border
        let panel_rect = ui.max_rect();
        ui.painter().line_segment(
            [
                egui::pos2(panel_rect.left() - 14.0, panel_rect.top() - 10.0),
                egui::pos2(panel_rect.left() - 14.0, panel_rect.bottom() + 10.0),
            ],
            egui::Stroke::new(1.0, colors.border),
        );

        // --- Tileset section ---
        Self::section_header(ui, colors, "Tileset");
        ui.add_space(8.0);

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

        // Reserve space at bottom for tab map section.
        // Tab map height scales with panel width: (avail_w * 0.5) + ~50px overhead.
        let available = ui.available_size();
        let tab_map_reserved = (available.x * 0.5 + 50.0).max(160.0);
        let grid_height = (available.y - tab_map_reserved).max(80.0);
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(available.x, grid_height),
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

        // --- Tileset bottom border ---
        Self::full_width_border(ui, colors, panel_rect);

        // --- Tab Map section ---
        Self::section_header(ui, colors, "Tab Map");
        ui.add_space(6.0);

        Self::draw_tab_map(ui, colors, map, sotp);

        // --- Tab Map bottom border ---
        Self::full_width_border(ui, colors, panel_rect);
    }

    /// Section header: bold label.
    fn section_header(ui: &mut egui::Ui, colors: &ThemeColors, label: &str) {
        ui.label(
            egui::RichText::new(label)
                .size(14.0)
                .strong()
                .color(colors.text),
        );
    }

    /// Draw a full-width horizontal border spanning edge to edge.
    fn full_width_border(ui: &mut egui::Ui, colors: &ThemeColors, panel_rect: egui::Rect) {
        ui.add_space(10.0);
        let cursor_y = ui.cursor().top();
        let left = panel_rect.left() - 14.0;
        let right = panel_rect.right() + 14.0;
        ui.painter().line_segment(
            [egui::pos2(left, cursor_y), egui::pos2(right, cursor_y)],
            egui::Stroke::new(1.0, colors.border),
        );
        ui.add_space(10.0);
    }

    /// Render a minimap showing solid (impassable) tiles as filled isometric
    /// diamonds with wireframe edges on boundaries between solid and non-solid.
    fn draw_tab_map(
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        map: &map::Map,
        sotp: Option<&[u8]>,
    ) {
        let w = map.width as usize;
        let h = map.height as usize;

        if w == 0 || h == 0 {
            ui.label(egui::RichText::new("Empty map").size(13.0).color(colors.muted));
            return;
        }

        // Build solid grid from SOTP collision data
        let solid = Self::build_solid_grid(map, sotp);

        // Scale to fit available width — grows proportionally with the panel.
        let avail_w = ui.available_width();
        let total_iso_w = (w + h) as f32;
        let hw = (avail_w / total_iso_w).max(1.0);
        let hh = hw * 0.5;

        let total_w = total_iso_w * hw;
        let total_h = total_iso_w * hh;

        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(total_w, total_h),
            egui::Sense::hover(),
        );

        let origin_x = rect.left() + h as f32 * hw;
        let origin_y = rect.top();

        let painter = ui.painter();

        // White/light colors for tab map
        let solid_fill = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 18);
        let edge_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 120);
        let edge_stroke = egui::Stroke::new(1.0, edge_color);
        let _ = colors; // tab map uses white palette, not theme accent

        let is_solid = |col: i32, row: i32| -> bool {
            if col < 0 || col >= w as i32 || row < 0 || row >= h as i32 {
                return false;
            }
            solid[row as usize * w + col as usize]
        };

        for row in 0..h {
            for col in 0..w {
                let idx = row * w + col;
                if !solid[idx] {
                    continue;
                }

                let cx = origin_x + (col as f32 - row as f32) * hw;
                let cy = origin_y + (col as f32 + row as f32) * hh;

                let top = egui::pos2(cx, cy);
                let right = egui::pos2(cx + hw, cy + hh);
                let bottom = egui::pos2(cx, cy + 2.0 * hh);
                let left = egui::pos2(cx - hw, cy + hh);

                // Fill solid tile
                painter.add(egui::Shape::convex_polygon(
                    vec![top, right, bottom, left],
                    solid_fill,
                    egui::Stroke::NONE,
                ));

                // Draw edges where neighbor is non-solid (boundary wireframe)
                let c = col as i32;
                let r = row as i32;

                // Top-right edge: neighbor (col, row-1)
                if !is_solid(c, r - 1) {
                    painter.line_segment([top, right], edge_stroke);
                }
                // Bottom-right edge: neighbor (col+1, row)
                if !is_solid(c + 1, r) {
                    painter.line_segment([right, bottom], edge_stroke);
                }
                // Bottom-left edge: neighbor (col, row+1)
                if !is_solid(c, r + 1) {
                    painter.line_segment([bottom, left], edge_stroke);
                }
                // Top-left edge: neighbor (col-1, row)
                if !is_solid(c - 1, r) {
                    painter.line_segment([left, top], edge_stroke);
                }
            }
        }
    }

    /// Build a flat bool grid indicating whether each tile is solid (impassable).
    /// A tile is solid if `sotp[left_wall - 1] & 0xF == 0xF` or
    /// `sotp[right_wall - 1] & 0xF == 0xF`.
    fn build_solid_grid(map: &map::Map, sotp: Option<&[u8]>) -> Vec<bool> {
        let sotp = match sotp {
            Some(s) => s,
            None => return vec![false; map.tiles.len()],
        };

        map.tiles
            .iter()
            .map(|tile| {
                let left_solid = tile.left_wall > 0
                    && sotp
                        .get((tile.left_wall - 1) as usize)
                        .map(|&b| b & 0xF == 0xF)
                        .unwrap_or(false);
                let right_solid = tile.right_wall > 0
                    && sotp
                        .get((tile.right_wall - 1) as usize)
                        .map(|&b| b & 0xF == 0xF)
                        .unwrap_or(false);
                left_solid || right_solid
            })
            .collect()
    }
}
