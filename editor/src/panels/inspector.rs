use eframe::egui;

use super::toolbar::Tool;
use crate::document::PaintLayer;
use crate::prefab::{OccupiedBounds, PrefabAsset};
use crate::theme::{ThemeColors, theme_colors};
use crate::widgets::{icons, tooltip};

const INSPECTOR_MIN_WIDTH: f32 = 260.0;
const INSPECTOR_MAX_WIDTH: f32 = 760.0;
const INSPECTOR_DEFAULT_WIDTH: f32 = 420.0;
const TILE_PREVIEW_HEIGHT: f32 = 26.0;
const WALL_PREVIEW_WIDTH: f32 = 28.0;
const WALL_PREVIEW_HEIGHT: f32 = 56.0;
const TILE_SHEET_MIN_HEIGHT: f32 = 96.0;
const TAB_MAP_COLLAPSE_ID: &str = "inspector_tab_map_preview";
const TAB_MAP_ROW_HEIGHT_OPEN: f32 = 22.0;
const TAB_MAP_ROW_HEIGHT_COLLAPSED: f32 = 18.0;
const TAB_MAP_BORDER_PAD_OPEN: f32 = 10.0;
const TAB_MAP_BORDER_PAD_COLLAPSED: f32 = 4.0;
const TAB_MAP_LABEL_NUDGE_Y: f32 = 1.0;
const PREFAB_LIST_HEADER_HEIGHT: f32 = 24.0;
const PREFAB_LIST_ROW_HEIGHT: f32 = 28.0;
const PREFAB_LIST_CELL_PAD_X: f32 = 10.0;
const PREFAB_LIST_SIZE_COLUMN_WIDTH: f32 = 62.0;
const PREFAB_BROWSER_MIN_HEIGHT: f32 = 120.0;
const PREFAB_PREVIEW_DEFAULT_HEIGHT: f32 = 220.0;
const PREFAB_PREVIEW_MIN_HEIGHT: f32 = 110.0;
const PREFAB_SPLITTER_HEIGHT: f32 = 10.0;
const PREFAB_PREVIEW_HEIGHT_ID: &str = "inspector_prefab_preview_height";
const PREFAB_PREVIEW_HEADER_HEIGHT: f32 = 20.0;
const PREFAB_PREVIEW_TOP_PAD: f32 = 4.0;
const PREFAB_PREVIEW_BOTTOM_PAD: f32 = 44.0;
const PREFAB_SECTION_ICON: &str = "\u{E5D8}";
const TILE_PALETTE_SECTION_ICON: &str = "\u{E2A1}";
const PREFAB_EDIT_ICON: &str = "\u{1F4DD}";

#[derive(Default)]
pub struct InspectorResponse {
    pub new_prefab_requested: bool,
    pub import_prefab_requested: bool,
    pub select_prefab_index: Option<usize>,
    pub edit_prefab_index: Option<usize>,
    pub duplicate_prefab_index: Option<usize>,
    pub start_rename_prefab_index: Option<usize>,
    pub submit_prefab_rename: bool,
    pub cancel_prefab_rename: bool,
    pub delete_prefab_index: Option<usize>,
}

pub struct InspectorPanel;

#[derive(Clone, Copy)]
struct RenderBounds {
    min: egui::Pos2,
    max: egui::Pos2,
}

#[derive(Clone, Copy)]
struct PrefabPaneLayout {
    top_height: f32,
    preview_body_height: f32,
    min_preview_height: f32,
    max_preview_height: f32,
}

struct PrefabListRowOutput {
    response: egui::Response,
    rename_submitted: bool,
    rename_canceled: bool,
}

impl RenderBounds {
    fn width(self) -> f32 {
        (self.max.x - self.min.x).max(1.0)
    }

    fn height(self) -> f32 {
        (self.max.y - self.min.y).max(1.0)
    }
}

