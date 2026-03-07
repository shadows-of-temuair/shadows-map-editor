use eframe::egui;

use super::toolbar::Tool;
use crate::document::{Camera, LayerVisibility, PaintLayer, TileSelection};
use crate::prefab;
use crate::shape::{self, ShapeKind};
use crate::theme::{ThemeColors, theme_colors};
use crate::widgets::{icons, tooltip};

const SELECTION_AUTO_PAN_MARGIN: f32 = 56.0;
const SELECTION_AUTO_PAN_SPEED: f32 = 520.0;
const CONTEXT_ICON_CUT: &str = "\u{2702}";
const CONTEXT_ICON_COPY: &str = "\u{E202}";
const CONTEXT_ICON_PASTE: &str = "\u{1F4CB}";
const CONTEXT_ICON_CREATE_PREFAB: &str = "\u{EB03}";
const CONTEXT_ICON_DELETE: &str = "\u{1F5D1}";

/// Returns true if a wall tile ID should be rendered.
///
/// Matches the reference engine's `IsRenderedTileIndex`:
///   `(id > 10012) || ((id % 10000) > 12)`
/// ID 0 is always "no wall".
fn is_rendered_wall(id: u16) -> bool {
    if id == 0 {
        return false;
    }
    (id > 10012) || ((id % 10000) > 12)
}

fn is_layer_visible(layers: &LayerVisibility, layer: PaintLayer) -> bool {
    match layer {
        PaintLayer::Ground => layers.ground,
        PaintLayer::LeftWall => layers.left_wall,
        PaintLayer::RightWall => layers.right_wall,
    }
}

#[derive(Default)]
pub struct ViewportResult {
    pub hover_tile: Option<(u16, u16)>,
    pub clicked_tile: Option<(u16, u16)>,
    pub selection_drag_started: Option<(u16, u16)>,
    pub selection_drag_tile: Option<(u16, u16)>,
    pub cut_selection_requested: bool,
    pub copy_selection_requested: bool,
    pub clear_selection_requested: bool,
    pub activate_paste_preview: bool,
    pub paste_preview_clicked_tile: Option<(u16, u16)>,
    pub cancel_paste_preview: bool,
    pub create_prefab_requested: bool,
    pub delete_selection_requested: bool,
    pub painted_tile: Option<(u16, u16, u16)>,
    pub fill_clicked_tile: Option<(u16, u16, u16)>,
    pub pencil_clicked_tile: Option<(u16, u16)>,
    pub pencil_shift_clicked_tile: Option<(u16, u16)>,
    pub line_clicked_tile: Option<(u16, u16)>,
    pub shape_clicked_tile: Option<(u16, u16)>,
    pub stamp_clicked_tile: Option<(u16, u16)>,
    pub eyedropper_pick: Option<EyedropperPick>,
}

#[derive(Clone, Copy)]
pub struct SelectionMovePreview<'a> {
    pub map: &'a map::Map,
    pub top_left: (u16, u16),
    pub layers: LayerVisibility,
    pub ignore_overwrite_region: Option<TileSelection>,
}

#[derive(Clone, Copy)]
pub enum EyedropperPick {
    Ground(u16),
    LeftWall(u16),
    RightWall(u16),
}

pub struct ViewportPanel;

#[derive(Clone, Copy)]
struct WallOutlineSprite {
    source_rect: (u32, u32, u32, u32),
    screen_rect: egui::Rect,
}

impl ViewportPanel {
    pub fn show(
        ctx: &egui::Context,
        map: &map::Map,
        camera: &mut Camera,
        interaction_enabled: bool,
        active_tool: Tool,
        active_shape: ShapeKind,
        paint_layer: PaintLayer,
        selected_ground_tile: u16,
        selected_wall_tile: u16,
        line_preview_start: Option<(u16, u16)>,
        shape_preview_start: Option<(u16, u16)>,
        current_selection: Option<TileSelection>,
        selection_move_active: bool,
        selection_drag_start_tile: Option<(u16, u16)>,
        selection_move_preview: Option<SelectionMovePreview<'_>>,
        paste_preview: Option<SelectionMovePreview<'_>>,
        paste_preview_anchor: Option<(u16, u16)>,
        selection_clipboard_available: bool,
        stamp_prefab: Option<&map::Map>,
        tile_atlas: Option<&render::TileAtlas>,
        atlas_texture: Option<&egui::TextureHandle>,
        wall_atlas: Option<&render::SpriteAtlas>,
        wall_texture: Option<&egui::TextureHandle>,
        tab_overlay_texture: Option<&egui::TextureHandle>,
        layers: &mut LayerVisibility,
        show_grid: &mut bool,
        show_collision_overlay: &mut bool,
        sotp: Option<&[u8]>,
    ) -> ViewportResult {
        let colors = theme_colors();
        let mut result = ViewportResult::default();
        let paste_preview_active = paste_preview.is_some();
        let selected_paint_tile = match paint_layer {
            PaintLayer::Ground => selected_ground_tile,
            PaintLayer::LeftWall | PaintLayer::RightWall => selected_wall_tile,
        };

        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(colors.bg)
                    .inner_margin(egui::Margin::ZERO),
            )
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let viewport_center_id = ui.id().with("viewport_center");
                if let Some(previous_center) =
                    ui.ctx().data(|data| data.get_temp::<egui::Pos2>(viewport_center_id))
                {
                    camera.offset += rect.center() - previous_center;
                }
                ui.ctx().data_mut(|data| {
                    data.insert_temp(viewport_center_id, rect.center());
                });
                let response = ui.interact(
                    rect,
                    ui.id().with("viewport"),
                    if interaction_enabled {
                        egui::Sense::click_and_drag()
                    } else {
                        egui::Sense::hover()
                    },
                );
                let context_menu_open = interaction_enabled && response.context_menu_opened();
                let mut cursor_icon = if response.hovered() {
                    Some(egui::CursorIcon::Default)
                } else {
                    None
                };

                // Right-click or middle-click drag panning
                let is_mouse_panning = interaction_enabled
                    && (response.dragged_by(egui::PointerButton::Secondary)
                        || response.dragged_by(egui::PointerButton::Middle));
                if is_mouse_panning {
                    camera.offset -= response.drag_delta();
                    cursor_icon = Some(egui::CursorIcon::Grabbing);
                }

                // Mouse wheel zoom (only when the viewport is hovered).
                // This prevents map zoom from triggering while scrolling side panels.
                if interaction_enabled && response.hovered() {
                    let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                    if scroll != 0.0 {
                        let old_zoom = camera.zoom;
                        let steps = (scroll / 40.0).round();
                        camera.zoom = (camera.zoom + steps * 0.05).clamp(0.25, 4.0);
                        // Scale offset to keep viewport center at the same map point
                        camera.offset *= camera.zoom / old_zoom;
                    }
                }

