use eframe::egui;

use crate::document::{Camera, LayerVisibility};
use crate::theme::{ThemeColors, theme_colors};

pub struct ViewportPanel;

impl ViewportPanel {
    pub fn show(
        ctx: &egui::Context,
        map: &map::Map,
        camera: &mut Camera,
        tile_atlas: Option<&render::TileAtlas>,
        atlas_texture: Option<&egui::TextureHandle>,
        layers: &mut LayerVisibility,
        show_grid: &mut bool,
    ) -> Option<(u16, u16)> {
        let colors = theme_colors();
        let mut hover_tile = None;

        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(colors.bg)
                    .inner_margin(egui::Margin::ZERO),
            )
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let response =
                    ui.interact(rect, ui.id().with("viewport"), egui::Sense::click_and_drag());

                // Right-click or middle-click drag panning
                if response.dragged_by(egui::PointerButton::Secondary)
                    || response.dragged_by(egui::PointerButton::Middle)
                {
                    camera.offset -= response.drag_delta();
                }

                // Mouse wheel zoom
                let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll != 0.0 {
                    let old_zoom = camera.zoom;
                    let steps = (scroll / 40.0).round();
                    camera.zoom = (camera.zoom + steps * 0.05).clamp(0.25, 4.0);
                    // Scale offset to keep viewport center at the same map point
                    camera.offset *= camera.zoom / old_zoom;
                }

                // Arrow key panning (uses OS key repeat)
                Self::handle_arrow_keys(ui, camera);

                // Clamp camera to bounds
                Self::clamp_camera(camera, map, rect);

                let painter = ui.painter_at(rect);

                // Background dot pattern
                Self::draw_background_dots(&painter, rect);

                // Zoom-scaled tile half-sizes
                let zoom = camera.zoom;
                let half_w = map::TILE_WIDTH / 2.0 * zoom;
                let half_h = map::TILE_HEIGHT / 2.0 * zoom;
                let map_center_x = (map.width as f32 - map.height as f32) * half_w / 2.0;
                let map_center_y = (map.width as f32 + map.height as f32) * half_h / 2.0;
                let viewport_center = rect.center();
                let origin = egui::pos2(
                    viewport_center.x - camera.offset.x - map_center_x,
                    viewport_center.y - camera.offset.y - map_center_y,
                );

                // Draw textured tiles
                if layers.ground {
                    if let (Some(atlas), Some(texture)) = (tile_atlas, atlas_texture) {
                        Self::draw_tiles(
                            &painter, map, atlas, texture, origin, half_w, half_h, rect,
                        );
                    }
                }

                // Draw grid overlay
                if *show_grid {
                    Self::draw_grid(&painter, map, origin, half_w, half_h);
                }

                // Mouse → tile hover
                if let Some(pointer_pos) = response.hover_pos() {
                    let tile = Self::screen_to_tile(pointer_pos, origin, half_w, half_h, map);
                    if let Some((col, row)) = tile {
                        hover_tile = Some((col, row));
                        Self::draw_tile_highlight(
                            &painter, col, row, origin, half_w, half_h, &colors,
                        );
                    }
                }

