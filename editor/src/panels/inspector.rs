use eframe::egui;

use crate::theme::{ThemeColors, theme_colors};

const INSPECTOR_MIN_WIDTH: f32 = 260.0;
const INSPECTOR_MAX_WIDTH: f32 = 760.0;
const INSPECTOR_DEFAULT_WIDTH: f32 = 420.0;
const TILE_COLUMN_GAP: f32 = 4.0;
const TILE_ROW_GAP: f32 = 2.0;
const TILE_PREVIEW_HEIGHT: f32 = 26.0;

pub struct InspectorPanel;

impl InspectorPanel {
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        ctx: &egui::Context,
        map: &map::Map,
        sotp: Option<&[u8]>,
        tile_atlas: Option<&render::TileAtlas>,
        atlas_texture: Option<&egui::TextureHandle>,
        selected_ground_tile: &mut u16,
    ) {
        let colors = theme_colors();

        egui::SidePanel::right("inspector")
            .width_range(INSPECTOR_MIN_WIDTH..=INSPECTOR_MAX_WIDTH)
            .default_width(INSPECTOR_DEFAULT_WIDTH)
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
                Self::draw(
                    ui,
                    &colors,
                    map,
                    sotp,
                    tile_atlas,
                    atlas_texture,
                    selected_ground_tile,
                );
            });
    }

    #[allow(clippy::too_many_arguments)]
    fn draw(
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        map: &map::Map,
        sotp: Option<&[u8]>,
        tile_atlas: Option<&render::TileAtlas>,
        atlas_texture: Option<&egui::TextureHandle>,
        selected_ground_tile: &mut u16,
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
        Self::section_header(ui, colors, "Ground Tiles");
        ui.add_space(8.0);
        Self::draw_tileset(ui, colors, tile_atlas, atlas_texture, selected_ground_tile);

        // --- Tileset bottom border ---
        Self::full_width_border(ui, colors, panel_rect);

        // --- Tab Map section ---
        Self::section_header(ui, colors, "Tab Map");
        ui.add_space(6.0);
        Self::draw_tab_map(ui, colors, map, sotp);

        // --- Tab Map bottom border ---
        Self::full_width_border(ui, colors, panel_rect);
    }

    fn draw_tileset(
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        tile_atlas: Option<&render::TileAtlas>,
        atlas_texture: Option<&egui::TextureHandle>,
        selected_ground_tile: &mut u16,
    ) {
        let (atlas, texture) = match (tile_atlas, atlas_texture) {
            (Some(a), Some(t)) => (a, t),
            _ => {
                ui.label(
                    egui::RichText::new("Tile atlas unavailable")
                        .size(13.0)
                        .color(colors.muted),
                );
                return;
            }
        };

        let atlas_count = atlas.tile_count() as usize;
        if atlas_count == 0 {
            ui.label(
                egui::RichText::new("No tiles loaded")
                    .size(13.0)
                    .color(colors.muted),
            );
            return;
        }

        let selectable_count = atlas_count.min(u16::MAX as usize);
        if *selected_ground_tile == 0 {
            *selected_ground_tile = 1;
        }
        let max_selectable = selectable_count as u16;
        if *selected_ground_tile > max_selectable {
            *selected_ground_tile = max_selectable;
        }

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Selected")
                    .size(13.0)
                    .color(colors.muted),
            );
            ui.label(
                egui::RichText::new(format!("#{}", *selected_ground_tile))
                    .size(13.0)
                    .family(egui::FontFamily::Monospace)
                    .color(colors.accent),
            );
        });
        ui.add_space(6.0);

        if selectable_count < atlas_count {
            ui.label(
                egui::RichText::new(format!(
                    "Showing first {} tiles (map format supports up to 65535).",
                    selectable_count
                ))
                .size(12.0)
                .color(colors.muted),
            );
            ui.add_space(6.0);
        }

        let (tile_w, tile_h) = atlas.tile_size();
        let preview_scale = TILE_PREVIEW_HEIGHT / tile_h as f32;
        let thumb_size = egui::vec2(tile_w as f32 * preview_scale, tile_h as f32 * preview_scale);
        let cell_size = egui::vec2((thumb_size.x + 16.0).max(54.0), thumb_size.y + 8.0);

        let available_w = ui.available_width().max(cell_size.x);
        let columns = ((available_w + TILE_COLUMN_GAP) / (cell_size.x + TILE_COLUMN_GAP))
            .floor()
            .max(1.0) as usize;
        let row_count = selectable_count.div_ceil(columns);
        let row_height = cell_size.y + TILE_ROW_GAP;

        // Keep room for the tab map section below while allowing tile scrolling.
        let available = ui.available_size();
        let tab_map_reserved = (available.x * 0.35 + 80.0).clamp(160.0, 300.0);
        let max_palette_height = (available.y - tab_map_reserved).max(96.0);

        egui::ScrollArea::vertical()
            .id_salt("ground_tiles_scroll")
            .max_height(max_palette_height)
            .auto_shrink([false, false])
            .show_rows(ui, row_height, row_count, |ui, row_range| {
                for row in row_range {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = TILE_COLUMN_GAP;
                        for col in 0..columns {
                            let idx = row * columns + col;
                            if idx >= selectable_count {
                                break;
                            }
                            Self::tile_picker_cell(
                                ui,
                                colors,
                                atlas,
                                texture,
                                idx as u32,
                                thumb_size,
                                cell_size,
                                selected_ground_tile,
                            );
                        }
                    });
                }
            });
    }

    #[allow(clippy::too_many_arguments)]
    fn tile_picker_cell(
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        atlas: &render::TileAtlas,
        texture: &egui::TextureHandle,
        atlas_index: u32,
        thumb_size: egui::Vec2,
        cell_size: egui::Vec2,
        selected_ground_tile: &mut u16,
    ) {
        let tile_id = (atlas_index + 1).min(u16::MAX as u32) as u16;
        let is_selected = *selected_ground_tile == tile_id;
        let (rect, response) = ui.allocate_exact_size(cell_size, egui::Sense::click());

        let bg = if is_selected {
            colors.accent.gamma_multiply(0.18)
        } else if response.hovered() {
            colors.panel_2
        } else {
            colors.bg.gamma_multiply(0.75)
        };
        let border = if is_selected {
            egui::Stroke::new(1.0, colors.accent)
        } else if response.hovered() {
            egui::Stroke::new(1.0, colors.border)
        } else {
            egui::Stroke::new(1.0, colors.border.gamma_multiply(0.5))
        };

        ui.painter()
            .rect(rect, 4.0, bg, border, egui::StrokeKind::Inside);

        let image_rect = egui::Rect::from_center_size(rect.center(), thumb_size);
        if let Some((u0, v0, u1, v1)) = atlas.tile_uv(atlas_index) {
            let uv = egui::Rect::from_min_max(egui::pos2(u0, v0), egui::pos2(u1, v1));
            let mut mesh = egui::Mesh::with_texture(texture.id());
            mesh.add_rect_with_uv(image_rect, uv, egui::Color32::WHITE);
            ui.painter().add(egui::Shape::mesh(mesh));
        }

        if response.clicked() {
            *selected_ground_tile = tile_id;
        }
        response.on_hover_text(format!("Tile {}", tile_id));
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

    /// Render a minimap showing only solid (impassable) boundaries.
    fn draw_tab_map(ui: &mut egui::Ui, colors: &ThemeColors, map: &map::Map, sotp: Option<&[u8]>) {
        let w = map.width as usize;
        let h = map.height as usize;

        if w == 0 || h == 0 {
            ui.label(
                egui::RichText::new("Empty map")
                    .size(13.0)
                    .color(colors.muted),
            );
            return;
        }

        let solid = Self::build_solid_grid(map, sotp);

        // Scale to fit available width — grows proportionally with the panel.
        let avail_w = ui.available_width();
        let total_iso_w = (w + h) as f32;
        let hw = (avail_w / total_iso_w).max(1.0);
        let hh = hw * 0.5;

        let total_w = total_iso_w * hw;
        let total_h = total_iso_w * hh;

        let (rect, _) = ui.allocate_exact_size(egui::vec2(total_w, total_h), egui::Sense::hover());

        let origin_x = rect.left() + h as f32 * hw;
        let origin_y = rect.top();

        let painter = ui.painter();
        let solid_fill = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 20);
        let solid_edge = egui::Stroke::new(
            1.0,
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 120),
        );

        let is_solid = |col: i32, row: i32| -> bool {
            if col < 0 || col >= w as i32 || row < 0 || row >= h as i32 {
                return false;
            }
            solid[row as usize * w + col as usize]
        };

        for row in 0..h {
            for col in 0..w {
                let idx = row * w + col;
                let solid_here = solid[idx];
                if !solid_here {
                    continue;
                }

                let cx = origin_x + (col as f32 - row as f32) * hw;
                let cy = origin_y + (col as f32 + row as f32) * hh;

                let top = egui::pos2(cx, cy);
                let right = egui::pos2(cx + hw, cy + hh);
                let bottom = egui::pos2(cx, cy + 2.0 * hh);
                let left = egui::pos2(cx - hw, cy + hh);
                let poly = vec![top, right, bottom, left];

                painter.add(egui::Shape::convex_polygon(
                    poly,
                    solid_fill,
                    egui::Stroke::NONE,
                ));

                let c = col as i32;
                let r = row as i32;

                if solid_here && !is_solid(c, r - 1) {
                    painter.line_segment([top, right], solid_edge);
                }
                if solid_here && !is_solid(c + 1, r) {
                    painter.line_segment([right, bottom], solid_edge);
                }
                if solid_here && !is_solid(c, r + 1) {
                    painter.line_segment([bottom, left], solid_edge);
                }
                if solid_here && !is_solid(c - 1, r) {
                    painter.line_segment([left, top], solid_edge);
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