impl InspectorPanel {
    fn select_all_text(ui: &egui::Ui, widget_id: egui::Id, value_text: &str) {
        let mut state = egui::TextEdit::load_state(ui.ctx(), widget_id).unwrap_or_default();
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::default(),
                egui::text::CCursor::new(value_text.chars().count()),
            )));
        state.store(ui.ctx(), widget_id);
    }

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
        reveal_ground_tile: &mut Option<u16>,
        reveal_wall_tile: &mut Option<u16>,
        prefabs: &[PrefabAsset],
        selected_prefab_index: Option<usize>,
        prefab_search: &mut String,
        renaming_prefab_index: Option<usize>,
        prefab_rename_buffer: &mut String,
        prefab_rename_should_focus: &mut bool,
        active_tool: Tool,
    ) -> InspectorResponse {
        let colors = theme_colors();
        let mut response = InspectorResponse::default();

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
                    reveal_ground_tile,
                    reveal_wall_tile,
                    prefabs,
                    selected_prefab_index,
                    prefab_search,
                    renaming_prefab_index,
                    prefab_rename_buffer,
                    prefab_rename_should_focus,
                    active_tool,
                    &mut response,
                );
            });

        response
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
        reveal_ground_tile: &mut Option<u16>,
        reveal_wall_tile: &mut Option<u16>,
        prefabs: &[PrefabAsset],
        selected_prefab_index: Option<usize>,
        prefab_search: &mut String,
        renaming_prefab_index: Option<usize>,
        prefab_rename_buffer: &mut String,
        prefab_rename_should_focus: &mut bool,
        active_tool: Tool,
        response: &mut InspectorResponse,
    ) {
        let prefab_mode = active_tool == Tool::Stamp;
        let bottom_section_id = ui.make_persistent_id(egui::Id::new(TAB_MAP_COLLAPSE_ID));
        let mut bottom_section_state =
            egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                bottom_section_id,
                true,
            );
        let tab_map_open = bottom_section_state.is_open();
        let tab_map_border_pad = if tab_map_open {
            TAB_MAP_BORDER_PAD_OPEN
        } else {
            TAB_MAP_BORDER_PAD_COLLAPSED
        };
        let tab_map_row_height = if tab_map_open {
            TAB_MAP_ROW_HEIGHT_OPEN
        } else {
            TAB_MAP_ROW_HEIGHT_COLLAPSED
        };
        let prefab_pane_layout = prefab_mode
            .then(|| Self::prefab_pane_layout(ui, PREFAB_PREVIEW_HEADER_HEIGHT, 0.0, 0.0));

        // Draw left border
        let panel_rect = ui.max_rect();
        ui.painter().line_segment(
            [
                egui::pos2(panel_rect.left() - 14.0, panel_rect.top() - 10.0),
                egui::pos2(panel_rect.left() - 14.0, panel_rect.bottom() + 10.0),
            ],
            egui::Stroke::new(1.0, colors.border),
        );

        if prefab_mode {
            Self::section_header(ui, colors, "Prefabs", Some(PREFAB_SECTION_ICON));
            ui.add_space(8.0);
            let top_height = prefab_pane_layout
                .map(|layout| layout.top_height)
                .unwrap_or_else(|| ui.available_height());
            let (top_rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), top_height),
                egui::Sense::hover(),
            );
            let mut top_ui = ui.new_child(egui::UiBuilder::new().max_rect(top_rect));
            Self::draw_prefab_browser(
                &mut top_ui,
                colors,
                prefabs,
                selected_prefab_index,
                prefab_search,
                renaming_prefab_index,
                prefab_rename_buffer,
                prefab_rename_should_focus,
                response,
                top_rect.height(),
            );
        } else {
            Self::section_header(ui, colors, "Tile Palette", Some(TILE_PALETTE_SECTION_ICON));
            ui.add_space(8.0);
            let top_height = Self::top_section_max_height(ui, tab_map_open);
            let (top_rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), top_height),
                egui::Sense::hover(),
            );
            let mut top_ui = ui.new_child(egui::UiBuilder::new().max_rect(top_rect));
            Self::draw_tile_palette(
                &mut top_ui,
                colors,
                tile_atlas,
                atlas_texture,
                wall_atlas,
                wall_texture,
                active_paint_layer,
                selected_ground_tile,
                selected_wall_tile,
                reveal_ground_tile,
                reveal_wall_tile,
                tab_map_open,
            );
        }

        if prefab_mode {
            let layout = prefab_pane_layout.expect("prefab layout should exist in prefab mode");
            let splitter_response = Self::draw_prefab_splitter(ui, colors, panel_rect);
            if splitter_response.dragged() {
                let delta_y = ui.input(|i| i.pointer.delta().y);
                let preview_height = (layout.preview_body_height - delta_y)
                    .clamp(layout.min_preview_height, layout.max_preview_height);
                Self::store_prefab_preview_height(ui.ctx(), preview_height);
            }
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), PREFAB_PREVIEW_HEADER_HEIGHT),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(
                        egui::RichText::new("Preview")
                            .size(14.0)
                            .strong()
                            .color(colors.text),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let edit_enabled = selected_prefab_index
                            .map(|index| index < prefabs.len())
                            .unwrap_or(false);
                        let edit_response = ui.add_enabled_ui(edit_enabled, |ui| {
                            ui.add_sized(
                                egui::vec2(24.0, 18.0),
                                egui::Button::new(
                                    egui::RichText::new(PREFAB_EDIT_ICON)
                                        .font(icons::symbol_icon_font_id(13.0))
                                        .color(colors.text),
                                )
                                .fill(egui::Color32::TRANSPARENT)
                                .stroke(egui::Stroke::new(1.0, colors.border))
                                .corner_radius(0.0),
                            )
                        });
                        let _ = tooltip::attach(edit_response.response, "Edit Prefab");
                        if edit_response.inner.clicked() {
                            response.edit_prefab_index =
                                selected_prefab_index.filter(|index| *index < prefabs.len());
                        }
                    });
                },
            );
            let (preview_rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), layout.preview_body_height.max(1.0)),
                egui::Sense::hover(),
            );
            let full_preview_rect = egui::Rect::from_min_max(
                egui::pos2(panel_rect.left() - 14.0, preview_rect.top()),
                egui::pos2(panel_rect.right() + 14.0, preview_rect.bottom()),
            );
            let mut preview_ui = ui.new_child(egui::UiBuilder::new().max_rect(full_preview_rect));
            Self::draw_selected_prefab_preview(
                &mut preview_ui,
                colors,
                tile_atlas,
                atlas_texture,
                wall_atlas,
                wall_texture,
                prefabs,
                selected_prefab_index,
            );
        } else {
            Self::full_width_border(ui, colors, panel_rect, tab_map_border_pad);

            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), tab_map_row_height),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.vertical(|ui| {
                        ui.add_space(TAB_MAP_LABEL_NUDGE_Y);
                        ui.label(
                            egui::RichText::new("Tab Map")
                                .size(14.0)
                                .strong()
                                .color(colors.text),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let toggle_label = if bottom_section_state.is_open() {
                            "-"
                        } else {
                            "+"
                        };
                        let toggle_response = ui.add_sized(
                            egui::vec2(18.0, 16.0),
                            egui::Button::new(
                                egui::RichText::new(toggle_label)
                                    .size(12.0)
                                    .strong()
                                    .color(colors.text),
                            )
                            .fill(egui::Color32::TRANSPARENT)
                            .stroke(egui::Stroke::new(1.0, colors.border))
                            .corner_radius(0.0),
                        );
                        if toggle_response.clicked() {
                            bottom_section_state.toggle(ui);
                        }
                    });
                },
            );
            bottom_section_state.store(ui.ctx());
            if bottom_section_state.is_open() {
                ui.add_space(6.0);
                Self::draw_tab_map(ui, colors, map, sotp);
            }
            Self::full_width_border(ui, colors, panel_rect, tab_map_border_pad);
        }
    }

    fn prefab_pane_layout(
        ui: &egui::Ui,
        header_height: f32,
        bottom_border_pad: f32,
        preview_gap: f32,
    ) -> PrefabPaneLayout {
        let reserved =
            PREFAB_SPLITTER_HEIGHT + header_height + preview_gap + bottom_border_pad * 2.0;
        let available_for_panes = (ui.available_height() - reserved).max(0.0);
        let min_top_height = PREFAB_BROWSER_MIN_HEIGHT.min(available_for_panes);
        let max_preview_height = (available_for_panes - min_top_height).max(0.0);
        let min_preview_height = PREFAB_PREVIEW_MIN_HEIGHT.min(max_preview_height);
        let preview_height = Self::load_prefab_preview_height(ui.ctx())
            .clamp(min_preview_height, max_preview_height);
        let top_height = (available_for_panes - preview_height).max(0.0);

        PrefabPaneLayout {
            top_height,
            preview_body_height: preview_height,
            min_preview_height,
            max_preview_height,
        }
    }

    fn load_prefab_preview_height(ctx: &egui::Context) -> f32 {
        ctx.data_mut(|data| {
            data.get_persisted::<f32>(egui::Id::new(PREFAB_PREVIEW_HEIGHT_ID))
                .unwrap_or(PREFAB_PREVIEW_DEFAULT_HEIGHT)
        })
    }

    fn store_prefab_preview_height(ctx: &egui::Context, height: f32) {
        ctx.data_mut(|data| {
            data.insert_persisted(egui::Id::new(PREFAB_PREVIEW_HEIGHT_ID), height);
        });
    }

    fn draw_prefab_splitter(
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        panel_rect: egui::Rect,
    ) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), PREFAB_SPLITTER_HEIGHT),
            egui::Sense::click_and_drag(),
        );

        let line_y = rect.center().y;
        let left = panel_rect.left() - 14.0;
        let right = panel_rect.right() + 14.0;
        let handle_color = if response.hovered() || response.dragged() {
            colors.accent
        } else {
            colors.border
        };

        ui.painter().line_segment(
            [egui::pos2(left, line_y), egui::pos2(right, line_y)],
            egui::Stroke::new(1.0, colors.border),
        );

        let handle_rect = egui::Rect::from_center_size(
            egui::pos2(rect.center().x, line_y),
            egui::vec2(54.0, 4.0),
        );
        ui.painter().rect_filled(handle_rect, 2.0, handle_color);

        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }

        response
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
        reveal_ground_tile: &mut Option<u16>,
        reveal_wall_tile: &mut Option<u16>,
        tab_map_open: bool,
    ) {
        let wall_selected = !matches!(*active_paint_layer, PaintLayer::Ground);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;

            let ground_response = Self::outline_toggle(ui, colors, "Ground", !wall_selected, 68.0);
            if ground_response.clicked() {
                *active_paint_layer = PaintLayer::Ground;
            }

            let wall_response = Self::outline_toggle(ui, colors, "Wall", wall_selected, 56.0);
            if wall_response.clicked() {
                if matches!(*active_paint_layer, PaintLayer::Ground) {
                    *active_paint_layer = PaintLayer::LeftWall;
                }
            }

            if wall_selected {
                ui.add_space(10.0);
                ui.label(egui::RichText::new("Side").size(12.0).color(colors.muted));

                let left_response = Self::outline_toggle(
                    ui,
                    colors,
                    "Left",
                    matches!(*active_paint_layer, PaintLayer::LeftWall),
                    52.0,
                );
                if left_response.clicked() {
                    *active_paint_layer = PaintLayer::LeftWall;
                }
                let _ = tooltip::attach(left_response, "Toggle [Q]");

                let right_response = Self::outline_toggle(
                    ui,
                    colors,
                    "Right",
                    matches!(*active_paint_layer, PaintLayer::RightWall),
                    56.0,
                );
                if right_response.clicked() {
                    *active_paint_layer = PaintLayer::RightWall;
                }
                let _ = tooltip::attach(right_response, "Toggle [Q]");
            }
        });
        ui.add_space(6.0);

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

        let max_palette_height = Self::top_section_max_height(ui, tab_map_open);

        if matches!(*active_paint_layer, PaintLayer::Ground) {
            Self::draw_ground_sheet(
                ui,
                colors,
                tile_atlas,
                atlas_texture,
                selected_ground_tile,
                reveal_ground_tile,
                max_palette_height,
            );
        } else {
            Self::draw_wall_sheet(
                ui,
                colors,
                wall_atlas,
                wall_texture,
                selected_wall_tile,
                reveal_wall_tile,
                max_palette_height,
            );
        }
    }

    fn top_section_max_height(ui: &egui::Ui, bottom_section_open: bool) -> f32 {
        // Keep room for the bottom inspector sections below the tile sheet:
        // - border after palette
        // - Tab Map row (+/- toggle)
        // - optional tab-map preview body
        // - final bottom border
        let available = ui.available_size();
        let border_pad = if bottom_section_open {
            TAB_MAP_BORDER_PAD_OPEN
        } else {
            TAB_MAP_BORDER_PAD_COLLAPSED
        };
        let border_h = border_pad * 2.0;
        let bottom_row_h = if bottom_section_open {
            TAB_MAP_ROW_HEIGHT_OPEN
        } else {
            TAB_MAP_ROW_HEIGHT_COLLAPSED
        };
        let bottom_preview_h = if bottom_section_open {
            6.0 + (available.x.max(0.0) * 0.5)
        } else {
            0.0
        };
        let reserved = border_h + bottom_row_h + bottom_preview_h + border_h;

        (available.y - reserved).max(TILE_SHEET_MIN_HEIGHT)
    }

    fn outline_toggle(
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        label: &str,
        selected: bool,
        width: f32,
    ) -> egui::Response {
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(width, 24.0), egui::Sense::click());

        let fill = if selected {
            colors.accent.gamma_multiply(0.14)
        } else if response.hovered() {
            colors.panel_2
        } else {
            egui::Color32::TRANSPARENT
        };

        let stroke = if selected {
            egui::Stroke::new(1.0, colors.accent)
        } else {
            egui::Stroke::new(1.0, colors.border)
        };

        let text_color = if selected {
            colors.accent
        } else if response.hovered() {
            colors.text
        } else {
            colors.muted
        };

        ui.painter()
            .rect(rect, 4.0, fill, stroke, egui::StrokeKind::Inside);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(12.5),
            text_color,
        );

        response
    }

    fn draw_ground_sheet(
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        tile_atlas: Option<&render::TileAtlas>,
        atlas_texture: Option<&egui::TextureHandle>,
        selected_ground_tile: &mut u16,
        reveal_ground_tile: &mut Option<u16>,
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

        let (tile_w, tile_h) = atlas.tile_size();
        let preview_scale = TILE_PREVIEW_HEIGHT / tile_h as f32;
        let cell_size = egui::vec2(
            (tile_w as f32 * preview_scale).max(1.0).round(),
            (tile_h as f32 * preview_scale).max(1.0).round(),
        );

        let row_height = cell_size.y;
        let row_step = row_height + ui.spacing().item_spacing.y;
        let (columns, row_count) = Self::grid_layout(
            ui,
            selectable_count,
            cell_size.x,
            row_step,
            max_palette_height,
        );

        let mut scroll_area = egui::ScrollArea::vertical()
            .id_salt("ground_tiles_scroll")
            .max_height(max_palette_height)
            .auto_shrink([false, false]);
        let mut consumed_reveal = false;
        if let Some(target_id) = *reveal_ground_tile {
            let target_idx = target_id.saturating_sub(1) as usize;
            if target_idx < selectable_count {
                let target_row = target_idx / columns;
                let target_offset =
                    Self::reveal_scroll_offset(target_row, row_step, row_count, max_palette_height);
                scroll_area = scroll_area.vertical_scroll_offset(target_offset);
                consumed_reveal = true;
            } else {
                *reveal_ground_tile = None;
            }
        }

        scroll_area.show_rows(ui, row_height, row_count, |ui, row_range| {
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
        if consumed_reveal {
            *reveal_ground_tile = None;
        }
    }

    fn draw_wall_sheet(
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        wall_atlas: Option<&render::SpriteAtlas>,
        wall_texture: Option<&egui::TextureHandle>,
        selected_wall_tile: &mut u16,
        reveal_wall_tile: &mut Option<u16>,
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

        let cell_size = egui::vec2(WALL_PREVIEW_WIDTH, WALL_PREVIEW_HEIGHT);
        let row_height = cell_size.y;
        let row_step = row_height + ui.spacing().item_spacing.y;
        let (columns, row_count) = Self::grid_layout(
            ui,
            wall_ids.len(),
            cell_size.x,
            row_step,
            max_palette_height,
        );

        let mut scroll_area = egui::ScrollArea::vertical()
            .id_salt("wall_tiles_scroll")
            .max_height(max_palette_height)
            .auto_shrink([false, false]);
        let mut consumed_reveal = false;
        if let Some(target_id) = *reveal_wall_tile {
            if let Some(target_idx) = wall_ids.iter().position(|&id| id == target_id as u32) {
                let target_row = target_idx / columns;
                let target_offset =
                    Self::reveal_scroll_offset(target_row, row_step, row_count, max_palette_height);
                scroll_area = scroll_area.vertical_scroll_offset(target_offset);
                consumed_reveal = true;
            } else {
                *reveal_wall_tile = None;
            }
        }

        scroll_area.show_rows(ui, row_height, row_count, |ui, row_range| {
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
        if consumed_reveal {
            *reveal_wall_tile = None;
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
        if is_selected {
            let tint = egui::Color32::from_rgba_unmultiplied(
                colors.accent.r(),
                colors.accent.g(),
                colors.accent.b(),
                36,
            );
            ui.painter().rect_filled(rect, 0.0, tint);
        }

        let border = if is_selected {
            egui::Stroke::new(2.5, colors.accent)
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
        let _ = tooltip::attach(response, format!("Tile {}", tile_id));
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
        if is_selected {
            let tint = egui::Color32::from_rgba_unmultiplied(
                colors.accent.r(),
                colors.accent.g(),
                colors.accent.b(),
                28,
            );
            ui.painter().rect_filled(rect, 0.0, tint);
        }

        let border = if is_selected {
            egui::Stroke::new(2.5, colors.accent)
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
        let _ = tooltip::attach(response, format!("Wall {}", wall_id));
    }

    fn grid_layout(
        ui: &egui::Ui,
        item_count: usize,
        cell_width: f32,
        row_step: f32,
        max_height: f32,
    ) -> (usize, usize) {
        if item_count == 0 {
            return (1, 0);
        }

        let base_width = ui.available_width().max(cell_width);
        let mut columns = (base_width / cell_width).floor().max(1.0) as usize;

        // If scrolling is needed, available inner width shrinks by the scrollbar.
        // Recompute columns so row math (including reveal-scroll rows) matches
        // what is actually visible after panel resize.
        let scrollbar_width = ui.spacing().scroll.allocated_width();
        let width_with_scrollbar = (base_width - scrollbar_width).max(cell_width);
        let columns_with_scrollbar = (width_with_scrollbar / cell_width).floor().max(1.0) as usize;

        let mut row_count = item_count.div_ceil(columns);
        let needs_scroll = (row_count as f32 * row_step) > max_height;
        if needs_scroll && columns_with_scrollbar != columns {
            columns = columns_with_scrollbar;
            row_count = item_count.div_ceil(columns);
        }

        (columns, row_count)
    }

    fn reveal_scroll_offset(
        target_row: usize,
        row_step: f32,
        row_count: usize,
        viewport_height: f32,
    ) -> f32 {
        if row_count == 0 || row_step <= 0.0 || viewport_height <= 0.0 {
            return 0.0;
        }

        let content_height = row_count as f32 * row_step;
        let max_offset = (content_height - viewport_height).max(0.0);
        if max_offset <= 0.0 {
            return 0.0;
        }

        // Keep reveal behavior deterministic: place the target in the first
        // visible row whenever possible.
        let target_top = target_row as f32 * row_step;
        target_top.clamp(0.0, max_offset)
    }

    /// Section header: bold label.
    fn section_header(
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        label: &str,
        icon: Option<&str>,
    ) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if let Some(icon) = icon {
                ui.label(
                    egui::RichText::new(icon)
                        .font(icons::symbol_icon_font_id(14.0))
                        .color(colors.text),
                );
            }
            ui.label(
                egui::RichText::new(label)
                    .size(14.0)
                    .strong()
                    .color(colors.text),
            );
        });
    }

    /// Draw a full-width horizontal border spanning edge to edge.
    fn full_width_border(
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        panel_rect: egui::Rect,
        padding: f32,
    ) {
        ui.add_space(padding);
        let cursor_y = ui.cursor().top();
        let left = panel_rect.left() - 14.0;
        let right = panel_rect.right() + 14.0;
        ui.painter().line_segment(
            [egui::pos2(left, cursor_y), egui::pos2(right, cursor_y)],
            egui::Stroke::new(1.0, colors.border),
        );
        ui.add_space(padding);
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

    fn is_rendered_wall(id: u16) -> bool {
        if id == 0 {
            return false;
        }
        (id > 10012) || ((id % 10000) > 12)
    }

    fn prefab_render_bounds(
        map: &map::Map,
        bounds: OccupiedBounds,
        wall_atlas: Option<&render::SpriteAtlas>,
    ) -> Option<RenderBounds> {
        let half_w = map::TILE_WIDTH * 0.5;
        let half_h = map::TILE_HEIGHT * 0.5;
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        let mut found = false;

        for row in bounds.min_row..=bounds.max_row {
            for col in bounds.min_col..=bounds.max_col {
                let idx = row as usize * map.width as usize + col as usize;
                let tile = map.tiles[idx];
                if tile.ground == 0 && tile.left_wall == 0 && tile.right_wall == 0 {
                    continue;
                }

                found = true;
                let cx = (col as f32 - row as f32) * half_w;
                let cy = (col as f32 + row as f32) * half_h;
                min_x = min_x.min(cx - half_w);
                max_x = max_x.max(cx + half_w);
                min_y = min_y.min(cy);
                max_y = max_y.max(cy + map::TILE_HEIGHT);

                if Self::is_rendered_wall(tile.left_wall) {
                    let sprite_h = wall_atlas
                        .map(|atlas| atlas.sprite_height(tile.left_wall as u32) as f32)
                        .unwrap_or(map::TILE_HEIGHT);
                    min_y = min_y.min(cy + map::TILE_HEIGHT - sprite_h);
                }
                if Self::is_rendered_wall(tile.right_wall) {
                    let sprite_h = wall_atlas
                        .map(|atlas| atlas.sprite_height(tile.right_wall as u32) as f32)
                        .unwrap_or(map::TILE_HEIGHT);
                    min_y = min_y.min(cy + map::TILE_HEIGHT - sprite_h);
                }
            }
        }

        found.then_some(RenderBounds {
            min: egui::pos2(min_x, min_y),
            max: egui::pos2(max_x, max_y),
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_prefab_render_scene(
        painter: &egui::Painter,
        map: &map::Map,
        bounds: OccupiedBounds,
        scale: f32,
        offset: egui::Vec2,
        tile_atlas: Option<&render::TileAtlas>,
        tile_texture: Option<&egui::TextureHandle>,
        wall_atlas: Option<&render::SpriteAtlas>,
        wall_texture: Option<&egui::TextureHandle>,
    ) {
        let half_w = map::TILE_WIDTH * 0.5 * scale;
        let half_h = map::TILE_HEIGHT * 0.5 * scale;
        let min_depth = bounds.min_col as u32 + bounds.min_row as u32;
        let max_depth = bounds.max_col as u32 + bounds.max_row as u32;

        for depth in min_depth..=max_depth {
            for row in bounds.min_row..=bounds.max_row {
                let col_i32 = depth as i32 - row as i32;
                if col_i32 < bounds.min_col as i32 || col_i32 > bounds.max_col as i32 {
                    continue;
                }
                let col = col_i32 as u16;
                let idx = row as usize * map.width as usize + col as usize;
                let tile = map.tiles[idx];
                if tile.ground == 0 && tile.left_wall == 0 && tile.right_wall == 0 {
                    continue;
                }

                let cx = (col as f32 - row as f32) * half_w + offset.x;
                let cy = (col as f32 + row as f32) * half_h + offset.y;

                if tile.ground != 0 {
                    if let (Some(atlas), Some(texture)) = (tile_atlas, tile_texture) {
                        if let Some((u0, v0, u1, v1)) =
                            atlas.tile_uv(tile.ground.saturating_sub(1) as u32)
                        {
                            let rect = egui::Rect::from_min_size(
                                egui::pos2(cx - half_w, cy),
                                egui::vec2(map::TILE_WIDTH * scale, map::TILE_HEIGHT * scale),
                            );
                            let uv =
                                egui::Rect::from_min_max(egui::pos2(u0, v0), egui::pos2(u1, v1));
                            let mut mesh = egui::Mesh::with_texture(texture.id());
                            mesh.add_rect_with_uv(rect, uv, egui::Color32::WHITE);
                            painter.add(egui::Shape::mesh(mesh));
                        }
                    }
                }

                let bottom_y = cy + map::TILE_HEIGHT * scale;

                if Self::is_rendered_wall(tile.left_wall) {
                    if let (Some(atlas), Some(texture)) = (wall_atlas, wall_texture) {
                        let wall_id = tile.left_wall as u32;
                        let sprite_h = atlas.sprite_height(wall_id);
                        if sprite_h > 0 {
                            if let Some((u0, v0, u1, v1)) = atlas.sprite_uv(wall_id) {
                                let rect = egui::Rect::from_min_max(
                                    egui::pos2(cx - half_w, bottom_y - sprite_h as f32 * scale),
                                    egui::pos2(cx, bottom_y),
                                );
                                let uv = egui::Rect::from_min_max(
                                    egui::pos2(u0, v0),
                                    egui::pos2(u1, v1),
                                );
                                let mut mesh = egui::Mesh::with_texture(texture.id());
                                mesh.add_rect_with_uv(rect, uv, egui::Color32::WHITE);
                                painter.add(egui::Shape::mesh(mesh));
                            }
                        }
                    }
                }

                if Self::is_rendered_wall(tile.right_wall) {
                    if let (Some(atlas), Some(texture)) = (wall_atlas, wall_texture) {
                        let wall_id = tile.right_wall as u32;
                        let sprite_h = atlas.sprite_height(wall_id);
                        if sprite_h > 0 {
                            if let Some((u0, v0, u1, v1)) = atlas.sprite_uv(wall_id) {
                                let rect = egui::Rect::from_min_max(
                                    egui::pos2(cx, bottom_y - sprite_h as f32 * scale),
                                    egui::pos2(cx + half_w, bottom_y),
                                );
                                let uv = egui::Rect::from_min_max(
                                    egui::pos2(u0, v0),
                                    egui::pos2(u1, v1),
                                );
                                let mut mesh = egui::Mesh::with_texture(texture.id());
                                mesh.add_rect_with_uv(rect, uv, egui::Color32::WHITE);
                                painter.add(egui::Shape::mesh(mesh));
                            }
                        }
                    }
                }
            }
        }
    }

    fn draw_prefab_browser(
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        prefabs: &[PrefabAsset],
        selected_prefab_index: Option<usize>,
        prefab_search: &mut String,
        renaming_prefab_index: Option<usize>,
        prefab_rename_buffer: &mut String,
        prefab_rename_should_focus: &mut bool,
        response: &mut InspectorResponse,
        max_list_height: f32,
    ) {
        ui.horizontal(|ui| {
            if ui.button("New").clicked() {
                response.new_prefab_requested = true;
            }
            if ui.button("Import").clicked() {
                response.import_prefab_requested = true;
            }
        });
        ui.add_space(8.0);
        ui.add(
            egui::TextEdit::singleline(prefab_search)
                .hint_text("Search prefabs...")
                .desired_width(f32::INFINITY),
        );
        ui.add_space(8.0);

        let search = prefab_search.trim().to_lowercase();
        let filtered = prefabs
            .iter()
            .enumerate()
            .filter(|(_, prefab)| {
                search.is_empty() || prefab.file_stem_name().to_lowercase().contains(&search)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();

        let list_height = max_list_height.min(ui.available_height()).max(1.0);
        egui::Frame::NONE
            .fill(colors.bg_3)
            .stroke(egui::Stroke::new(1.0, colors.border))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                Self::draw_prefab_list_header(ui, colors);

                let body_height = (list_height - PREFAB_LIST_HEADER_HEIGHT).max(1.0);
                if prefabs.is_empty() {
                    Self::draw_prefab_empty_state(ui, colors, body_height, "No prefabs available");
                    return;
                }

                if filtered.is_empty() {
                    Self::draw_prefab_empty_state(
                        ui,
                        colors,
                        body_height,
                        "No prefabs match the current search.",
                    );
                    return;
                }

                egui::ScrollArea::vertical()
                    .id_salt("prefab_list_scroll")
                    .max_height(body_height)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        for index in filtered {
                            let prefab = &prefabs[index];
                            let selected = Some(index) == selected_prefab_index;
                            let row_output = Self::draw_prefab_list_row(
                                ui,
                                colors,
                                prefab,
                                selected,
                                Some(index) == renaming_prefab_index,
                                prefab_rename_buffer,
                                prefab_rename_should_focus,
                            );
                            if row_output.response.clicked() {
                                response.select_prefab_index = Some(index);
                            }
                            if row_output.response.double_clicked() {
                                response.edit_prefab_index = Some(index);
                            }
                            if row_output.response.secondary_clicked() {
                                response.select_prefab_index = Some(index);
                            }
                            if row_output.rename_submitted {
                                response.submit_prefab_rename = true;
                            }
                            if row_output.rename_canceled {
                                response.cancel_prefab_rename = true;
                            }
                            if Some(index) != renaming_prefab_index {
                                row_output.response.context_menu(|ui| {
                                    ui.set_min_width(140.0);
                                    if ui.button("Duplicate Prefab").clicked() {
                                        response.select_prefab_index = Some(index);
                                        response.duplicate_prefab_index = Some(index);
                                        ui.close();
                                    }
                                    if ui.button("Rename Prefab...").clicked() {
                                        response.select_prefab_index = Some(index);
                                        response.start_rename_prefab_index = Some(index);
                                        ui.close();
                                    }
                                    ui.separator();
                                    if ui
                                        .button(
                                            egui::RichText::new("Delete Prefab")
                                                .color(colors.accent),
                                        )
                                        .clicked()
                                    {
                                        response.select_prefab_index = Some(index);
                                        response.delete_prefab_index = Some(index);
                                        ui.close();
                                    }
                                });
                            }
                        }
                    });
            });
    }

    fn draw_prefab_list_header(ui: &mut egui::Ui, colors: &ThemeColors) {
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(width, PREFAB_LIST_HEADER_HEIGHT),
            egui::Sense::hover(),
        );

        ui.painter().rect_filled(rect, 0.0, colors.bg_2);
        ui.painter().line_segment(
            [
                egui::pos2(rect.left(), rect.bottom()),
                egui::pos2(rect.right(), rect.bottom()),
            ],
            egui::Stroke::new(1.0, colors.border),
        );
        ui.painter().text(
            egui::pos2(rect.left() + PREFAB_LIST_CELL_PAD_X, rect.center().y),
            egui::Align2::LEFT_CENTER,
            "Prefab",
            egui::FontId::proportional(12.0),
            colors.muted,
        );
        ui.painter().text(
            egui::pos2(rect.right() - PREFAB_LIST_CELL_PAD_X, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            "Size",
            egui::FontId::monospace(11.5),
            colors.muted,
        );
    }

    fn draw_prefab_empty_state(
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        height: f32,
        message: &str,
    ) {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), height),
            egui::Sense::hover(),
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            message,
            egui::FontId::proportional(13.0),
            colors.muted,
        );
    }

    fn draw_prefab_list_row(
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        prefab: &PrefabAsset,
        selected: bool,
        editing: bool,
        rename_buffer: &mut String,
        rename_should_focus: &mut bool,
    ) -> PrefabListRowOutput {
        let width = ui.available_width();
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(width, PREFAB_LIST_ROW_HEIGHT),
            egui::Sense::click(),
        );

        let fill = if selected {
            colors.accent.gamma_multiply(0.14)
        } else if response.hovered() {
            colors.panel_2.gamma_multiply(0.65)
        } else {
            egui::Color32::TRANSPARENT
        };
        if fill != egui::Color32::TRANSPARENT {
            ui.painter().rect_filled(rect, 0.0, fill);
        }

        if selected {
            let accent_bar = egui::Rect::from_min_max(
                rect.left_top(),
                egui::pos2(rect.left() + 3.0, rect.bottom()),
            );
            ui.painter().rect_filled(accent_bar, 0.0, colors.accent);
        }

        ui.painter().line_segment(
            [
                egui::pos2(rect.left(), rect.bottom()),
                egui::pos2(rect.right(), rect.bottom()),
            ],
            egui::Stroke::new(1.0, colors.border.gamma_multiply(0.8)),
        );

        let (width, height) = prefab.occupied_dimensions();
        let size_label = format!("{width}x{height}");
        let name_space =
            (rect.width() - PREFAB_LIST_CELL_PAD_X * 2.0 - PREFAB_LIST_SIZE_COLUMN_WIDTH).max(8.0);
        let max_name_chars = (name_space / 7.2).floor().max(1.0) as usize;
        let name_label = Self::ellipsize_tail(prefab.file_stem_name(), max_name_chars);
        let name_color = if selected {
            colors.text
        } else {
            colors.text.gamma_multiply(0.92)
        };
        let size_color = if selected {
            colors.accent
        } else {
            colors.muted
        };

        ui.painter().text(
            egui::pos2(rect.right() - PREFAB_LIST_CELL_PAD_X, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            size_label,
            egui::FontId::monospace(12.0),
            size_color,
        );

        let mut rename_submitted = false;
        let mut rename_canceled = false;
        if editing {
            let text_edit_id = response.id.with("rename");
            let edit_right =
                (rect.right() - PREFAB_LIST_CELL_PAD_X - PREFAB_LIST_SIZE_COLUMN_WIDTH - 8.0)
                    .max(rect.left() + PREFAB_LIST_CELL_PAD_X + 48.0);
            let edit_rect = egui::Rect::from_min_max(
                egui::pos2(rect.left() + PREFAB_LIST_CELL_PAD_X - 4.0, rect.top() + 3.0),
                egui::pos2(edit_right, rect.bottom() - 3.0),
            );
            let original_name = prefab.file_stem_name();
            let text_response = ui.put(
                edit_rect,
                egui::TextEdit::singleline(rename_buffer)
                    .id(text_edit_id)
                    .desired_width(edit_rect.width())
                    .margin(egui::Margin::symmetric(6, 4)),
            );
            if *rename_should_focus {
                text_response.request_focus();
                Self::select_all_text(ui, text_edit_id, rename_buffer);
                *rename_should_focus = false;
            }

            let escape_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));
            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
            let lost_focus = text_response.lost_focus();
            let changed = rename_buffer.trim() != original_name;
            let escape_cancel = escape_pressed && (text_response.has_focus() || lost_focus);
            let enter_commit = enter_pressed && (text_response.has_focus() || lost_focus);
            let focus_loss_commit = lost_focus && !escape_pressed && !enter_pressed && changed;
            let focus_loss_cancel = lost_focus && !escape_pressed && !enter_pressed && !changed;

            rename_canceled = escape_cancel || focus_loss_cancel;
            rename_submitted = !rename_canceled && (enter_commit || focus_loss_commit);
        } else {
            ui.painter().text(
                egui::pos2(rect.left() + PREFAB_LIST_CELL_PAD_X, rect.center().y),
                egui::Align2::LEFT_CENTER,
                name_label,
                egui::FontId::proportional(13.0),
                name_color,
            );
        }

        PrefabListRowOutput {
            response,
            rename_submitted,
            rename_canceled,
        }
    }

    fn ellipsize_tail(text: &str, max_chars: usize) -> String {
        if max_chars == 0 {
            return String::new();
        }

        let char_count = text.chars().count();
        if char_count <= max_chars {
            return text.to_string();
        }

        if max_chars <= 3 {
            return ".".repeat(max_chars);
        }

        let visible = max_chars - 3;
        let truncated = text.chars().take(visible).collect::<String>();
        format!("{truncated}...")
    }

    fn draw_selected_prefab_preview(
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        tile_atlas: Option<&render::TileAtlas>,
        atlas_texture: Option<&egui::TextureHandle>,
        wall_atlas: Option<&render::SpriteAtlas>,
        wall_texture: Option<&egui::TextureHandle>,
        prefabs: &[PrefabAsset],
        selected_prefab_index: Option<usize>,
    ) {
        let Some(index) = selected_prefab_index.filter(|index| *index < prefabs.len()) else {
            Self::draw_prefab_preview_empty(ui, colors, "No prefab selected");
            return;
        };

        let prefab = &prefabs[index];
        Self::draw_prefab_preview(
            ui,
            colors,
            prefab,
            tile_atlas,
            atlas_texture,
            wall_atlas,
            wall_texture,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_prefab_preview(
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        prefab: &PrefabAsset,
        tile_atlas: Option<&render::TileAtlas>,
        atlas_texture: Option<&egui::TextureHandle>,
        wall_atlas: Option<&render::SpriteAtlas>,
        wall_texture: Option<&egui::TextureHandle>,
    ) {
        let preview_size = egui::vec2(ui.available_width(), ui.available_height().max(1.0));
        let (rect, _) = ui.allocate_exact_size(preview_size, egui::Sense::hover());
        let painter = ui.painter_at(rect);

        painter.rect_filled(rect, 0.0, colors.panel_2.gamma_multiply(0.35));
        painter.line_segment(
            [
                egui::pos2(rect.left(), rect.top()),
                egui::pos2(rect.right(), rect.top()),
            ],
            egui::Stroke::new(1.0, colors.border),
        );

        let Some(bounds) = prefab.occupied_bounds() else {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Prefab is empty.",
                egui::FontId::proportional(13.0),
                colors.muted,
            );
            return;
        };

        let Some(world_bounds) = Self::prefab_render_bounds(&prefab.map, bounds, wall_atlas) else {
            return;
        };

        let fit_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), rect.top() + PREFAB_PREVIEW_TOP_PAD),
            egui::pos2(
                rect.right(),
                (rect.bottom() - PREFAB_PREVIEW_BOTTOM_PAD)
                    .max(rect.top() + PREFAB_PREVIEW_TOP_PAD + 1.0),
            ),
        );
        let fit_w = fit_rect.width().max(1.0);
        let fit_h = fit_rect.height().max(1.0);
        let scale_x = fit_w / world_bounds.width();
        let scale_y = fit_h / world_bounds.height();
        let scale = scale_x.min(scale_y).clamp(0.15, 8.0);

        let offset = egui::vec2(
            fit_rect.center().x - (world_bounds.min.x + world_bounds.max.x) * 0.5 * scale,
            fit_rect.center().y - (world_bounds.min.y + world_bounds.max.y) * 0.5 * scale,
        );

        Self::draw_prefab_render_scene(
            &painter,
            &prefab.map,
            bounds,
            scale,
            offset,
            tile_atlas,
            atlas_texture,
            wall_atlas,
            wall_texture,
        );

        if tile_atlas.is_none() && wall_atlas.is_none() {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No render assets loaded",
                egui::FontId::proportional(13.0),
                colors.muted,
            );
        }
    }

    fn draw_prefab_preview_empty(ui: &mut egui::Ui, colors: &ThemeColors, text: &str) {
        let preview_size = egui::vec2(ui.available_width(), ui.available_height().max(1.0));
        let (rect, _) = ui.allocate_exact_size(preview_size, egui::Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, colors.panel_2.gamma_multiply(0.35));
        painter.line_segment(
            [
                egui::pos2(rect.left(), rect.top()),
                egui::pos2(rect.right(), rect.top()),
            ],
            egui::Stroke::new(1.0, colors.border),
        );
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(13.0),
            colors.muted,
        );
    }
}