                // Floating overlay: Grid | G L R
                Self::draw_overlay(ui, rect, show_grid, layers, &colors);
            });

        hover_tile
    }

    fn draw_overlay(
        ui: &mut egui::Ui,
        viewport_rect: egui::Rect,
        show_grid: &mut bool,
        layers: &mut LayerVisibility,
        colors: &ThemeColors,
    ) {
        let btn_h = 24.0;
        let grid_w = 42.0;
        let layer_w = 26.0;
        let sep_w = 1.0;
        let pad = 6.0;
        let gap = 4.0;

        let total_w = pad + grid_w + gap + sep_w + gap + layer_w * 3.0 + gap * 2.0 + pad;
        let total_h = pad * 2.0 + btn_h;

        let overlay_rect = egui::Rect::from_min_size(
            egui::pos2(
                viewport_rect.right() - total_w - 12.0,
                viewport_rect.top() + 12.0,
            ),
            egui::vec2(total_w, total_h),
        );

        let btn_y = overlay_rect.top() + pad;
        let mut x = overlay_rect.left() + pad;

        // Compute all rects first
        let grid_rect =
            egui::Rect::from_min_size(egui::pos2(x, btn_y), egui::vec2(grid_w, btn_h));
        x += grid_w + gap;

        let sep_rect = egui::Rect::from_min_size(
            egui::pos2(x, btn_y + 2.0),
            egui::vec2(sep_w, btn_h - 4.0),
        );
        x += sep_w + gap;

        let ground_rect =
            egui::Rect::from_min_size(egui::pos2(x, btn_y), egui::vec2(layer_w, btn_h));
        x += layer_w + gap;

        let left_rect =
            egui::Rect::from_min_size(egui::pos2(x, btn_y), egui::vec2(layer_w, btn_h));
        x += layer_w + gap;

        let right_rect =
            egui::Rect::from_min_size(egui::pos2(x, btn_y), egui::vec2(layer_w, btn_h));

        // Do all interactions (needs &mut ui)
        Self::overlay_toggle(ui, "Grid", grid_rect, show_grid, colors, "Grid");
        Self::overlay_toggle(
            ui,
            "G",
            ground_rect,
            &mut layers.ground,
            colors,
            "Ground (Cmd+1)",
        );
        Self::overlay_toggle(
            ui,
            "L",
            left_rect,
            &mut layers.left_wall,
            colors,
            "Left Wall (Cmd+2)",
        );
        Self::overlay_toggle(
            ui,
            "R",
            right_rect,
            &mut layers.right_wall,
            colors,
            "Right Wall (Cmd+3)",
        );

        // Paint background and separator (only needs &ui for painter)
        ui.painter().rect(
            overlay_rect,
            6.0,
            egui::Color32::from_rgba_unmultiplied(18, 20, 24, 200),
            egui::Stroke::new(1.0, colors.border),
            egui::StrokeKind::Inside,
        );
        ui.painter().rect_filled(sep_rect, 0.0, colors.border);
    }

    fn overlay_toggle(
        ui: &mut egui::Ui,
        label: &str,
        rect: egui::Rect,
        enabled: &mut bool,
        colors: &ThemeColors,
        tooltip: &str,
    ) {
        let id = ui.id().with(("overlay_toggle", label));
        let response = ui.interact(rect, id, egui::Sense::click());

        let bg = if *enabled {
            colors.accent.gamma_multiply(0.2)
        } else if response.hovered() {
            colors.panel_2
        } else {
            egui::Color32::TRANSPARENT
        };

        let border = if *enabled {
            egui::Stroke::new(1.0, colors.accent)
        } else if response.hovered() {
            egui::Stroke::new(1.0, colors.border)
        } else {
            egui::Stroke::NONE
        };

        ui.painter()
            .rect(rect, 4.0, bg, border, egui::StrokeKind::Inside);

        let text_color = if *enabled {
            colors.accent
        } else {
            colors.muted
        };

        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(13.0),
            text_color,
        );

        if response.clicked() {
            *enabled = !*enabled;
        }

        response.on_hover_text(tooltip);
    }

    fn handle_arrow_keys(ui: &egui::Ui, camera: &mut Camera) {
        let step = map::TILE_WIDTH * camera.zoom;

        ui.input(|i| {
            if i.key_pressed(egui::Key::ArrowRight) {
                camera.offset.x += step;
            }
            if i.key_pressed(egui::Key::ArrowLeft) {
                camera.offset.x -= step;
            }
            if i.key_pressed(egui::Key::ArrowDown) {
                camera.offset.y += step;
            }
            if i.key_pressed(egui::Key::ArrowUp) {
                camera.offset.y -= step;
            }
        });
    }

    fn clamp_camera(camera: &mut Camera, map: &map::Map, viewport: egui::Rect) {
        let half_w = map::TILE_WIDTH / 2.0 * camera.zoom;
        let half_h = map::TILE_HEIGHT / 2.0 * camera.zoom;

        // Map diamond pixel extents
        let map_pixel_w = (map.width as f32 + map.height as f32) * half_w;
        let map_pixel_h = (map.width as f32 + map.height as f32) * half_h;

        let max_x = map_pixel_w / 2.0 + viewport.width() / 2.0;
        let max_y = map_pixel_h / 2.0 + viewport.height() / 2.0;

        camera.offset.x = camera.offset.x.clamp(-max_x, max_x);
        camera.offset.y = camera.offset.y.clamp(-max_y, max_y);
    }

    fn draw_background_dots(painter: &egui::Painter, rect: egui::Rect) {
        let grid_spacing = 24.0;
        let dot_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 8);
        let mut x = rect.left() + grid_spacing;
        while x < rect.right() {
            let mut y = rect.top() + grid_spacing;
            while y < rect.bottom() {
                painter.circle_filled(egui::pos2(x, y), 0.5, dot_color);
                y += grid_spacing;
            }
            x += grid_spacing;
        }
    }

    fn draw_tiles(
        painter: &egui::Painter,
        map: &map::Map,
        atlas: &render::TileAtlas,
        texture: &egui::TextureHandle,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
        viewport: egui::Rect,
    ) {
        let tw = half_w * 2.0;
        let th = half_h * 2.0;
        let mut mesh = egui::Mesh::with_texture(texture.id());

        for row in 0..map.height {
            for col in 0..map.width {
                let tile = &map.tiles[row as usize * map.width as usize + col as usize];
                let ground = tile.ground;
                if ground == 0 {
                    continue;
                }
                // Ground IDs are 1-based (0 = empty), atlas indices are 0-based
                let atlas_index = (ground - 1) as u32;

                // Top vertex of the diamond
                let cx = origin.x + (col as f32 - row as f32) * half_w;
                let cy = origin.y + (col as f32 + row as f32) * half_h;

                // Bounding rect for the tile image
                let tile_rect =
                    egui::Rect::from_min_size(egui::pos2(cx - half_w, cy), egui::vec2(tw, th));

                // Frustum cull
                if !viewport.intersects(tile_rect) {
                    continue;
                }

                if let Some((u_min, v_min, u_max, v_max)) = atlas.tile_uv(atlas_index) {
                    let uv = egui::Rect::from_min_max(
                        egui::pos2(u_min, v_min),
                        egui::pos2(u_max, v_max),
                    );
                    mesh.add_rect_with_uv(tile_rect, uv, egui::Color32::WHITE);
                }
            }
        }

        if !mesh.is_empty() {
            painter.add(egui::Shape::mesh(mesh));
        }
    }

    fn draw_grid(
        painter: &egui::Painter,
        map: &map::Map,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
    ) {
        let grid_w = map.width as f32;
        let grid_h = map.height as f32;

        let grid_line = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 18);
        let grid_stroke = egui::Stroke::new(0.5, grid_line);

        // Horizontal grid lines (along iso X axis)
        for row in 0..=map.height {
            let r = row as f32;
            let start_x = origin.x - r * half_w;
            let start_y = origin.y + r * half_h;
            let end_x = start_x + grid_w * half_w;
            let end_y = start_y + grid_w * half_h;
            painter.line_segment(
                [egui::pos2(start_x, start_y), egui::pos2(end_x, end_y)],
                grid_stroke,
            );
        }

        // Vertical grid lines (along iso Y axis)
        for col in 0..=map.width {
            let c = col as f32;
            let start_x = origin.x + c * half_w;
            let start_y = origin.y + c * half_h;
            let end_x = start_x - grid_h * half_w;
            let end_y = start_y + grid_h * half_h;
            painter.line_segment(
                [egui::pos2(start_x, start_y), egui::pos2(end_x, end_y)],
                grid_stroke,
            );
        }
    }

    fn screen_to_tile(
        screen_pos: egui::Pos2,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
        map: &map::Map,
    ) -> Option<(u16, u16)> {
        let dx = screen_pos.x - origin.x;
        let dy = screen_pos.y - origin.y;

        let col = ((dx / half_w + dy / half_h) / 2.0).floor();
        let row = ((dy / half_h - dx / half_w) / 2.0).floor();

        if col >= 0.0 && row >= 0.0 && (col as u16) < map.width && (row as u16) < map.height {
            Some((col as u16, row as u16))
        } else {
            None
        }
    }

    fn draw_tile_highlight(
        painter: &egui::Painter,
        col: u16,
        row: u16,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
        colors: &ThemeColors,
    ) {
        // Diamond vertices for the hovered tile
        let cx = origin.x + (col as f32 - row as f32) * half_w;
        let cy = origin.y + (col as f32 + row as f32) * half_h;

        let top = egui::pos2(cx, cy);
        let right = egui::pos2(cx + half_w, cy + half_h);
        let bottom = egui::pos2(cx, cy + half_h * 2.0);
        let left = egui::pos2(cx - half_w, cy + half_h);

        let stroke = egui::Stroke::new(1.5, colors.accent);
        painter.line_segment([top, right], stroke);
        painter.line_segment([right, bottom], stroke);
        painter.line_segment([bottom, left], stroke);
        painter.line_segment([left, top], stroke);
    }
}
