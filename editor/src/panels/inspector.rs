use eframe::egui;

use crate::document::PaintLayer;
use crate::theme::{ThemeColors, theme_colors};

const INSPECTOR_MIN_WIDTH: f32 = 260.0;
const INSPECTOR_MAX_WIDTH: f32 = 760.0;
const INSPECTOR_DEFAULT_WIDTH: f32 = 420.0;
const TILE_PREVIEW_HEIGHT: f32 = 26.0;
const WALL_PREVIEW_WIDTH: f32 = 28.0;
const WALL_PREVIEW_HEIGHT: f32 = 56.0;
const TILE_SHEET_MIN_HEIGHT: f32 = 96.0;

pub struct InspectorPanel;

impl InspectorPanel {
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        ctx: &egui::Context,
        map: &map::Map,
        sotp: Option<&[u8]>,
        tile_atlas: Option<&render::TileAtlas>,
        atlas_texture: Option<&egui::TextureHandle>,
        wall_atlas: Option<&render::SpriteAtlas>,
        wall_texture: Option<&egui::TextureHandle>,
        active_paint_layer: &mut PaintLayer,
        selected_ground_tile: &mut u16,
        selected_wall_tile: &mut u16,
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
                    wall_atlas,
                    wall_texture,
                    active_paint_layer,
                    selected_ground_tile,
                    selected_wall_tile,
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
        wall_atlas: Option<&render::SpriteAtlas>,
        wall_texture: Option<&egui::TextureHandle>,
        active_paint_layer: &mut PaintLayer,
        selected_ground_tile: &mut u16,
        selected_wall_tile: &mut u16,
    ) {
        let tab_map_id = ui.make_persistent_id("inspector_tab_map_preview");
        let tab_map_open = egui::collapsing_header::CollapsingState::load_with_default_open(
            ui.ctx(),
            tab_map_id,
            true,
        )
        .is_open();

        // Draw left border
        let panel_rect = ui.max_rect();
        ui.painter().line_segment(
            [
                egui::pos2(panel_rect.left() - 14.0, panel_rect.top() - 10.0),
                egui::pos2(panel_rect.left() - 14.0, panel_rect.bottom() + 10.0),
            ],
            egui::Stroke::new(1.0, colors.border),
        );

        // --- Tile palette section ---
        Self::section_header(ui, colors, "Tile Palette");
        ui.add_space(8.0);
        Self::draw_tile_palette(
            ui,
            colors,
            tile_atlas,
            atlas_texture,
            wall_atlas,
            wall_texture,
            active_paint_layer,
            selected_ground_tile,
            selected_wall_tile,
            tab_map_open,
        );

        // --- Tile palette bottom border ---
        Self::full_width_border(ui, colors, panel_rect);

        // --- Tab Map section ---
        egui::CollapsingHeader::new(
            egui::RichText::new("Tab Map")
                .size(14.0)
                .strong()
                .color(colors.text),
        )
        .id_salt("inspector_tab_map_preview")
        .default_open(true)
        .show_unindented(ui, |ui| {
            ui.add_space(6.0);
            Self::draw_tab_map(ui, colors, map, sotp);
        });

        // --- Tab Map bottom border ---
        Self::full_width_border(ui, colors, panel_rect);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_tile_palette(
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        tile_atlas: Option<&render::TileAtlas>,
        atlas_texture: Option<&egui::TextureHandle>,
        wall_atlas: Option<&render::SpriteAtlas>,
        wall_texture: Option<&egui::TextureHandle>,
        active_paint_layer: &mut PaintLayer,
        selected_ground_tile: &mut u16,
        selected_wall_tile: &mut u16,
        tab_map_open: bool,
    ) {
        ui.horizontal(|ui| {
            let show_ground = matches!(*active_paint_layer, PaintLayer::Ground);
            if ui.selectable_label(show_ground, "Ground").clicked() {
                *active_paint_layer = PaintLayer::Ground;
            }
            if ui.selectable_label(!show_ground, "Wall").clicked() {
                if matches!(*active_paint_layer, PaintLayer::Ground) {
                    *active_paint_layer = PaintLayer::LeftWall;
                }
            }
        });
        ui.add_space(4.0);

        if !matches!(*active_paint_layer, PaintLayer::Ground) {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Target").size(12.0).color(colors.muted));
                let left_response = ui
                    .selectable_label(matches!(*active_paint_layer, PaintLayer::LeftWall), "Left");
                if left_response.clicked() {
                    *active_paint_layer = PaintLayer::LeftWall;
                }
                left_response.on_hover_text("Toggle [Q]");

                let right_response = ui.selectable_label(
                    matches!(*active_paint_layer, PaintLayer::RightWall),
                    "Right",
                );
                if right_response.clicked() {
                    *active_paint_layer = PaintLayer::RightWall;
                }
                right_response.on_hover_text("Toggle [Q]");
            });
            ui.add_space(4.0);
        }

        let (selected_label, selected_id) = match *active_paint_layer {
            PaintLayer::Ground => ("Ground", *selected_ground_tile),
            PaintLayer::LeftWall => ("Wall (Left)", *selected_wall_tile),
            PaintLayer::RightWall => ("Wall (Right)", *selected_wall_tile),
        };

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Selected")
                    .size(13.0)
                    .color(colors.muted),
            );
            ui.label(
                egui::RichText::new(format!("{selected_label} #{selected_id}"))
                    .size(13.0)
                    .family(egui::FontFamily::Monospace)
                    .color(colors.accent),
            );
        });
        ui.add_space(6.0);

        let max_palette_height = Self::palette_max_height(ui, tab_map_open);

        if matches!(*active_paint_layer, PaintLayer::Ground) {
            Self::draw_ground_sheet(
                ui,
                colors,
                tile_atlas,
                atlas_texture,
                selected_ground_tile,
                max_palette_height,
            );
        } else {
            Self::draw_wall_sheet(
                ui,
                colors,
                wall_atlas,
                wall_texture,
                selected_wall_tile,
                max_palette_height,
            );
        }
    }

    fn palette_max_height(ui: &egui::Ui, tab_map_open: bool) -> f32 {
        // Keep room for the tab map section below while allowing tile scrolling.
        let available = ui.available_size();
        let tab_map_reserved = if tab_map_open {
            (available.x * 0.35 + 80.0).clamp(160.0, 300.0)
        } else {
            40.0
        };
        (available.y - tab_map_reserved).max(TILE_SHEET_MIN_HEIGHT)
    }

    fn draw_ground_sheet(
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        tile_atlas: Option<&render::TileAtlas>,
        atlas_texture: Option<&egui::TextureHandle>,
        selected_ground_tile: &mut u16,
        max_palette_height: f32,
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
                egui::RichText::new("No ground tiles loaded")
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

        let (tile_w, tile_h) = atlas.tile_size();
        let preview_scale = TILE_PREVIEW_HEIGHT / tile_h as f32;
        let cell_size = egui::vec2(
            (tile_w as f32 * preview_scale).max(1.0).round(),
            (tile_h as f32 * preview_scale).max(1.0).round(),
        );

        let available_w = ui.available_width().max(cell_size.x);
        let columns = (available_w / cell_size.x).floor().max(1.0) as usize;
        let row_count = selectable_count.div_ceil(columns);
        let row_height = cell_size.y;

        egui::ScrollArea::vertical()
            .id_salt("ground_tiles_scroll")
            .max_height(max_palette_height)
            .auto_shrink([false, false])
            .show_rows(ui, row_height, row_count, |ui, row_range| {
                for row in row_range {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                        for col in 0..columns {
                            let idx = row * columns + col;
                            if idx >= selectable_count {
                                break;
                            }
                            Self::ground_picker_cell(
                                ui,
                                colors,
                                atlas,
                                texture,
                                idx as u32,
                                cell_size,
                                selected_ground_tile,
                            );
                        }
                    });
                }
            });
    }

    fn draw_wall_sheet(
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        wall_atlas: Option<&render::SpriteAtlas>,
        wall_texture: Option<&egui::TextureHandle>,
        selected_wall_tile: &mut u16,
        max_palette_height: f32,
    ) {
        let (atlas, texture) = match (wall_atlas, wall_texture) {
            (Some(a), Some(t)) => (a, t),
            _ => {
                ui.label(
                    egui::RichText::new("Wall atlas unavailable")
                        .size(13.0)
                        .color(colors.muted),
                );
                return;
            }
        };

        let wall_ids = (1..atlas.sprite_count())
            .filter(|&id| atlas.sprite_rect(id).is_some())
            .collect::<Vec<_>>();
        if wall_ids.is_empty() {
            ui.label(
                egui::RichText::new("No wall sprites loaded")
                    .size(13.0)
                    .color(colors.muted),
            );
            return;
        }

        Self::clamp_wall_selection(selected_wall_tile, &wall_ids);

        let cell_size = egui::vec2(WALL_PREVIEW_WIDTH, WALL_PREVIEW_HEIGHT);
        let available_w = ui.available_width().max(cell_size.x);
        let columns = (available_w / cell_size.x).floor().max(1.0) as usize;
        let row_count = wall_ids.len().div_ceil(columns);
        let row_height = cell_size.y;

        egui::ScrollArea::vertical()
            .id_salt("wall_tiles_scroll")
            .max_height(max_palette_height)
            .auto_shrink([false, false])
            .show_rows(ui, row_height, row_count, |ui, row_range| {
                for row in row_range {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                        for col in 0..columns {
                            let idx = row * columns + col;
                            if idx >= wall_ids.len() {
                                break;
                            }
                            Self::wall_picker_cell(
                                ui,
                                colors,
                                atlas,
                                texture,
                                wall_ids[idx],
                                cell_size,
                                selected_wall_tile,
                            );
                        }
                    });
                }
            });
    }

    fn clamp_wall_selection(selected_wall_tile: &mut u16, wall_ids: &[u32]) {
        let first = wall_ids.first().copied().unwrap_or(0);
        if *selected_wall_tile == 0 {
            *selected_wall_tile = first.min(u16::MAX as u32) as u16;
            return;
        }
        let selected = *selected_wall_tile as u32;
        if wall_ids.binary_search(&selected).is_err() {
            *selected_wall_tile = first.min(u16::MAX as u32) as u16;
        }
    }

    fn ground_picker_cell(
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        atlas: &render::TileAtlas,
        texture: &egui::TextureHandle,
        atlas_index: u32,
        cell_size: egui::Vec2,
        selected_ground_tile: &mut u16,
    ) {
        let tile_id = (atlas_index + 1).min(u16::MAX as u32) as u16;
        let is_selected = *selected_ground_tile == tile_id;
        let (rect, response) = ui.allocate_exact_size(cell_size, egui::Sense::click());
        if let Some((u0, v0, u1, v1)) = atlas.tile_uv(atlas_index) {
            let uv = egui::Rect::from_min_max(egui::pos2(u0, v0), egui::pos2(u1, v1));
            let mut mesh = egui::Mesh::with_texture(texture.id());
            mesh.add_rect_with_uv(rect, uv, egui::Color32::WHITE);
            ui.painter().add(egui::Shape::mesh(mesh));
        }

        let border = if is_selected {
            egui::Stroke::new(2.0, colors.accent)
        } else if response.hovered() {
            egui::Stroke::new(1.0, colors.border)
        } else {
            egui::Stroke::NONE
        };
        if border.width > 0.0 {
            ui.painter()
                .rect_stroke(rect, 0.0, border, egui::StrokeKind::Inside);
        }

        if response.clicked() {
            *selected_ground_tile = tile_id;
        }
        response.on_hover_text(format!("Tile {}", tile_id));
    }

    fn wall_picker_cell(
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        atlas: &render::SpriteAtlas,
        texture: &egui::TextureHandle,
        wall_id: u32,
        cell_size: egui::Vec2,
        selected_wall_tile: &mut u16,
    ) {
        let is_selected = *selected_wall_tile as u32 == wall_id;
        let (rect, response) = ui.allocate_exact_size(cell_size, egui::Sense::click());

        if let (Some((u0, v0, u1, v1)), Some((_, _, sw, sh))) =
            (atlas.sprite_uv(wall_id), atlas.sprite_rect(wall_id))
        {
            let mut draw_w = cell_size.x;
            let mut draw_h = sh as f32 * (draw_w / sw as f32);
            if draw_h > cell_size.y {
                let fit = cell_size.y / draw_h;
                draw_w *= fit;
                draw_h *= fit;
            }
            let image_rect = egui::Rect::from_min_max(
                egui::pos2(rect.center().x - draw_w * 0.5, rect.bottom() - draw_h),
                egui::pos2(rect.center().x + draw_w * 0.5, rect.bottom()),
            );
            let uv = egui::Rect::from_min_max(egui::pos2(u0, v0), egui::pos2(u1, v1));
            let mut mesh = egui::Mesh::with_texture(texture.id());
            mesh.add_rect_with_uv(image_rect, uv, egui::Color32::WHITE);
            ui.painter().add(egui::Shape::mesh(mesh));
        }

        let border = if is_selected {
            egui::Stroke::new(2.0, colors.accent)
        } else if response.hovered() {
            egui::Stroke::new(1.0, colors.border)
        } else {
            egui::Stroke::NONE
        };
        if border.width > 0.0 {
            ui.painter()
                .rect_stroke(rect, 0.0, border, egui::StrokeKind::Inside);
        }

        if response.clicked() {
            *selected_wall_tile = wall_id.min(u16::MAX as u32) as u16;
        }
        response.on_hover_text(format!("Wall {}", wall_id));
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