                let overlay_rect = Self::overlay_rect(rect);
                let global_primary_down =
                    ui.input(|i| i.pointer.button_down(egui::PointerButton::Primary));
                let viewport_primary_down =
                    response.is_pointer_button_down_on() && global_primary_down;
                let primary_pressed = interaction_enabled
                    && !context_menu_open
                    && response.contains_pointer()
                    && ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary));
                let primary_down = interaction_enabled
                    && !context_menu_open
                    && global_primary_down
                    && (viewport_primary_down || selection_drag_start_tile.is_some());
                let pointer_interact_pos = ui.input(|i| i.pointer.interact_pos());
                let selection_press_valid = active_tool == Tool::Select
                    && !paste_preview_active
                    && primary_pressed
                    && pointer_interact_pos
                        .map(|pos| rect.contains(pos) && !overlay_rect.contains(pos))
                        .unwrap_or(false);
                let selection_drag_active = active_tool == Tool::Select
                    && !paste_preview_active
                    && primary_down
                    && (selection_drag_start_tile.is_some() || selection_press_valid);

                if active_tool == Tool::Select
                    && interaction_enabled
                    && !paste_preview_active
                    && primary_down
                    && selection_drag_start_tile.is_some()
                {
                    if let Some(pointer_pos) = pointer_interact_pos {
                        if rect.contains(pointer_pos) {
                            let dt = ui.input(|i| i.predicted_dt.max(1.0 / 120.0));
                            let auto_pan = Self::selection_auto_pan_delta(rect, pointer_pos, dt);
                            if auto_pan != egui::Vec2::ZERO {
                                camera.offset += auto_pan;
                                ctx.request_repaint();
                            }
                        }
                    }
                }

                // Arrow key panning (uses OS key repeat)
                if interaction_enabled {
                    Self::handle_arrow_keys(ui, camera);
                }

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
                let under_wall_selection = Self::under_wall_selection(
                    response.hovered(),
                    map,
                    active_tool,
                    paste_preview_active,
                    current_selection,
                    selection_move_active,
                    selection_drag_active,
                    selection_drag_start_tile,
                    pointer_interact_pos,
                    rect,
                    overlay_rect,
                    origin,
                    half_w,
                    half_h,
                );
                let selection_pulse = Self::selection_pulse_opacity(ctx);
                if active_tool == Tool::Select
                    && !paste_preview_active
                    && (current_selection.is_some()
                        || (selection_drag_active && !selection_move_active))
                {
                    ctx.request_repaint();
                }

                // Draw all tiles in isometric depth order (back-to-front).
                // For each tile: ground, then left wall, then right wall.
                // This prevents tall wall sprites from clipping behind
                // tiles that are closer to the camera.
                Self::draw_scene(
                    &painter,
                    map,
                    tile_atlas,
                    atlas_texture,
                    wall_atlas,
                    wall_texture,
                    origin,
                    half_w,
                    half_h,
                    rect,
                    layers,
                    under_wall_selection,
                    selection_pulse,
                );

                // Draw grid overlay
                if *show_grid {
                    Self::draw_grid(&painter, map, origin, half_w, half_h);
                }

                // Draw collision overlay from SOTP passability data.
                if *show_collision_overlay {
                    if let Some(sotp) = sotp {
                        Self::draw_collision_overlay(
                            &painter,
                            map,
                            sotp,
                            tab_overlay_texture,
                            origin,
                            half_w,
                            half_h,
                            rect,
                        );
                    } else {
                        *show_collision_overlay = false;
                    }
                }

                let mut selection_preview = None;

                // Mouse → tile hover and click selection
                let pointer_pos = if !interaction_enabled {
                    None
                } else if selection_drag_active {
                    pointer_interact_pos.map(|pos| Self::clamp_point_to_rect(pos, rect.shrink(1.0)))
                } else {
                    response.hover_pos()
                };
                if let Some(pointer_pos) = pointer_pos {
                    if selection_drag_active || !overlay_rect.contains(pointer_pos) {
                        let tile = if selection_drag_active {
                            Self::screen_to_tile_clamped(pointer_pos, origin, half_w, half_h, map)
                        } else if active_tool == Tool::Select {
                            current_selection
                                .and_then(|selection| {
                                    Self::selected_tile_at_screen_pos(
                                        selection,
                                        pointer_pos,
                                        origin,
                                        half_w,
                                        half_h,
                                    )
                                })
                                .or_else(|| {
                                    Self::screen_to_tile(pointer_pos, origin, half_w, half_h, map)
                                })
                        } else {
                            Self::screen_to_tile(pointer_pos, origin, half_w, half_h, map)
                        };
                        if let Some((col, row)) = tile {
                            result.hover_tile = Some((col, row));

                            if paste_preview_active {
                                cursor_icon = Some(egui::CursorIcon::Crosshair);
                                if response.clicked_by(egui::PointerButton::Primary) {
                                    result.paste_preview_clicked_tile = Some((col, row));
                                }
                                if response.clicked_by(egui::PointerButton::Secondary) {
                                    result.cancel_paste_preview = true;
                                }
                            } else {
                                match active_tool {
                                    Tool::Select => {
                                        cursor_icon =
                                            Some(if selection_move_active && primary_down {
                                                egui::CursorIcon::Grabbing
                                            } else {
                                                egui::CursorIcon::Crosshair
                                            });
                                        if selection_press_valid {
                                            result.selection_drag_started = Some((col, row));
                                        }

                                        if primary_down {
                                            let anchor = result
                                                .selection_drag_started
                                                .or(selection_drag_start_tile);
                                            if let Some(anchor) = anchor {
                                                result.selection_drag_tile = Some((col, row));
                                                if !selection_move_active {
                                                    selection_preview =
                                                        Some(TileSelection::from_points(
                                                            anchor,
                                                            (col, row),
                                                        ));
                                                }
                                            }
                                        } else if primary_pressed && !selection_move_active {
                                            selection_preview = Some(TileSelection::from_points(
                                                (col, row),
                                                (col, row),
                                            ));
                                        }
                                    }
                                    Tool::Pencil if selected_paint_tile != 0 => {
                                        cursor_icon = Some(egui::CursorIcon::Crosshair);
                                        Self::draw_paint_preview(
                                            &painter,
                                            col,
                                            row,
                                            origin,
                                            half_w,
                                            half_h,
                                            paint_layer,
                                            selected_paint_tile,
                                            tile_atlas,
                                            atlas_texture,
                                            wall_atlas,
                                            wall_texture,
                                        );
                                        let shift_held = ui.input(|i| i.modifiers.shift);
                                        if response.clicked_by(egui::PointerButton::Primary) {
                                            if shift_held {
                                                result.pencil_shift_clicked_tile = Some((col, row));
                                            } else {
                                                result.pencil_clicked_tile = Some((col, row));
                                            }
                                        }
                                        let is_primary_painting = response
                                            .is_pointer_button_down_on()
                                            && ui.input(|i| {
                                                i.pointer.button_down(egui::PointerButton::Primary)
                                            });
                                        if is_primary_painting && !shift_held {
                                            result.painted_tile =
                                                Some((col, row, selected_paint_tile));
                                        }
                                    }
                                    Tool::Line if selected_paint_tile != 0 => {
                                        cursor_icon = Some(egui::CursorIcon::Crosshair);
                                        if let Some(start) = line_preview_start {
                                            Self::draw_paint_line_preview(
                                                &painter,
                                                start,
                                                (col, row),
                                                origin,
                                                half_w,
                                                half_h,
                                                paint_layer,
                                                selected_paint_tile,
                                                tile_atlas,
                                                atlas_texture,
                                                wall_atlas,
                                                wall_texture,
                                            );
                                        } else {
                                            Self::draw_paint_preview(
                                                &painter,
                                                col,
                                                row,
                                                origin,
                                                half_w,
                                                half_h,
                                                paint_layer,
                                                selected_paint_tile,
                                                tile_atlas,
                                                atlas_texture,
                                                wall_atlas,
                                                wall_texture,
                                            );
                                        }
                                        if response.clicked_by(egui::PointerButton::Primary) {
                                            result.line_clicked_tile = Some((col, row));
                                            result.clicked_tile = Some((col, row));
                                        }
                                    }
                                    Tool::Shape if selected_paint_tile != 0 => {
                                        cursor_icon = Some(egui::CursorIcon::Crosshair);
                                        if let Some(start) = shape_preview_start {
                                            Self::draw_paint_shape_preview(
                                                &painter,
                                                map,
                                                active_shape,
                                                start,
                                                (col, row),
                                                origin,
                                                half_w,
                                                half_h,
                                                paint_layer,
                                                selected_paint_tile,
                                                tile_atlas,
                                                atlas_texture,
                                                wall_atlas,
                                                wall_texture,
                                            );
                                        } else {
                                            Self::draw_paint_preview(
                                                &painter,
                                                col,
                                                row,
                                                origin,
                                                half_w,
                                                half_h,
                                                paint_layer,
                                                selected_paint_tile,
                                                tile_atlas,
                                                atlas_texture,
                                                wall_atlas,
                                                wall_texture,
                                            );
                                        }
                                        if response.clicked_by(egui::PointerButton::Primary) {
                                            result.shape_clicked_tile = Some((col, row));
                                            result.clicked_tile = Some((col, row));
                                        }
                                    }
                                    Tool::Fill if selected_paint_tile != 0 => {
                                        cursor_icon = Some(egui::CursorIcon::Crosshair);
                                        Self::draw_paint_preview(
                                            &painter,
                                            col,
                                            row,
                                            origin,
                                            half_w,
                                            half_h,
                                            paint_layer,
                                            selected_paint_tile,
                                            tile_atlas,
                                            atlas_texture,
                                            wall_atlas,
                                            wall_texture,
                                        );
                                        if response.clicked_by(egui::PointerButton::Primary) {
                                            result.fill_clicked_tile =
                                                Some((col, row, selected_paint_tile));
                                            result.clicked_tile = Some((col, row));
                                        }
                                    }
                                    Tool::Stamp => {
                                        if let Some(prefab) = stamp_prefab {
                                            cursor_icon = Some(egui::CursorIcon::Crosshair);
                                            Self::draw_prefab_wall_overwrite_preview(
                                                &painter,
                                                map,
                                                prefab,
                                                (col, row),
                                                origin,
                                                half_w,
                                                half_h,
                                                wall_atlas,
                                                wall_texture,
                                            );
                                            Self::draw_prefab_preview(
                                                &painter,
                                                prefab,
                                                (col, row),
                                                origin,
                                                half_w,
                                                half_h,
                                                tile_atlas,
                                                atlas_texture,
                                                wall_atlas,
                                                wall_texture,
                                            );
                                            if response.clicked_by(egui::PointerButton::Primary) {
                                                result.stamp_clicked_tile = Some((col, row));
                                                result.clicked_tile = Some((col, row));
                                            }
                                        }
                                    }
                                    Tool::Eyedropper => {
                                        cursor_icon = Some(egui::CursorIcon::Crosshair);
                                        let idx = row as usize * map.width as usize + col as usize;
                                        let tile = &map.tiles[idx];
                                        let pick = Self::eyedropper_target(tile, paint_layer);

                                        Self::draw_eyedropper_target_highlight(
                                            &painter, col, row, origin, half_w, half_h, &pick,
                                            &colors,
                                        );
                                        match pick {
                                            EyedropperPick::Ground(tile_id) if tile_id != 0 => {
                                                Self::draw_ground_preview(
                                                    &painter,
                                                    col,
                                                    row,
                                                    origin,
                                                    half_w,
                                                    half_h,
                                                    tile_atlas,
                                                    atlas_texture,
                                                    tile_id,
                                                    180,
                                                );
                                            }
                                            EyedropperPick::LeftWall(wall_id) if wall_id != 0 => {
                                                Self::draw_wall_preview(
                                                    &painter,
                                                    col,
                                                    row,
                                                    origin,
                                                    half_w,
                                                    half_h,
                                                    wall_atlas,
                                                    wall_texture,
                                                    wall_id,
                                                    true,
                                                    180,
                                                );
                                                Self::draw_wall_silhouette_outline_for_tile(
                                                    &painter,
                                                    col,
                                                    row,
                                                    origin,
                                                    half_w,
                                                    half_h,
                                                    wall_atlas,
                                                    wall_id,
                                                    true,
                                                );
                                            }
                                            EyedropperPick::RightWall(wall_id) if wall_id != 0 => {
                                                Self::draw_wall_preview(
                                                    &painter,
                                                    col,
                                                    row,
                                                    origin,
                                                    half_w,
                                                    half_h,
                                                    wall_atlas,
                                                    wall_texture,
                                                    wall_id,
                                                    false,
                                                    180,
                                                );
                                                Self::draw_wall_silhouette_outline_for_tile(
                                                    &painter,
                                                    col,
                                                    row,
                                                    origin,
                                                    half_w,
                                                    half_h,
                                                    wall_atlas,
                                                    wall_id,
                                                    false,
                                                );
                                            }
                                            _ => {}
                                        }

                                        if response.clicked_by(egui::PointerButton::Primary) {
                                            result.eyedropper_pick = Some(pick);
                                            result.clicked_tile = Some((col, row));
                                        }
                                    }
                                    Tool::Eraser if is_layer_visible(layers, paint_layer) => {
                                        cursor_icon = Some(egui::CursorIcon::Crosshair);
                                        Self::draw_erase_preview(
                                            &painter, col, row, origin, half_w, half_h,
                                        );
                                        let is_primary_painting = response
                                            .is_pointer_button_down_on()
                                            && ui.input(|i| {
                                                i.pointer.button_down(egui::PointerButton::Primary)
                                            });
                                        if is_primary_painting {
                                            result.painted_tile = Some((col, row, 0));
                                        }
                                    }
                                    _ => {
                                        if response.clicked_by(egui::PointerButton::Primary) {
                                            result.clicked_tile = Some((col, row));
                                        }
                                    }
                                }
                            }
                        } else if active_tool == Tool::Select
                            && !paste_preview_active
                            && primary_pressed
                            && current_selection.is_some()
                        {
                            result.clear_selection_requested = true;
                        }
                    }
                }

                let selection_context_tile_id = ui.id().with("selection_context_tile");
                let selection_context_tile = ui
                    .ctx()
                    .data(|data| data.get_temp::<(u16, u16)>(selection_context_tile_id));
                if paste_preview_active {
                    ui.ctx().data_mut(|data| {
                        data.remove::<(u16, u16)>(selection_context_tile_id);
                    });
                }

                if let Some(preview) = selection_move_preview {
                    Self::draw_wall_overwrite_preview(
                        &painter,
                        map,
                        preview.map,
                        preview.top_left,
                        &preview.layers,
                        preview.ignore_overwrite_region,
                        origin,
                        half_w,
                        half_h,
                        wall_atlas,
                        wall_texture,
                    );
                    Self::draw_selection_move_preview(
                        &painter,
                        preview.map,
                        preview.top_left,
                        &preview.layers,
                        origin,
                        half_w,
                        half_h,
                        tile_atlas,
                        atlas_texture,
                        wall_atlas,
                        wall_texture,
                    );
                }

                if let (Some(preview), Some((col, row))) = (
                    paste_preview,
                    result
                        .hover_tile
                        .or(selection_context_tile)
                        .or(paste_preview_anchor),
                ) {
                    Self::draw_wall_overwrite_preview(
                        &painter,
                        map,
                        preview.map,
                        (col, row),
                        &preview.layers,
                        preview.ignore_overwrite_region,
                        origin,
                        half_w,
                        half_h,
                        wall_atlas,
                        wall_texture,
                    );
                    Self::draw_selection_move_preview(
                        &painter,
                        preview.map,
                        (col, row),
                        &preview.layers,
                        origin,
                        half_w,
                        half_h,
                        tile_atlas,
                        atlas_texture,
                        wall_atlas,
                        wall_texture,
                    );
                    Self::draw_tile_selection_outline(
                        &painter,
                        TileSelection::from_top_left_size(
                            (col, row),
                            preview.map.width,
                            preview.map.height,
                        ),
                        origin,
                        half_w,
                        half_h,
                        &colors,
                    );
                } else if active_tool == Tool::Select {
                    if under_wall_selection.is_none() {
                        if let Some(selection) = selection_preview.or(current_selection) {
                            Self::draw_tile_selection(
                                &painter,
                                selection,
                                origin,
                                half_w,
                                half_h,
                                &colors,
                                selection_pulse,
                            );
                        } else if let Some((col, row)) = result.hover_tile {
                            Self::draw_tile_highlight(
                                &painter, col, row, origin, half_w, half_h, &colors,
                            );
                        }
                    }
                } else if let Some((col, row)) = result.hover_tile {
                    if active_tool == Tool::Stamp {
                        if let Some(prefab) = stamp_prefab {
                            Self::draw_prefab_highlight(
                                &painter,
                                prefab,
                                (col, row),
                                origin,
                                half_w,
                                half_h,
                                &colors,
                            );
                        } else {
                            Self::draw_tile_highlight(
                                &painter, col, row, origin, half_w, half_h, &colors,
                            );
                        }
                    } else {
                        Self::draw_tile_highlight(
                            &painter, col, row, origin, half_w, half_h, &colors,
                        );
                    }
                }
                if interaction_enabled
                    && !paste_preview_active
                    && (active_tool == Tool::Select || selection_clipboard_available)
                    && response.secondary_clicked()
                {
                    let context_tile = pointer_interact_pos.and_then(|pointer_pos| {
                        if rect.contains(pointer_pos) && !overlay_rect.contains(pointer_pos) {
                            Self::screen_to_tile(pointer_pos, origin, half_w, half_h, map)
                        } else {
                            None
                        }
                    });
                    ui.ctx().data_mut(|data| {
                        if let Some(tile) = context_tile {
                            data.insert_temp(selection_context_tile_id, tile);
                        } else {
                            data.remove::<(u16, u16)>(selection_context_tile_id);
                        }
                    });
                }
                let selection_context_tile = ui
                    .ctx()
                    .data(|data| data.get_temp::<(u16, u16)>(selection_context_tile_id));
                let implicit_context_selection = selection_context_tile
                    .map(|tile| TileSelection::from_points(tile, tile))
                    .filter(|selection| Self::selection_contains_tiles(map, *selection));
                let context_selection = current_selection.or(implicit_context_selection);
                let can_create_prefab = context_selection
                    .map(|selection| Self::selection_contains_tiles(map, selection))
                    .unwrap_or(false);
                let show_selection_context_menu = interaction_enabled
                    && !paste_preview_active
                    && selection_context_tile.is_some()
                    && (active_tool == Tool::Select || selection_clipboard_available);
                if show_selection_context_menu {
                    response.context_menu(|ui| {
                        let Some(_) = selection_context_tile else {
                            return;
                        };

                        ui.set_min_width(190.0);
                        ui.add_space(2.0);

                        if active_tool == Tool::Select && context_selection.is_some() {
                            if Self::context_menu_action(
                                ui,
                                CONTEXT_ICON_CUT,
                                "Cut",
                                Some("Cmd+X"),
                                None,
                                true,
                            )
                                .clicked()
                            {
                                result.cut_selection_requested = true;
                                ui.ctx().data_mut(|data| {
                                    data.remove::<(u16, u16)>(selection_context_tile_id);
                                });
                                ui.close();
                            }
                            if Self::context_menu_action(
                                ui,
                                CONTEXT_ICON_COPY,
                                "Copy",
                                Some("Cmd+C"),
                                None,
                                true,
                            )
                                .clicked()
                            {
                                result.copy_selection_requested = true;
                                ui.ctx().data_mut(|data| {
                                    data.remove::<(u16, u16)>(selection_context_tile_id);
                                });
                                ui.close();
                            }
                        }

                        if Self::context_menu_action(
                            ui,
                            CONTEXT_ICON_PASTE,
                            "Paste",
                            Some("Cmd+V"),
                            None,
                            selection_clipboard_available,
                        )
                        .clicked()
                        {
                            result.activate_paste_preview = true;
                            ui.close();
                        }

                        if active_tool == Tool::Select && context_selection.is_some() {
                            if Self::context_menu_action(
                                ui,
                                CONTEXT_ICON_CREATE_PREFAB,
                                "Create Prefab...",
                                None,
                                None,
                                can_create_prefab,
                            )
                                .clicked()
                            {
                                result.create_prefab_requested = true;
                                ui.ctx().data_mut(|data| {
                                    data.remove::<(u16, u16)>(selection_context_tile_id);
                                });
                                ui.close();
                            }
                            ui.separator();
                            if Self::context_menu_action(
                                ui,
                                CONTEXT_ICON_DELETE,
                                "Delete",
                                Some("Backspace"),
                                Some(colors.accent),
                                true,
                            )
                            .clicked()
                            {
                                result.delete_selection_requested = true;
                                ui.ctx().data_mut(|data| {
                                    data.remove::<(u16, u16)>(selection_context_tile_id);
                                });
                                ui.close();
                            }
                        }
                    });
                }

                if is_mouse_panning {
                    cursor_icon = Some(egui::CursorIcon::Grabbing);
                }
                if let Some(cursor_icon) = cursor_icon {
                    ui.ctx().set_cursor_icon(cursor_icon);
                }

                // Floating overlay: Grid | Tab | G L R
                Self::draw_overlay(
                    ui,
                    rect,
                    show_grid,
                    show_collision_overlay,
                    sotp.is_some(),
                    layers,
                    &colors,
                );
            });

        result
    }

    /// Draw ground tiles and wall sprites in isometric depth order.
    ///
    /// Iterates tiles by increasing `row + col` (back-to-front). For each tile
    /// the draw order is: ground, left wall, right wall. This ensures that tall
    /// wall sprites on near tiles correctly occlude sprites on far tiles.
    ///
    /// Ground tiles sharing the same atlas are batched into one mesh per depth
    /// band, and walls likewise, so we get reasonable draw-call counts while
    /// maintaining correct depth ordering.
    #[allow(clippy::too_many_arguments)]
    fn draw_scene(
        painter: &egui::Painter,
        map: &map::Map,
        tile_atlas: Option<&render::TileAtlas>,
        tile_texture: Option<&egui::TextureHandle>,
        wall_atlas: Option<&render::SpriteAtlas>,
        wall_texture: Option<&egui::TextureHandle>,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
        viewport: egui::Rect,
        layers: &LayerVisibility,
        wall_outline_selection: Option<TileSelection>,
        selection_pulse: f32,
    ) {
        let tw = half_w * 2.0;
        let th = half_h * 2.0;
        let zoom = half_w / (map::TILE_WIDTH / 2.0);

        let width = map.width as u32;
        let height = map.height as u32;
        let max_depth = width + height;
        let mut wall_outline_sprites: Vec<WallOutlineSprite> = Vec::new();
        let mut wall_meshes: Vec<egui::Mesh> = Vec::new();

        for depth in 0..max_depth {
            // All (col, row) pairs with col + row == depth, within map bounds.
            let row_min = depth.saturating_sub(width.saturating_sub(1));
            let row_max = depth.min(height.saturating_sub(1));

            // Batch ground tiles for this depth band.
            let mut ground_mesh = tile_texture
                .as_ref()
                .map(|t| egui::Mesh::with_texture(t.id()));

            // Batch wall sprites for this depth band.
            let mut wall_mesh = wall_texture
                .as_ref()
                .map(|t| egui::Mesh::with_texture(t.id()));

            for row in row_min..=row_max {
                let col = depth - row;
                let tile = &map.tiles[row as usize * width as usize + col as usize];
                let outline_walls = wall_outline_selection
                    .map(|selection| selection.contains((col as u16, row as u16)))
                    .unwrap_or(false);

                let cx = origin.x + (col as f32 - row as f32) * half_w;
                let cy = origin.y + (col as f32 + row as f32) * half_h;

                // --- Ground ---
                if layers.ground && tile.ground != 0 {
                    if let (Some(atlas), Some(mesh)) = (tile_atlas, ground_mesh.as_mut()) {
                        let atlas_index = (tile.ground - 1) as u32;
                        let tile_rect = egui::Rect::from_min_size(
                            egui::pos2(cx - half_w, cy),
                            egui::vec2(tw, th),
                        );
                        if viewport.intersects(tile_rect) {
                            if let Some((u0, v0, u1, v1)) = atlas.tile_uv(atlas_index) {
                                let uv = egui::Rect::from_min_max(
                                    egui::pos2(u0, v0),
                                    egui::pos2(u1, v1),
                                );
                                mesh.add_rect_with_uv(tile_rect, uv, egui::Color32::WHITE);
                            }
                        }
                    }
                }

                let bottom_y = cy + 2.0 * half_h;

                // --- Left wall ---
                if layers.left_wall && is_rendered_wall(tile.left_wall) {
                    if let (Some(atlas), Some(mesh)) = (wall_atlas, wall_mesh.as_mut()) {
                        let idx = tile.left_wall as u32;
                        let sh = atlas.sprite_height(idx);
                        if sh > 0 {
                            let screen_h = sh as f32 * zoom;
                            let sprite_rect = egui::Rect::from_min_max(
                                egui::pos2(cx - half_w, bottom_y - screen_h),
                                egui::pos2(cx, bottom_y),
                            );
                            if viewport.intersects(sprite_rect) {
                                if let Some((u0, v0, u1, v1)) = atlas.sprite_uv(idx) {
                                    let uv = egui::Rect::from_min_max(
                                        egui::pos2(u0, v0),
                                        egui::pos2(u1, v1),
                                    );
                                    mesh.add_rect_with_uv(sprite_rect, uv, egui::Color32::WHITE);
                                    if outline_walls {
                                        if let Some(source_rect) = atlas.sprite_rect(idx) {
                                            wall_outline_sprites.push(WallOutlineSprite {
                                                source_rect,
                                                screen_rect: sprite_rect,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // --- Right wall ---
                if layers.right_wall && is_rendered_wall(tile.right_wall) {
                    if let (Some(atlas), Some(mesh)) = (wall_atlas, wall_mesh.as_mut()) {
                        let idx = tile.right_wall as u32;
                        let sh = atlas.sprite_height(idx);
                        if sh > 0 {
                            let screen_h = sh as f32 * zoom;
                            let sprite_rect = egui::Rect::from_min_max(
                                egui::pos2(cx, bottom_y - screen_h),
                                egui::pos2(cx + half_w, bottom_y),
                            );
                            if viewport.intersects(sprite_rect) {
                                if let Some((u0, v0, u1, v1)) = atlas.sprite_uv(idx) {
                                    let uv = egui::Rect::from_min_max(
                                        egui::pos2(u0, v0),
                                        egui::pos2(u1, v1),
                                    );
                                    mesh.add_rect_with_uv(sprite_rect, uv, egui::Color32::WHITE);
                                    if outline_walls {
                                        if let Some(source_rect) = atlas.sprite_rect(idx) {
                                            wall_outline_sprites.push(WallOutlineSprite {
                                                source_rect,
                                                screen_rect: sprite_rect,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Flush this depth band: ground first, then walls on top.
            if let Some(mesh) = ground_mesh {
                if !mesh.is_empty() {
                    painter.add(egui::Shape::mesh(mesh));
                }
            }
            if let Some(mesh) = wall_mesh {
                if !mesh.is_empty() {
                    wall_meshes.push(mesh);
                }
            }
        }
        if let Some(selection) = wall_outline_selection {
            Self::draw_tile_selection(
                painter,
                selection,
                origin,
                half_w,
                half_h,
                &theme_colors(),
                selection_pulse,
            );
        }
        for mesh in wall_meshes {
            painter.add(egui::Shape::mesh(mesh));
        }
        if let Some(atlas) = wall_atlas {
            Self::draw_merged_sprite_silhouette_outline(painter, atlas, &wall_outline_sprites);
        }
    }

    fn draw_overlay(
        ui: &mut egui::Ui,
        viewport_rect: egui::Rect,
        show_grid: &mut bool,
        show_collision_overlay: &mut bool,
        collision_available: bool,
        layers: &mut LayerVisibility,
        colors: &ThemeColors,
    ) {
        let btn_h = 24.0;
        let grid_w = 42.0;
        let tab_w = 42.0;
        let layer_w = 26.0;
        let sep_w = 1.0;
        let pad = 6.0;
        let gap = 4.0;

        let overlay_rect = Self::overlay_rect(viewport_rect);

        let btn_y = overlay_rect.top() + pad;
        let mut x = overlay_rect.left() + pad;

        // Compute all rects first
        let grid_rect = egui::Rect::from_min_size(egui::pos2(x, btn_y), egui::vec2(grid_w, btn_h));
        x += grid_w + gap;

        let tab_rect = egui::Rect::from_min_size(egui::pos2(x, btn_y), egui::vec2(tab_w, btn_h));
        x += tab_w + gap;

        let sep_rect =
            egui::Rect::from_min_size(egui::pos2(x, btn_y + 2.0), egui::vec2(sep_w, btn_h - 4.0));
        x += sep_w + gap;

        let ground_rect =
            egui::Rect::from_min_size(egui::pos2(x, btn_y), egui::vec2(layer_w, btn_h));
        x += layer_w + gap;

        let left_rect = egui::Rect::from_min_size(egui::pos2(x, btn_y), egui::vec2(layer_w, btn_h));
        x += layer_w + gap;

        let right_rect =
            egui::Rect::from_min_size(egui::pos2(x, btn_y), egui::vec2(layer_w, btn_h));

        // Paint background and separator first (each painter() borrow is temporary)
        ui.painter().rect(
            overlay_rect,
            6.0,
            egui::Color32::from_rgba_unmultiplied(18, 20, 24, 200),
            egui::Stroke::new(1.0, colors.border),
            egui::StrokeKind::Inside,
        );
        ui.painter().rect_filled(sep_rect, 0.0, colors.border);

        // Then do interactions + paint buttons on top
        Self::overlay_toggle(
            ui,
            "Grid",
            grid_rect,
            show_grid,
            true,
            colors,
            "Grid (Cmd+4)",
        );
        Self::overlay_toggle(
            ui,
            "Tab",
            tab_rect,
            show_collision_overlay,
            collision_available,
            colors,
            if collision_available {
                "Tab collision overlay (Tab)"
            } else {
                "Tab collision overlay (unavailable: SOTP.DAT missing)"
            },
        );
        Self::overlay_toggle(
            ui,
            "G",
            ground_rect,
            &mut layers.ground,
            true,
            colors,
            "Ground (Cmd+1)",
        );
        Self::overlay_toggle(
            ui,
            "L",
            left_rect,
            &mut layers.left_wall,
            true,
            colors,
            "Left Wall (Cmd+2)",
        );
        Self::overlay_toggle(
            ui,
            "R",
            right_rect,
            &mut layers.right_wall,
            true,
            colors,
            "Right Wall (Cmd+3)",
        );
    }

    fn overlay_toggle(
        ui: &mut egui::Ui,
        label: &str,
        rect: egui::Rect,
        enabled: &mut bool,
        interactive: bool,
        colors: &ThemeColors,
        tooltip: &str,
    ) {
        let id = ui.id().with(("overlay_toggle", label));
        let sense = if interactive {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        };
        let response = ui.interact(rect, id, sense);

        // Use opaque backgrounds so the semi-transparent panel doesn't bleed through
        let bg = if !interactive {
            egui::Color32::from_rgb(16, 18, 22)
        } else if *enabled {
            colors.accent.gamma_multiply(0.2)
        } else if response.hovered() {
            colors.panel_2
        } else {
            egui::Color32::from_rgb(18, 20, 24)
        };

        let border = if !interactive {
            egui::Stroke::new(1.0, colors.border.gamma_multiply(0.35))
        } else if *enabled {
            egui::Stroke::new(1.0, colors.accent)
        } else if response.hovered() {
            egui::Stroke::new(1.0, colors.border)
        } else {
            egui::Stroke::NONE
        };

        ui.painter()
            .rect(rect, 4.0, bg, border, egui::StrokeKind::Inside);

        let text_color = if !interactive {
            colors.border.gamma_multiply(0.8)
        } else if *enabled {
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

        if interactive && response.clicked() {
            *enabled = !*enabled;
        }

        let _ = tooltip::attach(response, tooltip);
    }

    fn overlay_rect(viewport_rect: egui::Rect) -> egui::Rect {
        let btn_h = 24.0;
        let grid_w = 42.0;
        let tab_w = 42.0;
        let layer_w = 26.0;
        let sep_w = 1.0;
        let pad = 6.0;
        let gap = 4.0;
        let total_w =
            pad + grid_w + gap + tab_w + gap + sep_w + gap + layer_w * 3.0 + gap * 2.0 + pad;
        let total_h = pad * 2.0 + btn_h;

        egui::Rect::from_min_size(
            egui::pos2(
                viewport_rect.right() - total_w - 12.0,
                viewport_rect.top() + 12.0,
            ),
            egui::vec2(total_w, total_h),
        )
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

    fn draw_collision_overlay(
        painter: &egui::Painter,
        map: &map::Map,
        sotp: &[u8],
        checker_texture: Option<&egui::TextureHandle>,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
        viewport: egui::Rect,
    ) {
        let w = map.width as usize;
        let h = map.height as usize;
        if w == 0 || h == 0 {
            return;
        }

        let solid = Self::build_solid_grid(map, sotp);
        let checker_repeat = 2.0;
        let mut dither_mesh = checker_texture.map(|texture| egui::Mesh::with_texture(texture.id()));
        let edge_stroke = egui::Stroke::new(
            2.0,
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 230),
        );

        let is_solid = |col: i32, row: i32| -> bool {
            if col < 0 || col >= w as i32 || row < 0 || row >= h as i32 {
                return false;
            }
            solid[row as usize * w + col as usize]
        };

        for row in 0..h {
            for col in 0..w {
                if !solid[row * w + col] {
                    continue;
                }

                let cx = origin.x + (col as f32 - row as f32) * half_w;
                let cy = origin.y + (col as f32 + row as f32) * half_h;

                let tile_rect = egui::Rect::from_min_max(
                    egui::pos2(cx - half_w, cy),
                    egui::pos2(cx + half_w, cy + 2.0 * half_h),
                );
                if !viewport.intersects(tile_rect) {
                    continue;
                }

                let top = egui::pos2(cx, cy);
                let right = egui::pos2(cx + half_w, cy + half_h);
                let bottom = egui::pos2(cx, cy + 2.0 * half_h);
                let left = egui::pos2(cx - half_w, cy + half_h);

                if let Some(mesh) = dither_mesh.as_mut() {
                    let base = mesh.vertices.len() as u32;
                    let uv_top = egui::pos2(top.x / checker_repeat, top.y / checker_repeat);
                    let uv_right = egui::pos2(right.x / checker_repeat, right.y / checker_repeat);
                    let uv_bottom =
                        egui::pos2(bottom.x / checker_repeat, bottom.y / checker_repeat);
                    let uv_left = egui::pos2(left.x / checker_repeat, left.y / checker_repeat);

                    mesh.vertices.push(egui::epaint::Vertex {
                        pos: top,
                        uv: uv_top,
                        color: egui::Color32::WHITE,
                    });
                    mesh.vertices.push(egui::epaint::Vertex {
                        pos: right,
                        uv: uv_right,
                        color: egui::Color32::WHITE,
                    });
                    mesh.vertices.push(egui::epaint::Vertex {
                        pos: bottom,
                        uv: uv_bottom,
                        color: egui::Color32::WHITE,
                    });
                    mesh.vertices.push(egui::epaint::Vertex {
                        pos: left,
                        uv: uv_left,
                        color: egui::Color32::WHITE,
                    });
                    mesh.indices.extend_from_slice(&[
                        base,
                        base + 1,
                        base + 2,
                        base,
                        base + 2,
                        base + 3,
                    ]);
                }
            }
        }

        if let Some(mesh) = dither_mesh {
            if !mesh.is_empty() {
                painter.add(egui::Shape::mesh(mesh));
            }
        }

        // Draw boundary edges in a second pass so they remain visible on top
        // of the dither fill.
        for row in 0..h {
            for col in 0..w {
                if !solid[row * w + col] {
                    continue;
                }

                let cx = origin.x + (col as f32 - row as f32) * half_w;
                let cy = origin.y + (col as f32 + row as f32) * half_h;
                let tile_rect = egui::Rect::from_min_max(
                    egui::pos2(cx - half_w, cy),
                    egui::pos2(cx + half_w, cy + 2.0 * half_h),
                );
                if !viewport.intersects(tile_rect) {
                    continue;
                }

                let top = egui::pos2(cx, cy);
                let right = egui::pos2(cx + half_w, cy + half_h);
                let bottom = egui::pos2(cx, cy + 2.0 * half_h);
                let left = egui::pos2(cx - half_w, cy + half_h);

                let c = col as i32;
                let r = row as i32;
                if !is_solid(c, r - 1) {
                    painter.line_segment([top, right], edge_stroke);
                }
                if !is_solid(c + 1, r) {
                    painter.line_segment([right, bottom], edge_stroke);
                }
                if !is_solid(c, r + 1) {
                    painter.line_segment([bottom, left], edge_stroke);
                }
                if !is_solid(c - 1, r) {
                    painter.line_segment([left, top], edge_stroke);
                }
            }
        }
    }

    /// Build a flat bool grid indicating whether each tile is solid (impassable).
    /// A tile is solid if `sotp[left_wall - 1] & 0xF == 0xF` or
    /// `sotp[right_wall - 1] & 0xF == 0xF`.
    fn build_solid_grid(map: &map::Map, sotp: &[u8]) -> Vec<bool> {
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

    fn screen_to_tile_clamped(
        screen_pos: egui::Pos2,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
        map: &map::Map,
    ) -> Option<(u16, u16)> {
        if map.width == 0 || map.height == 0 {
            return None;
        }

        let dx = screen_pos.x - origin.x;
        let dy = screen_pos.y - origin.y;

        let col = ((dx / half_w + dy / half_h) / 2.0).floor() as i32;
        let row = ((dy / half_h - dx / half_w) / 2.0).floor() as i32;
        let max_col = i32::from(map.width.saturating_sub(1));
        let max_row = i32::from(map.height.saturating_sub(1));

        Some((col.clamp(0, max_col) as u16, row.clamp(0, max_row) as u16))
    }

    fn selected_tile_at_screen_pos(
        selection: TileSelection,
        screen_pos: egui::Pos2,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
    ) -> Option<(u16, u16)> {
        let (min_col, min_row, max_col, max_row) = selection.normalized_bounds();
        for row in min_row..=max_row {
            for col in min_col..=max_col {
                if !selection.contains((col, row)) {
                    continue;
                }
                if Self::point_in_tile_diamond(screen_pos, col, row, origin, half_w, half_h) {
                    return Some((col, row));
                }
            }
        }
        None
    }

    fn point_in_tile_diamond(
        screen_pos: egui::Pos2,
        col: u16,
        row: u16,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
    ) -> bool {
        let cx = origin.x + (col as f32 - row as f32) * half_w;
        let cy = origin.y + (col as f32 + row as f32) * half_h;
        let dx = (screen_pos.x - cx).abs();
        let dy = (screen_pos.y - cy).abs();
        (dx / half_w) + (dy / half_h) <= 1.0
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

    fn draw_tile_selection(
        painter: &egui::Painter,
        selection: TileSelection,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
        colors: &ThemeColors,
        pulse_opacity: f32,
    ) {
        let (min_col, min_row, max_col, max_row) = selection.normalized_bounds();
        let fill = colors.accent.gamma_multiply(0.08 + 0.08 * pulse_opacity);
        let stroke =
            egui::Stroke::new(1.5, colors.accent.gamma_multiply(0.7 + 0.3 * pulse_opacity));

        for row in min_row..=max_row {
            for col in min_col..=max_col {
                let (top, right, bottom, left) = Self::tile_diamond_points(
                    i32::from(col),
                    i32::from(row),
                    origin,
                    half_w,
                    half_h,
                );
                painter.add(egui::Shape::convex_polygon(
                    vec![top, right, bottom, left],
                    fill,
                    egui::Stroke::NONE,
                ));
            }
        }

        let (top, _, _, left_top) = Self::tile_diamond_points(
            i32::from(min_col),
            i32::from(min_row),
            origin,
            half_w,
            half_h,
        );
        let (top_right, right_top, _, _) = Self::tile_diamond_points(
            i32::from(max_col),
            i32::from(min_row),
            origin,
            half_w,
            half_h,
        );
        let (_, right_bottom, bottom, _) = Self::tile_diamond_points(
            i32::from(max_col),
            i32::from(max_row),
            origin,
            half_w,
            half_h,
        );
        let (_, _, bottom_left, left_bottom) = Self::tile_diamond_points(
            i32::from(min_col),
            i32::from(max_row),
            origin,
            half_w,
            half_h,
        );

        let mut outline = vec![
            top,
            top_right,
            right_top,
            right_bottom,
            bottom,
            bottom_left,
            left_bottom,
            left_top,
        ];
        outline.dedup_by(|a, b| a.distance_sq(*b) < 0.01);
        painter.add(egui::Shape::closed_line(outline, stroke));
    }

    fn selection_contains_tiles(map: &map::Map, selection: TileSelection) -> bool {
        let (min_col, min_row, max_col, max_row) = selection.normalized_bounds();
        for row in min_row..=max_row {
            for col in min_col..=max_col {
                let idx = row as usize * map.width as usize + col as usize;
                let tile = map.tiles[idx];
                if tile.ground != 0 || tile.left_wall != 0 || tile.right_wall != 0 {
                    return true;
                }
            }
        }
        false
    }

    fn draw_tile_selection_outline(
        painter: &egui::Painter,
        selection: TileSelection,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
        colors: &ThemeColors,
    ) {
        let (min_col, min_row, max_col, max_row) = selection.normalized_bounds();
        let (top, _, _, left_top) = Self::tile_diamond_points(
            i32::from(min_col),
            i32::from(min_row),
            origin,
            half_w,
            half_h,
        );
        let (top_right, right_top, _, _) = Self::tile_diamond_points(
            i32::from(max_col),
            i32::from(min_row),
            origin,
            half_w,
            half_h,
        );
        let (_, right_bottom, bottom, _) = Self::tile_diamond_points(
            i32::from(max_col),
            i32::from(max_row),
            origin,
            half_w,
            half_h,
        );
        let (_, _, bottom_left, left_bottom) = Self::tile_diamond_points(
            i32::from(min_col),
            i32::from(max_row),
            origin,
            half_w,
            half_h,
        );

        let mut outline = vec![
            top,
            top_right,
            right_top,
            right_bottom,
            bottom,
            bottom_left,
            left_bottom,
            left_top,
        ];
        outline.dedup_by(|a, b| a.distance_sq(*b) < 0.01);

        let stroke = egui::Stroke::new(1.5, colors.accent);
        painter.add(egui::Shape::closed_line(outline, stroke));
    }

    fn context_menu_action(
        ui: &mut egui::Ui,
        icon: &str,
        label: &str,
        hotkey: Option<&str>,
        color: Option<egui::Color32>,
        enabled: bool,
    ) -> egui::Response {
        const MENU_LEFT_PADDING: f32 = 8.0;
        const MENU_ICON_WIDTH: f32 = 18.0;
        const MENU_MIN_WIDTH: f32 = 276.0;
        const MENU_ICON_SIZE: f32 = 14.0;
        const MENU_HOTKEY_FONT_SIZE: f32 = 12.0;

        let colors = theme_colors();
        let text_color = color.unwrap_or(colors.text);
        let icon_color = if enabled { text_color } else { colors.muted };
        let label_color = if enabled { text_color } else { colors.muted };
        let hotkey_color = if enabled { colors.accent } else { colors.muted };
        let row_height = ui.spacing().interact_size.y;
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(MENU_MIN_WIDTH, row_height),
            if enabled {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            },
        );

        if response.hovered() {
            ui.painter().rect_filled(
                rect,
                2.0,
                ui.style().visuals.widgets.hovered.bg_fill,
            );
        }

        let icon_x = rect.left() + MENU_LEFT_PADDING;
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(icon_x, rect.top()),
            egui::vec2(MENU_ICON_WIDTH, row_height),
        );
        ui.painter().text(
            egui::pos2(icon_rect.left(), icon_rect.center().y),
            egui::Align2::LEFT_CENTER,
            icon,
            icons::symbol_icon_font_id(MENU_ICON_SIZE),
            icon_color,
        );

        let label_x = icon_rect.right() + 10.0;
        ui.painter().text(
            egui::pos2(label_x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::TextStyle::Button.resolve(ui.style()),
            label_color,
        );

        if let Some(hotkey) = hotkey {
            let hotkey_x = rect.left() + MENU_MIN_WIDTH - 8.0;
            ui.painter().text(
                egui::pos2(hotkey_x, rect.center().y),
                egui::Align2::RIGHT_CENTER,
                format!("[{hotkey}]"),
                egui::FontId::new(MENU_HOTKEY_FONT_SIZE, egui::FontFamily::Monospace),
                hotkey_color,
            );
        }
        response
    }

    fn draw_prefab_highlight(
        painter: &egui::Painter,
        prefab: &map::Map,
        origin_tile: (u16, u16),
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
        colors: &ThemeColors,
    ) {
        let Some((min_col, min_row, max_col, max_row)) =
            Self::prefab_highlight_bounds(prefab, origin_tile)
        else {
            Self::draw_tile_highlight(
                painter,
                origin_tile.0,
                origin_tile.1,
                origin,
                half_w,
                half_h,
                colors,
            );
            return;
        };

        let (top, _, _, left_top) =
            Self::tile_diamond_points(min_col, min_row, origin, half_w, half_h);
        let (top_right, right_top, _, _) =
            Self::tile_diamond_points(max_col, min_row, origin, half_w, half_h);
        let (_, right_bottom, bottom, _) =
            Self::tile_diamond_points(max_col, max_row, origin, half_w, half_h);
        let (_, _, bottom_left, left_bottom) =
            Self::tile_diamond_points(min_col, max_row, origin, half_w, half_h);

        let mut outline = vec![
            top,
            top_right,
            right_top,
            right_bottom,
            bottom,
            bottom_left,
            left_bottom,
            left_top,
        ];
        outline.dedup_by(|a, b| a.distance_sq(*b) < 0.01);

        let stroke = egui::Stroke::new(1.5, colors.accent);
        painter.add(egui::Shape::closed_line(outline, stroke));
    }

    fn prefab_highlight_bounds(
        prefab: &map::Map,
        origin_tile: (u16, u16),
    ) -> Option<(i32, i32, i32, i32)> {
        let bounds = prefab::occupied_bounds(prefab)?;
        let anchor = prefab::placement_anchor(prefab);
        Some((
            i32::from(origin_tile.0) + i32::from(bounds.min_col) - i32::from(anchor.0),
            i32::from(origin_tile.1) + i32::from(bounds.min_row) - i32::from(anchor.1),
            i32::from(origin_tile.0) + i32::from(bounds.max_col) - i32::from(anchor.0),
            i32::from(origin_tile.1) + i32::from(bounds.max_row) - i32::from(anchor.1),
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn under_wall_selection(
        viewport_hovered: bool,
        map: &map::Map,
        active_tool: Tool,
        paste_preview_active: bool,
        current_selection: Option<TileSelection>,
        selection_move_active: bool,
        selection_drag_active: bool,
        selection_drag_start_tile: Option<(u16, u16)>,
        pointer_interact_pos: Option<egui::Pos2>,
        rect: egui::Rect,
        overlay_rect: egui::Rect,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
    ) -> Option<TileSelection> {
        if active_tool != Tool::Select || paste_preview_active {
            return None;
        }
        if let Some(selection) = current_selection {
            return Some(selection);
        }

        let pointer_pos = if selection_drag_active {
            pointer_interact_pos.map(|pos| Self::clamp_point_to_rect(pos, rect.shrink(1.0)))
        } else if viewport_hovered {
            pointer_interact_pos
        } else {
            None
        }?;

        if !selection_drag_active && overlay_rect.contains(pointer_pos) {
            return None;
        }

        let hover_tile = if selection_drag_active {
            Self::screen_to_tile_clamped(pointer_pos, origin, half_w, half_h, map)
        } else {
            Self::screen_to_tile(pointer_pos, origin, half_w, half_h, map)
        }?;

        if selection_drag_active && !selection_move_active {
            if let Some(start) = selection_drag_start_tile {
                return Some(TileSelection::from_points(start, hover_tile));
            }
        }

        Some(TileSelection::from_points(hover_tile, hover_tile))
    }

    fn selection_pulse_opacity(ctx: &egui::Context) -> f32 {
        let time = ctx.input(|i| i.time) as f32;
        0.55 + 0.45 * ((time * 2.0).sin() * 0.5 + 0.5)
    }

    fn wall_outline_sprite_for_tile(
        col: u16,
        row: u16,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
        atlas: &render::SpriteAtlas,
        wall_id: u16,
        is_left_wall: bool,
    ) -> Option<WallOutlineSprite> {
        if !is_rendered_wall(wall_id) {
            return None;
        }
        let idx = wall_id as u32;
        let sprite_h = atlas.sprite_height(idx);
        if sprite_h == 0 {
            return None;
        }
        let source_rect = atlas.sprite_rect(idx)?;

        let zoom = half_w / (map::TILE_WIDTH / 2.0);
        let screen_h = sprite_h as f32 * zoom;
        let cx = origin.x + (col as f32 - row as f32) * half_w;
        let cy = origin.y + (col as f32 + row as f32) * half_h;
        let bottom_y = cy + 2.0 * half_h;
        let screen_rect = if is_left_wall {
            egui::Rect::from_min_max(
                egui::pos2(cx - half_w, bottom_y - screen_h),
                egui::pos2(cx, bottom_y),
            )
        } else {
            egui::Rect::from_min_max(
                egui::pos2(cx, bottom_y - screen_h),
                egui::pos2(cx + half_w, bottom_y),
            )
        };

        Some(WallOutlineSprite {
            source_rect,
            screen_rect,
        })
    }

    fn draw_wall_silhouette_outline_for_tile(
        painter: &egui::Painter,
        col: u16,
        row: u16,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
        wall_atlas: Option<&render::SpriteAtlas>,
        wall_id: u16,
        is_left_wall: bool,
    ) {
        let Some(atlas) = wall_atlas else {
            return;
        };
        let Some(sprite) =
            Self::wall_outline_sprite_for_tile(col, row, origin, half_w, half_h, atlas, wall_id, is_left_wall)
        else {
            return;
        };
        Self::draw_merged_sprite_silhouette_outline(painter, atlas, &[sprite]);
    }

    fn draw_merged_sprite_silhouette_outline(
        painter: &egui::Painter,
        atlas: &render::SpriteAtlas,
        sprites: &[WallOutlineSprite],
    ) {
        if sprites.is_empty() {
            return;
        }
        let (atlas_width, atlas_height) = atlas.dimensions();
        if atlas_width == 0 || atlas_height == 0 {
            return;
        }
        let pixels = atlas.pixels();
        let min_x = sprites
            .iter()
            .map(|sprite| sprite.screen_rect.left().floor() as i32)
            .min()
            .unwrap_or(0);
        let min_y = sprites
            .iter()
            .map(|sprite| sprite.screen_rect.top().floor() as i32)
            .min()
            .unwrap_or(0);
        let max_x = sprites
            .iter()
            .map(|sprite| sprite.screen_rect.right().ceil() as i32)
            .max()
            .unwrap_or(min_x);
        let max_y = sprites
            .iter()
            .map(|sprite| sprite.screen_rect.bottom().ceil() as i32)
            .max()
            .unwrap_or(min_y);
        let mask_width = (max_x - min_x).max(1) as usize;
        let mask_height = (max_y - min_y).max(1) as usize;
        if mask_width.saturating_mul(mask_height) > 4_000_000 {
            return;
        }
        let mut mask = vec![false; mask_width * mask_height];
        let stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 220));

        let alpha_at = |src_x: u32, src_y: u32, src_w: u32, src_h: u32, px: i32, py: i32| -> u8 {
            if px < 0 || py < 0 || px >= src_w as i32 || py >= src_h as i32 {
                return 0;
            }
            let atlas_px = src_x + px as u32;
            let atlas_py = src_y + py as u32;
            if atlas_px >= atlas_width || atlas_py >= atlas_height {
                return 0;
            }
            let idx = ((atlas_py * atlas_width + atlas_px) * 4 + 3) as usize;
            pixels.get(idx).copied().unwrap_or(0)
        };

        for sprite in sprites {
            let (src_x, src_y, src_w, src_h) = sprite.source_rect;
            if src_w == 0 || src_h == 0 {
                continue;
            }
            let scale_x = sprite.screen_rect.width() / src_w as f32;
            let scale_y = sprite.screen_rect.height() / src_h as f32;
            for py in 0..src_h as i32 {
                for px in 0..src_w as i32 {
                    if alpha_at(src_x, src_y, src_w, src_h, px, py) == 0 {
                        continue;
                    }

                    let x0 = (sprite.screen_rect.left() + px as f32 * scale_x).floor() as i32 - min_x;
                    let x1 =
                        (sprite.screen_rect.left() + (px + 1) as f32 * scale_x).ceil() as i32 - min_x;
                    let y0 = (sprite.screen_rect.top() + py as f32 * scale_y).floor() as i32 - min_y;
                    let y1 =
                        (sprite.screen_rect.top() + (py + 1) as f32 * scale_y).ceil() as i32 - min_y;

                    let x0 = x0.clamp(0, mask_width as i32);
                    let x1 = x1.clamp(0, mask_width as i32);
                    let y0 = y0.clamp(0, mask_height as i32);
                    let y1 = y1.clamp(0, mask_height as i32);
                    for sy in y0..y1 {
                        for sx in x0..x1 {
                            mask[sy as usize * mask_width + sx as usize] = true;
                        }
                    }
                }
            }
        }

        let mask_at = |x: i32, y: i32| -> bool {
            if x < 0 || y < 0 || x >= mask_width as i32 || y >= mask_height as i32 {
                return false;
            }
            mask[y as usize * mask_width + x as usize]
        };

        for y in 0..mask_height as i32 {
            for x in 0..mask_width as i32 {
                if !mask_at(x, y) {
                    continue;
                }
                let x0 = (min_x + x) as f32;
                let x1 = x0 + 1.0;
                let y0 = (min_y + y) as f32;
                let y1 = y0 + 1.0;
                if !mask_at(x, y - 1) {
                    painter.line_segment([egui::pos2(x0, y0), egui::pos2(x1, y0)], stroke);
                }
                if !mask_at(x + 1, y) {
                    painter.line_segment([egui::pos2(x1, y0), egui::pos2(x1, y1)], stroke);
                }
                if !mask_at(x, y + 1) {
                    painter.line_segment([egui::pos2(x0, y1), egui::pos2(x1, y1)], stroke);
                }
                if !mask_at(x - 1, y) {
                    painter.line_segment([egui::pos2(x0, y0), egui::pos2(x0, y1)], stroke);
                }
            }
        }
    }

    fn tile_diamond_points(
        col: i32,
        row: i32,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
    ) -> (egui::Pos2, egui::Pos2, egui::Pos2, egui::Pos2) {
        let cx = origin.x + (col as f32 - row as f32) * half_w;
        let cy = origin.y + (col as f32 + row as f32) * half_h;

        (
            egui::pos2(cx, cy),
            egui::pos2(cx + half_w, cy + half_h),
            egui::pos2(cx, cy + half_h * 2.0),
            egui::pos2(cx - half_w, cy + half_h),
        )
    }

    fn selection_auto_pan_delta(
        viewport_rect: egui::Rect,
        pointer_pos: egui::Pos2,
        dt: f32,
    ) -> egui::Vec2 {
        let left_zone = viewport_rect.left() + SELECTION_AUTO_PAN_MARGIN;
        let right_zone = viewport_rect.right() - SELECTION_AUTO_PAN_MARGIN;
        let top_zone = viewport_rect.top() + SELECTION_AUTO_PAN_MARGIN;
        let bottom_zone = viewport_rect.bottom() - SELECTION_AUTO_PAN_MARGIN;

        let x = if pointer_pos.x < left_zone {
            -((left_zone - pointer_pos.x) / SELECTION_AUTO_PAN_MARGIN).clamp(0.0, 1.0)
        } else if pointer_pos.x > right_zone {
            ((pointer_pos.x - right_zone) / SELECTION_AUTO_PAN_MARGIN).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let y = if pointer_pos.y < top_zone {
            -((top_zone - pointer_pos.y) / SELECTION_AUTO_PAN_MARGIN).clamp(0.0, 1.0)
        } else if pointer_pos.y > bottom_zone {
            ((pointer_pos.y - bottom_zone) / SELECTION_AUTO_PAN_MARGIN).clamp(0.0, 1.0)
        } else {
            0.0
        };

        egui::vec2(x, y) * (SELECTION_AUTO_PAN_SPEED * dt)
    }

    fn clamp_point_to_rect(point: egui::Pos2, rect: egui::Rect) -> egui::Pos2 {
        egui::pos2(
            point.x.clamp(rect.left(), rect.right()),
            point.y.clamp(rect.top(), rect.bottom()),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_paint_preview(
        painter: &egui::Painter,
        col: u16,
        row: u16,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
        paint_layer: PaintLayer,
        paint_value: u16,
        tile_atlas: Option<&render::TileAtlas>,
        tile_texture: Option<&egui::TextureHandle>,
        wall_atlas: Option<&render::SpriteAtlas>,
        wall_texture: Option<&egui::TextureHandle>,
    ) {
        Self::draw_paint_preview_alpha(
            painter,
            col,
            row,
            origin,
            half_w,
            half_h,
            paint_layer,
            paint_value,
            tile_atlas,
            tile_texture,
            wall_atlas,
            wall_texture,
            180,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_paint_preview_alpha(
        painter: &egui::Painter,
        col: u16,
        row: u16,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
        paint_layer: PaintLayer,
        paint_value: u16,
        tile_atlas: Option<&render::TileAtlas>,
        tile_texture: Option<&egui::TextureHandle>,
        wall_atlas: Option<&render::SpriteAtlas>,
        wall_texture: Option<&egui::TextureHandle>,
        alpha: u8,
    ) {
        match paint_layer {
            PaintLayer::Ground => Self::draw_ground_preview(
                painter,
                col,
                row,
                origin,
                half_w,
                half_h,
                tile_atlas,
                tile_texture,
                paint_value,
                alpha,
            ),
            PaintLayer::LeftWall => Self::draw_wall_preview(
                painter,
                col,
                row,
                origin,
                half_w,
                half_h,
                wall_atlas,
                wall_texture,
                paint_value,
                true,
                alpha,
            ),
            PaintLayer::RightWall => Self::draw_wall_preview(
                painter,
                col,
                row,
                origin,
                half_w,
                half_h,
                wall_atlas,
                wall_texture,
                paint_value,
                false,
                alpha,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_ground_preview(
        painter: &egui::Painter,
        col: u16,
        row: u16,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
        tile_atlas: Option<&render::TileAtlas>,
        tile_texture: Option<&egui::TextureHandle>,
        selected_ground_tile: u16,
        alpha: u8,
    ) {
        let (atlas, texture) = match (tile_atlas, tile_texture) {
            (Some(a), Some(t)) => (a, t),
            _ => return,
        };

        let atlas_index = selected_ground_tile.saturating_sub(1) as u32;
        let (u0, v0, u1, v1) = match atlas.tile_uv(atlas_index) {
            Some(uv) => uv,
            None => return,
        };

        let cx = origin.x + (col as f32 - row as f32) * half_w;
        let cy = origin.y + (col as f32 + row as f32) * half_h;
        let tile_rect = egui::Rect::from_min_size(
            egui::pos2(cx - half_w, cy),
            egui::vec2(half_w * 2.0, half_h * 2.0),
        );
        let uv = egui::Rect::from_min_max(egui::pos2(u0, v0), egui::pos2(u1, v1));

        let mut mesh = egui::Mesh::with_texture(texture.id());
        mesh.add_rect_with_uv(
            tile_rect,
            uv,
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
        );
        painter.add(egui::Shape::mesh(mesh));
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_wall_preview(
        painter: &egui::Painter,
        col: u16,
        row: u16,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
        wall_atlas: Option<&render::SpriteAtlas>,
        wall_texture: Option<&egui::TextureHandle>,
        wall_id: u16,
        is_left_wall: bool,
        alpha: u8,
    ) {
        let (atlas, texture) = match (wall_atlas, wall_texture) {
            (Some(a), Some(t)) => (a, t),
            _ => return,
        };
        if wall_id == 0 {
            return;
        }
        let idx = wall_id as u32;
        let sprite_h = atlas.sprite_height(idx);
        if sprite_h == 0 {
            return;
        }
        let (u0, v0, u1, v1) = match atlas.sprite_uv(idx) {
            Some(uv) => uv,
            None => return,
        };

        let zoom = half_w / (map::TILE_WIDTH / 2.0);
        let screen_h = sprite_h as f32 * zoom;
        let cx = origin.x + (col as f32 - row as f32) * half_w;
        let cy = origin.y + (col as f32 + row as f32) * half_h;
        let bottom_y = cy + 2.0 * half_h;
        let sprite_rect = if is_left_wall {
            egui::Rect::from_min_max(
                egui::pos2(cx - half_w, bottom_y - screen_h),
                egui::pos2(cx, bottom_y),
            )
        } else {
            egui::Rect::from_min_max(
                egui::pos2(cx, bottom_y - screen_h),
                egui::pos2(cx + half_w, bottom_y),
            )
        };
        let uv = egui::Rect::from_min_max(egui::pos2(u0, v0), egui::pos2(u1, v1));

        let mut mesh = egui::Mesh::with_texture(texture.id());
        mesh.add_rect_with_uv(
            sprite_rect,
            uv,
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
        );
        painter.add(egui::Shape::mesh(mesh));
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_prefab_preview(
        painter: &egui::Painter,
        prefab: &map::Map,
        origin_tile: (u16, u16),
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
        tile_atlas: Option<&render::TileAtlas>,
        tile_texture: Option<&egui::TextureHandle>,
        wall_atlas: Option<&render::SpriteAtlas>,
        wall_texture: Option<&egui::TextureHandle>,
    ) {
        let anchor = prefab::placement_anchor(prefab);

        for prefab_row in 0..prefab.height {
            for prefab_col in 0..prefab.width {
                let idx = prefab_row as usize * prefab.width as usize + prefab_col as usize;
                let tile = prefab.tiles[idx];
                let dst_col =
                    i32::from(origin_tile.0) + i32::from(prefab_col) - i32::from(anchor.0);
                let dst_row =
                    i32::from(origin_tile.1) + i32::from(prefab_row) - i32::from(anchor.1);
                if dst_col < 0
                    || dst_row < 0
                    || dst_col > i32::from(u16::MAX)
                    || dst_row > i32::from(u16::MAX)
                {
                    continue;
                }
                let (dst_col, dst_row) = (dst_col as u16, dst_row as u16);

                if tile.ground != 0 {
                    Self::draw_paint_preview_alpha(
                        painter,
                        dst_col,
                        dst_row,
                        origin,
                        half_w,
                        half_h,
                        PaintLayer::Ground,
                        tile.ground,
                        tile_atlas,
                        tile_texture,
                        wall_atlas,
                        wall_texture,
                        140,
                    );
                }
                if is_rendered_wall(tile.left_wall) {
                    Self::draw_paint_preview_alpha(
                        painter,
                        dst_col,
                        dst_row,
                        origin,
                        half_w,
                        half_h,
                        PaintLayer::LeftWall,
                        tile.left_wall,
                        tile_atlas,
                        tile_texture,
                        wall_atlas,
                        wall_texture,
                        150,
                    );
                }
                if is_rendered_wall(tile.right_wall) {
                    Self::draw_paint_preview_alpha(
                        painter,
                        dst_col,
                        dst_row,
                        origin,
                        half_w,
                        half_h,
                        PaintLayer::RightWall,
                        tile.right_wall,
                        tile_atlas,
                        tile_texture,
                        wall_atlas,
                        wall_texture,
                        150,
                    );
                }
            }
        }
    }

    fn draw_prefab_wall_overwrite_preview(
        painter: &egui::Painter,
        map: &map::Map,
        prefab: &map::Map,
        origin_tile: (u16, u16),
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
        wall_atlas: Option<&render::SpriteAtlas>,
        wall_texture: Option<&egui::TextureHandle>,
    ) {
        let anchor = prefab::placement_anchor(prefab);

        for prefab_row in 0..prefab.height {
            for prefab_col in 0..prefab.width {
                let idx = prefab_row as usize * prefab.width as usize + prefab_col as usize;
                let source_tile = prefab.tiles[idx];
                let dst_col =
                    i32::from(origin_tile.0) + i32::from(prefab_col) - i32::from(anchor.0);
                let dst_row =
                    i32::from(origin_tile.1) + i32::from(prefab_row) - i32::from(anchor.1);
                if dst_col < 0
                    || dst_row < 0
                    || dst_col >= i32::from(map.width)
                    || dst_row >= i32::from(map.height)
                {
                    continue;
                }

                let (dst_col, dst_row) = (dst_col as u16, dst_row as u16);
                let dst_idx = dst_row as usize * map.width as usize + dst_col as usize;
                let target_tile = map.tiles[dst_idx];
                let overwrite = Self::overwritten_wall_sides(source_tile, target_tile);
                if overwrite.left_wall || overwrite.right_wall {
                    Self::draw_wall_overwrite_warning(
                        painter,
                        dst_col,
                        dst_row,
                        target_tile,
                        overwrite,
                        origin,
                        half_w,
                        half_h,
                        wall_atlas,
                        wall_texture,
                    );
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_selection_move_preview(
        painter: &egui::Painter,
        selection_map: &map::Map,
        top_left: (u16, u16),
        layers: &LayerVisibility,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
        tile_atlas: Option<&render::TileAtlas>,
        tile_texture: Option<&egui::TextureHandle>,
        wall_atlas: Option<&render::SpriteAtlas>,
        wall_texture: Option<&egui::TextureHandle>,
    ) {
        for src_row in 0..selection_map.height {
            for src_col in 0..selection_map.width {
                let idx = src_row as usize * selection_map.width as usize + src_col as usize;
                let tile = selection_map.tiles[idx];
                let dst_col = top_left.0 + src_col;
                let dst_row = top_left.1 + src_row;

                if layers.ground && tile.ground != 0 {
                    Self::draw_paint_preview_alpha(
                        painter,
                        dst_col,
                        dst_row,
                        origin,
                        half_w,
                        half_h,
                        PaintLayer::Ground,
                        tile.ground,
                        tile_atlas,
                        tile_texture,
                        wall_atlas,
                        wall_texture,
                        140,
                    );
                }
                if layers.left_wall && is_rendered_wall(tile.left_wall) {
                    Self::draw_paint_preview_alpha(
                        painter,
                        dst_col,
                        dst_row,
                        origin,
                        half_w,
                        half_h,
                        PaintLayer::LeftWall,
                        tile.left_wall,
                        tile_atlas,
                        tile_texture,
                        wall_atlas,
                        wall_texture,
                        150,
                    );
                }
                if layers.right_wall && is_rendered_wall(tile.right_wall) {
                    Self::draw_paint_preview_alpha(
                        painter,
                        dst_col,
                        dst_row,
                        origin,
                        half_w,
                        half_h,
                        PaintLayer::RightWall,
                        tile.right_wall,
                        tile_atlas,
                        tile_texture,
                        wall_atlas,
                        wall_texture,
                        150,
                    );
                }
            }
        }
    }

    fn draw_wall_overwrite_preview(
        painter: &egui::Painter,
        map: &map::Map,
        source: &map::Map,
        top_left: (u16, u16),
        layers: &LayerVisibility,
        ignore_region: Option<TileSelection>,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
        wall_atlas: Option<&render::SpriteAtlas>,
        wall_texture: Option<&egui::TextureHandle>,
    ) {
        for src_row in 0..source.height {
            for src_col in 0..source.width {
                let dst_col = u32::from(top_left.0) + u32::from(src_col);
                let dst_row = u32::from(top_left.1) + u32::from(src_row);
                if dst_col >= u32::from(map.width) || dst_row >= u32::from(map.height) {
                    continue;
                }
                if ignore_region
                    .map(|selection| selection.contains((dst_col as u16, dst_row as u16)))
                    .unwrap_or(false)
                {
                    continue;
                }

                let src_idx = src_row as usize * source.width as usize + src_col as usize;
                let dst_idx = dst_row as usize * map.width as usize + dst_col as usize;
                let mut source_tile = source.tiles[src_idx];
                if !layers.left_wall {
                    source_tile.left_wall = 0;
                }
                if !layers.right_wall {
                    source_tile.right_wall = 0;
                }
                let target_tile = map.tiles[dst_idx];
                let overwrite = Self::overwritten_wall_sides(source_tile, target_tile);
                if overwrite.left_wall || overwrite.right_wall {
                    Self::draw_wall_overwrite_warning(
                        painter,
                        dst_col as u16,
                        dst_row as u16,
                        target_tile,
                        overwrite,
                        origin,
                        half_w,
                        half_h,
                        wall_atlas,
                        wall_texture,
                    );
                }
            }
        }
    }

    fn overwritten_wall_sides(source_tile: map::Tile, target_tile: map::Tile) -> LayerVisibility {
        LayerVisibility {
            ground: false,
            left_wall: is_rendered_wall(source_tile.left_wall)
                && is_rendered_wall(target_tile.left_wall),
            right_wall: is_rendered_wall(source_tile.right_wall)
                && is_rendered_wall(target_tile.right_wall),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_wall_overwrite_warning(
        painter: &egui::Painter,
        col: u16,
        row: u16,
        target_tile: map::Tile,
        overwrite: LayerVisibility,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
        wall_atlas: Option<&render::SpriteAtlas>,
        wall_texture: Option<&egui::TextureHandle>,
    ) {
        Self::draw_wall_overwrite_tile(painter, col, row, origin, half_w, half_h);
        if overwrite.left_wall {
            Self::draw_wall_overwrite_sprite_outline(
                painter,
                col,
                row,
                origin,
                half_w,
                half_h,
                wall_atlas,
                wall_texture,
                target_tile.left_wall,
                true,
            );
        }
        if overwrite.right_wall {
            Self::draw_wall_overwrite_sprite_outline(
                painter,
                col,
                row,
                origin,
                half_w,
                half_h,
                wall_atlas,
                wall_texture,
                target_tile.right_wall,
                false,
            );
        }
    }

    fn draw_wall_overwrite_tile(
        painter: &egui::Painter,
        col: u16,
        row: u16,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
    ) {
        let (top, right, bottom, left) =
            Self::tile_diamond_points(i32::from(col), i32::from(row), origin, half_w, half_h);
        painter.add(egui::Shape::convex_polygon(
            vec![top, right, bottom, left],
            egui::Color32::from_rgba_unmultiplied(192, 32, 24, 84),
            egui::Stroke::NONE,
        ));
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_wall_overwrite_sprite_outline(
        painter: &egui::Painter,
        col: u16,
        row: u16,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
        wall_atlas: Option<&render::SpriteAtlas>,
        wall_texture: Option<&egui::TextureHandle>,
        wall_id: u16,
        is_left_wall: bool,
    ) {
        let (atlas, texture) = match (wall_atlas, wall_texture) {
            (Some(a), Some(t)) => (a, t),
            _ => return,
        };
        if !is_rendered_wall(wall_id) {
            return;
        }
        let idx = wall_id as u32;
        let sprite_h = atlas.sprite_height(idx);
        if sprite_h == 0 {
            return;
        }
        let (u0, v0, u1, v1) = match atlas.sprite_uv(idx) {
            Some(uv) => uv,
            None => return,
        };

        let zoom = half_w / (map::TILE_WIDTH / 2.0);
        let screen_h = sprite_h as f32 * zoom;
        let cx = origin.x + (col as f32 - row as f32) * half_w;
        let cy = origin.y + (col as f32 + row as f32) * half_h;
        let bottom_y = cy + 2.0 * half_h;
        let sprite_rect = if is_left_wall {
            egui::Rect::from_min_max(
                egui::pos2(cx - half_w, bottom_y - screen_h),
                egui::pos2(cx, bottom_y),
            )
        } else {
            egui::Rect::from_min_max(
                egui::pos2(cx, bottom_y - screen_h),
                egui::pos2(cx + half_w, bottom_y),
            )
        };
        let uv = egui::Rect::from_min_max(egui::pos2(u0, v0), egui::pos2(u1, v1));
        let outline = (zoom.max(1.0) * 1.2).round();
        let offsets = [
            egui::vec2(-outline, 0.0),
            egui::vec2(outline, 0.0),
            egui::vec2(0.0, -outline),
            egui::vec2(0.0, outline),
            egui::vec2(-outline, -outline),
            egui::vec2(outline, -outline),
            egui::vec2(-outline, outline),
            egui::vec2(outline, outline),
        ];

        for offset in offsets {
            let mut mesh = egui::Mesh::with_texture(texture.id());
            mesh.add_rect_with_uv(
                sprite_rect.translate(offset),
                uv,
                egui::Color32::from_rgba_unmultiplied(220, 48, 32, 96),
            );
            painter.add(egui::Shape::mesh(mesh));
        }
    }

    fn draw_erase_preview(
        painter: &egui::Painter,
        col: u16,
        row: u16,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
    ) {
        let cx = origin.x + (col as f32 - row as f32) * half_w;
        let cy = origin.y + (col as f32 + row as f32) * half_h;

        let top = egui::pos2(cx, cy);
        let right = egui::pos2(cx + half_w, cy + half_h);
        let bottom = egui::pos2(cx, cy + half_h * 2.0);
        let left = egui::pos2(cx - half_w, cy + half_h);

        painter.add(egui::Shape::convex_polygon(
            vec![top, right, bottom, left],
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 90),
            egui::Stroke::NONE,
        ));
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_paint_shape_preview(
        painter: &egui::Painter,
        map: &map::Map,
        shape_kind: ShapeKind,
        start: (u16, u16),
        end: (u16, u16),
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
        paint_layer: PaintLayer,
        paint_value: u16,
        tile_atlas: Option<&render::TileAtlas>,
        tile_texture: Option<&egui::TextureHandle>,
        wall_atlas: Option<&render::SpriteAtlas>,
        wall_texture: Option<&egui::TextureHandle>,
    ) {
        let points = shape::paint_points(shape_kind, start, end);
        for (x, y) in points {
            if x < 0 || y < 0 {
                continue;
            }
            let (x, y) = (x as u16, y as u16);
            if x >= map.width || y >= map.height {
                continue;
            }
            Self::draw_paint_preview(
                painter,
                x,
                y,
                origin,
                half_w,
                half_h,
                paint_layer,
                paint_value,
                tile_atlas,
                tile_texture,
                wall_atlas,
                wall_texture,
            );
        }
    }

    fn eyedropper_target(
        tile: &map::Tile,
        paint_layer: PaintLayer,
    ) -> EyedropperPick {
        match paint_layer {
            PaintLayer::Ground => EyedropperPick::Ground(tile.ground),
            PaintLayer::LeftWall => EyedropperPick::LeftWall(tile.left_wall),
            PaintLayer::RightWall => EyedropperPick::RightWall(tile.right_wall),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_eyedropper_target_highlight(
        painter: &egui::Painter,
        col: u16,
        row: u16,
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
        pick: &EyedropperPick,
        colors: &ThemeColors,
    ) {
        let cx = origin.x + (col as f32 - row as f32) * half_w;
        let cy = origin.y + (col as f32 + row as f32) * half_h;
        let top = egui::pos2(cx, cy);
        let right = egui::pos2(cx + half_w, cy + half_h);
        let bottom = egui::pos2(cx, cy + 2.0 * half_h);
        let left = egui::pos2(cx - half_w, cy + half_h);
        let center = egui::pos2(cx, cy + half_h);

        let fill = colors.accent.gamma_multiply(0.2);
        let stroke = egui::Stroke::new(2.0, colors.accent);

        match pick {
            EyedropperPick::Ground(_) => {
                painter.add(egui::Shape::convex_polygon(
                    vec![top, right, bottom, left],
                    fill,
                    stroke,
                ));
            }
            EyedropperPick::LeftWall(_) => {
                painter.add(egui::Shape::convex_polygon(
                    vec![top, center, left],
                    fill,
                    egui::Stroke::NONE,
                ));
                painter.line_segment([top, left], stroke);
                painter.line_segment([left, center], stroke);
                painter.line_segment([center, top], stroke);
            }
            EyedropperPick::RightWall(_) => {
                painter.add(egui::Shape::convex_polygon(
                    vec![top, right, center],
                    fill,
                    egui::Stroke::NONE,
                ));
                painter.line_segment([top, right], stroke);
                painter.line_segment([right, center], stroke);
                painter.line_segment([center, top], stroke);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_paint_line_preview(
        painter: &egui::Painter,
        start: (u16, u16),
        end: (u16, u16),
        origin: egui::Pos2,
        half_w: f32,
        half_h: f32,
        paint_layer: PaintLayer,
        paint_value: u16,
        tile_atlas: Option<&render::TileAtlas>,
        tile_texture: Option<&egui::TextureHandle>,
        wall_atlas: Option<&render::SpriteAtlas>,
        wall_texture: Option<&egui::TextureHandle>,
    ) {
        let (mut x0, mut y0) = (start.0 as i32, start.1 as i32);
        let (x1, y1) = (end.0 as i32, end.1 as i32);

        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if x0 >= 0 && y0 >= 0 {
                Self::draw_paint_preview(
                    painter,
                    x0 as u16,
                    y0 as u16,
                    origin,
                    half_w,
                    half_h,
                    paint_layer,
                    paint_value,
                    tile_atlas,
                    tile_texture,
                    wall_atlas,
                    wall_texture,
                );
            }

            if x0 == x1 && y0 == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }
}
