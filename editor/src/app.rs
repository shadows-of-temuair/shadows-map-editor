use std::{collections::VecDeque, fs, path::Path, path::PathBuf};

use eframe::egui;
use tracing::{info, warn};

use crate::document::{DocumentKind, LayerVisibility, MapDocument, PaintLayer};
use crate::map_list::{MapList, MapMetadataHint};
use crate::palette_lookup::LoadedPaletteLookup;
use crate::panels::{
    ExportDialog, ExportDialogAction, EyedropperPick, InspectorPanel, InspectorResponse,
    MapSizeDialog, PrefabDeleteDialog, PrefabDeleteDialogAction, StatusBarAction, StatusBarPanel,
    TabBarAction, TabBarPanel, TitleBarPanel, Tool, ToolbarAction, ToolbarPanel,
    UnsavedChangesDialog, UnsavedChangesDialogAction, ViewportPanel, WindowFrame,
};
use crate::prefab::{self, PrefabAsset};
use crate::shape::{self, ShapeKind};
use crate::theme;

enum PendingDiscardAction {
    CloseTab { index: usize },
    CloseWindow { remaining_docs: Vec<usize> },
}

struct PendingUnsavedChanges {
    document_index: usize,
    action: PendingDiscardAction,
}

struct PendingPrefabDeletion {
    path: PathBuf,
    name: String,
}

pub struct EditorApp {
    documents: Vec<MapDocument>,
    active_tab: usize,
    active_tool: Tool,
    active_shape_kind: ShapeKind,
    active_paint_layer: PaintLayer,
    last_wall_paint_layer: PaintLayer,
    tab_bar: TabBarPanel,
    status_bar: StatusBarPanel,
    layer_visibility: LayerVisibility,
    show_grid: bool,
    show_collision_overlay: bool,
    tile_atlas: Option<render::TileAtlas>,
    atlas_texture: Option<egui::TextureHandle>,
    wall_atlas: Option<render::SpriteAtlas>,
    wall_texture: Option<egui::TextureHandle>,
    tab_overlay_texture: Option<egui::TextureHandle>,
    sotp_data: Option<Vec<u8>>,
    map_list: Option<MapList>,
    prefab_assets: Vec<PrefabAsset>,
    selected_prefab: Option<usize>,
    prefab_search: String,
    hover_tile: (u16, u16),
    selected_tile: Option<(u16, u16)>,
    selected_ground_tile: u16,
    selected_wall_tile: u16,
    reveal_ground_tile_in_palette: Option<u16>,
    reveal_wall_tile_in_palette: Option<u16>,
    last_pencil_click_tile: Option<(u16, u16)>,
    line_tool_start_tile: Option<(u16, u16)>,
    shape_tool_start_tile: Option<(u16, u16)>,
    export_dialog: ExportDialog,
    new_map_size_dialog: MapSizeDialog,
    pending_new_document_kind: DocumentKind,
    unsaved_changes_dialog: UnsavedChangesDialog,
    pending_unsaved_changes: Option<PendingUnsavedChanges>,
    prefab_delete_dialog: PrefabDeleteDialog,
    pending_prefab_deletion: Option<PendingPrefabDeletion>,
    allow_window_close: bool,
    status_message: String,
    atlas_needs_upload: bool,
    wall_atlas_needs_upload: bool,
    tab_overlay_texture_needs_upload: bool,
}

impl EditorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::widgets::icons::install_font(&cc.egui_ctx);
        theme::apply_theme(&cc.egui_ctx);

        // Build atlases CPU-side; texture uploads are deferred to first frame
        // because the renderer hasn't reported the real GPU max texture size yet.
        let (tile_atlas, wall_atlas, sotp_data) = Self::load_assets();
        let map_list = MapList::load_if_exists("maps.ron");
        let prefab_assets = prefab::load_prefab_assets().unwrap_or_default();
        let selected_prefab = (!prefab_assets.is_empty()).then_some(0);
        let selected_ground_tile = if tile_atlas.is_some() { 1 } else { 0 };
        let selected_wall_tile = wall_atlas
            .as_ref()
            .and_then(|atlas| {
                (1..atlas.sprite_count())
                    .find(|&id| atlas.sprite_rect(id).is_some())
                    .map(|id| id.min(u16::MAX as u32) as u16)
            })
            .unwrap_or(0);

        Self {
            documents: vec![MapDocument::new(50, 50)],
            active_tab: 0,
            active_tool: Tool::Pencil,
            active_shape_kind: ShapeKind::Rect,
            active_paint_layer: PaintLayer::Ground,
            last_wall_paint_layer: PaintLayer::LeftWall,
            tab_bar: TabBarPanel::default(),
            status_bar: StatusBarPanel::default(),
            layer_visibility: LayerVisibility::default(),
            show_grid: true,
            show_collision_overlay: false,
            atlas_needs_upload: tile_atlas.is_some(),
            wall_atlas_needs_upload: wall_atlas.is_some(),
            tile_atlas,
            wall_atlas,
            sotp_data,
            map_list,
            prefab_assets,
            selected_prefab,
            prefab_search: String::new(),
            atlas_texture: None,
            wall_texture: None,
            tab_overlay_texture: None,
            hover_tile: (0, 0),
            selected_tile: None,
            selected_ground_tile,
            selected_wall_tile,
            reveal_ground_tile_in_palette: None,
            reveal_wall_tile_in_palette: None,
            last_pencil_click_tile: None,
            line_tool_start_tile: None,
            shape_tool_start_tile: None,
            export_dialog: ExportDialog::default(),
            new_map_size_dialog: MapSizeDialog::default(),
            pending_new_document_kind: DocumentKind::Map,
            unsaved_changes_dialog: UnsavedChangesDialog::default(),
            pending_unsaved_changes: None,
            prefab_delete_dialog: PrefabDeleteDialog::default(),
            pending_prefab_deletion: None,
            allow_window_close: false,
            status_message: String::from("Ready"),
            tab_overlay_texture_needs_upload: true,
        }
    }

    fn clear_edit_anchors(&mut self) {
        self.last_pencil_click_tile = None;
        self.line_tool_start_tile = None;
        self.shape_tool_start_tile = None;
    }

    fn map_hint_for_path(&self, path: &std::path::Path) -> Option<MapMetadataHint> {
        self.map_list
            .as_ref()
            .and_then(|map_list| map_list.hint_for_path(path))
    }

    fn reload_prefabs(&mut self) {
        let selected_path = self
            .selected_prefab
            .and_then(|index| self.prefab_assets.get(index))
            .map(|prefab| prefab.path.clone());

        match prefab::load_prefab_assets() {
            Ok(prefabs) => {
                self.prefab_assets = prefabs;
                self.selected_prefab = selected_path
                    .as_ref()
                    .and_then(|path| {
                        self.prefab_assets
                            .iter()
                            .position(|prefab| &prefab.path == path)
                    })
                    .or_else(|| (!self.prefab_assets.is_empty()).then_some(0));
            }
            Err(error) => {
                warn!("Failed to reload prefabs: {}", error);
                self.status_message = format!("Prefab reload failed: {}", error);
            }
        }
    }

    fn select_prefab_by_path(&mut self, path: &std::path::Path) {
        self.selected_prefab = self
            .prefab_assets
            .iter()
            .position(|prefab| prefab.path == path);
    }

    fn selected_prefab_map(&self) -> Option<&map::Map> {
        self.selected_prefab
            .and_then(|index| self.prefab_assets.get(index))
            .map(|prefab| &prefab.map)
    }

    fn set_active_tool(&mut self, tool: Tool) {
        if self.active_tool != tool {
            self.active_tool = tool;
            self.line_tool_start_tile = None;
            self.shape_tool_start_tile = None;
        }
    }

    fn set_active_paint_layer(&mut self, layer: PaintLayer) {
        if self.active_paint_layer == layer {
            return;
        }
        self.documents[self.active_tab].finish_stroke();
        if matches!(layer, PaintLayer::LeftWall | PaintLayer::RightWall) {
            self.last_wall_paint_layer = layer;
        }
        self.active_paint_layer = layer;
        self.clear_edit_anchors();
    }

    fn selected_tile_for_layer(&self, layer: PaintLayer) -> u16 {
        match layer {
            PaintLayer::Ground => self.selected_ground_tile,
            PaintLayer::LeftWall | PaintLayer::RightWall => self.selected_wall_tile,
        }
    }

    fn tile_value_for_layer(tile: &map::Tile, layer: PaintLayer) -> u16 {
        match layer {
            PaintLayer::Ground => tile.ground,
            PaintLayer::LeftWall => tile.left_wall,
            PaintLayer::RightWall => tile.right_wall,
        }
    }

    fn get_pool_asset_case_insensitive<'a>(
        pool: &'a archive::AssetPool,
        name: &str,
    ) -> Option<&'a [u8]> {
        pool.get(name).or_else(|| {
            let actual_name = pool
                .names()
                .find(|entry| entry.eq_ignore_ascii_case(name))?;
            pool.get(actual_name)
        })
    }

    /// Load all assets from the archive: tile atlas, wall sprite atlas, and SOTP collision data.
    fn load_assets() -> (
        Option<render::TileAtlas>,
        Option<render::SpriteAtlas>,
        Option<Vec<u8>>,
    ) {
        let assets_dir = PathBuf::from("assets");

        let pool = match archive::AssetPool::load(&assets_dir) {
            Ok(pool) => pool,
            Err(e) => {
                warn!(
                    "Could not load asset archives from {}: {}",
                    assets_dir.display(),
                    e
                );
                return (None, None, None);
            }
        };

        let legacy_palette = match Self::get_pool_asset_case_insensitive(&pool, "legend.pal") {
            Some(data) => match render::Palette::from_bytes(data) {
                Ok(p) => Some(p),
                Err(e) => {
                    warn!("Failed to parse legend.pal: {}", e);
                    None
                }
            },
            None => {
                warn!("legend.pal not found in asset archives");
                None
            }
        };

        let ground_palette_lookup = LoadedPaletteLookup::from_pool(&pool, "mpt");
        if let Some(lookup) = &ground_palette_lookup {
            info!(
                "Detected mpt palette-table assets ({} palettes, {} mappings)",
                lookup.palette_count(),
                lookup.mapping_count()
            );
        }
        let wall_palette_lookup = LoadedPaletteLookup::from_pool(&pool, "stc");
        if let Some(lookup) = &wall_palette_lookup {
            info!(
                "Detected stc palette-table assets ({} palettes, {} mappings)",
                lookup.palette_count(),
                lookup.mapping_count()
            );
        }

        // Ground tile atlas
        let tile_atlas = match Self::get_pool_asset_case_insensitive(&pool, "TILEA.BMP") {
            Some(tile_data) => {
                let atlas_result = if let Some(lookup) = ground_palette_lookup.as_ref() {
                    match legacy_palette
                        .as_ref()
                        .or_else(|| lookup.fallback_palette())
                    {
                        Some(default_palette) => {
                            info!("Rendering TILEA.BMP using mpt palette-table mode");
                            Some(render::TileAtlas::from_raw_with_tile_palette(
                                tile_data,
                                56,
                                27,
                                |tile_index| {
                                    lookup
                                        .palette_for_id(tile_index + 2)
                                        .unwrap_or(default_palette)
                                },
                            ))
                        }
                        None => {
                            warn!("No fallback palette available for mpt palette-table rendering");
                            None
                        }
                    }
                } else if let Some(palette) = legacy_palette.as_ref() {
                    info!("Rendering TILEA.BMP using legacy legend palette");
                    Some(render::TileAtlas::from_raw(tile_data, palette, 56, 27))
                } else {
                    warn!("No usable palette found for TILEA.BMP");
                    None
                };

                match atlas_result {
                    Some(Ok(atlas)) => {
                        let (w, h) = atlas.dimensions();
                        info!(
                            "Built tile atlas: {}x{} ({} tiles)",
                            w,
                            h,
                            atlas.tile_count()
                        );
                        Some(atlas)
                    }
                    Some(Err(e)) => {
                        warn!("Failed to build tile atlas: {}", e);
                        None
                    }
                    None => None,
                }
            }
            None => {
                warn!("TILEA.BMP not found in asset archives");
                None
            }
        };

        // Wall sprite atlas from HPF files
        let wall_atlas = if let Some(lookup) = wall_palette_lookup.as_ref() {
            match legacy_palette
                .as_ref()
                .or_else(|| lookup.fallback_palette())
            {
                Some(default_palette) => {
                    info!("Rendering STC wall sprites using stc palette-table mode");
                    Self::load_wall_atlas(&pool, |wall_id| {
                        lookup
                            .palette_for_id(wall_id + 1)
                            .unwrap_or(default_palette)
                    })
                }
                None => {
                    warn!("No fallback palette available for stc palette-table rendering");
                    None
                }
            }
        } else if let Some(palette) = legacy_palette.as_ref() {
            info!("Rendering STC wall sprites using legacy legend palette");
            Self::load_wall_atlas(&pool, |_| palette)
        } else {
            warn!("No usable palette found for STC wall sprite rendering");
            None
        };

        // SOTP collision data
        let sotp_data = Self::get_pool_asset_case_insensitive(&pool, "SOTP.DAT").map(|data| {
            info!("Loaded SOTP.DAT ({} bytes)", data.len());
            data.to_vec()
        });
        if sotp_data.is_none() {
            warn!("SOTP.DAT not found in asset archives");
        }

        (tile_atlas, wall_atlas, sotp_data)
    }

    /// Probe for `stcNNNNN.hpf` files in the asset pool, decode them, and
    /// pack into a single sprite atlas.
    fn load_wall_atlas<'a, F>(
        pool: &archive::AssetPool,
        mut palette_for_wall: F,
    ) -> Option<render::SpriteAtlas>
    where
        F: FnMut(u32) -> &'a render::Palette,
    {
        let mut sprites: Vec<Option<render::HpfSprite>> = Vec::new();
        let mut last_found = 0u32;
        let mut found_count = 0u32;

        // Wall IDs are 1-indexed: tile wall_id=741 maps to stc00741.hpf.
        // We load starting from 0 to fill the array; index 0 is unused.
        for id in 0..=99_999u32 {
            // Try lowercase first, then uppercase (archive names may vary)
            let name_lower = format!("stc{:05}.hpf", id);
            let name_upper = format!("STC{:05}.HPF", id);
            let data = pool.get(&name_lower).or_else(|| pool.get(&name_upper));

            match data {
                Some(bytes) => match render::HpfSprite::decode(bytes) {
                    Ok(sprite) => {
                        while sprites.len() <= id as usize {
                            sprites.push(None);
                        }
                        sprites[id as usize] = Some(sprite);
                        last_found = id;
                        found_count += 1;
                    }
                    Err(e) => {
                        warn!("Failed to decode {}: {}", name_lower, e);
                        while sprites.len() <= id as usize {
                            sprites.push(None);
                        }
                    }
                },
                None => {
                    // Allow gaps, but stop after 100 consecutive misses past last found
                    if id > last_found + 100 && found_count > 0 {
                        break;
                    }
                    while sprites.len() <= id as usize {
                        sprites.push(None);
                    }
                }
            }
        }

        if found_count == 0 {
            warn!("No HPF wall sprites found in asset archives");
            return None;
        }

        match render::SpriteAtlas::build_with_sprite_palette(&sprites, 28, |wall_id| {
            palette_for_wall(wall_id)
        }) {
            Ok(atlas) => {
                let (w, h) = atlas.dimensions();
                info!(
                    "Built wall sprite atlas: {}x{} ({} sprites loaded, {} entries)",
                    w,
                    h,
                    found_count,
                    atlas.sprite_count()
                );
                Some(atlas)
            }
            Err(e) => {
                warn!("Failed to build wall sprite atlas: {}", e);
                None
            }
        }
    }

    /// Upload the tile atlas texture to the GPU on the first frame.
    fn try_upload_atlas(&mut self, ctx: &egui::Context) {
        if !self.atlas_needs_upload {
            return;
        }
        self.atlas_needs_upload = false;

        let atlas = match &self.tile_atlas {
            Some(a) => a,
            None => return,
        };

        let (w, h) = atlas.dimensions();
        let max_side = ctx.input(|i| i.max_texture_side);

        if w as usize > max_side || h as usize > max_side {
            warn!(
                "Tile atlas {}x{} exceeds GPU max texture size {}; tiles will not render",
                w, h, max_side
            );
            return;
        }

        let texture = ctx.load_texture(
            "tile_atlas",
            egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], atlas.pixels()),
            egui::TextureOptions::NEAREST,
        );
        info!("Uploaded tile atlas texture to GPU (max_side={})", max_side);
        self.atlas_texture = Some(texture);
    }

    /// Upload the wall sprite atlas texture to the GPU on the first frame.
    fn try_upload_wall_atlas(&mut self, ctx: &egui::Context) {
        if !self.wall_atlas_needs_upload {
            return;
        }
        self.wall_atlas_needs_upload = false;

        let atlas = match &self.wall_atlas {
            Some(a) => a,
            None => return,
        };

        let (w, h) = atlas.dimensions();
        let max_side = ctx.input(|i| i.max_texture_side);

        if w as usize > max_side || h as usize > max_side {
            warn!(
                "Wall atlas {}x{} exceeds GPU max texture size {}; walls will not render",
                w, h, max_side
            );
            return;
        }

        let texture = ctx.load_texture(
            "wall_atlas",
            egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], atlas.pixels()),
            egui::TextureOptions::NEAREST,
        );
        info!("Uploaded wall sprite atlas texture to GPU");
        self.wall_texture = Some(texture);
    }

    /// Upload a tiny repeating checker texture used for tab collision overlay fill.
    fn try_upload_tab_overlay_texture(&mut self, ctx: &egui::Context) {
        if !self.tab_overlay_texture_needs_upload {
            return;
        }
        self.tab_overlay_texture_needs_upload = false;

        // 2x2 checker: opaque-white and transparent texels.
        // Alpha 84 is ~33% opacity.
        let pixels: [u8; 16] = [255, 255, 255, 84, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 84];
        let texture = ctx.load_texture(
            "tab_overlay_checker",
            egui::ColorImage::from_rgba_unmultiplied([2, 2], &pixels),
            egui::TextureOptions::NEAREST_REPEAT,
        );
        self.tab_overlay_texture = Some(texture);
    }

    fn open_new_document_dialog(&mut self, kind: DocumentKind) {
        self.documents[self.active_tab].finish_stroke();
        self.pending_new_document_kind = kind;
        let (width, height) = match kind {
            DocumentKind::Map => {
                let map = &self.documents[self.active_tab].map;
                (map.width, map.height)
            }
            DocumentKind::Prefab => (6, 6),
        };
        self.new_map_size_dialog.open(width, height);
    }

    fn new_document(&mut self) {
        self.open_new_document_dialog(DocumentKind::Map);
    }

    fn new_prefab_document(&mut self) {
        self.open_new_document_dialog(DocumentKind::Prefab);
    }

    fn create_document_with_dimensions(&mut self, width: u16, height: u16) {
        if width == 0 || height == 0 {
            self.status_message = "Width and height must both be at least 1.".to_string();
            return;
        }
        let kind = self.pending_new_document_kind;
        let document = match kind {
            DocumentKind::Map => MapDocument::new_map(width, height),
            DocumentKind::Prefab => MapDocument::new_prefab(width, height),
        };
        self.documents.push(document);
        self.active_tab = self.documents.len() - 1;
        self.clear_edit_anchors();
        self.status_message = format!("Created new {} {}x{}", kind.noun(), width, height);
    }

    fn open_document(&mut self) {
        self.open_map_document();
    }

    fn open_map_document(&mut self) {
        self.documents[self.active_tab].finish_stroke();
        let file = rfd::FileDialog::new()
            .add_filter("Map", &["map"])
            .pick_file();

        if let Some(path) = file {
            self.open_document_from_path(path);
        }
    }

    fn open_document_from_path(&mut self, path: PathBuf) {
        for (i, doc) in self.documents.iter().enumerate() {
            if doc.path.as_ref() == Some(&path) {
                self.active_tab = i;
                self.clear_edit_anchors();
                if path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("ron"))
                    .unwrap_or(false)
                {
                    self.reload_prefabs();
                    self.select_prefab_by_path(&path);
                }
                return;
            }
        }

        let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
        let opened = if ext.eq_ignore_ascii_case("map") {
            let hint = self.map_hint_for_path(&path);
            MapDocument::open_map(path.clone(), hint)
        } else if ext.eq_ignore_ascii_case("ron") {
            MapDocument::open_prefab(path.clone())
        } else {
            return;
        };

        match opened {
            Ok(doc) => {
                let kind = doc.kind();
                self.documents.push(doc);
                self.active_tab = self.documents.len() - 1;
                self.clear_edit_anchors();
                if kind == DocumentKind::Prefab {
                    self.reload_prefabs();
                    self.select_prefab_by_path(&path);
                }
                info!("Opened {}: {}", kind.noun(), path.display());
                self.status_message = format!(
                    "Opened {}.",
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or(kind.noun())
                );
            }
            Err(error) => {
                warn!("Failed to open {}: {}", path.display(), error);
                self.status_message = format!("Open failed: {}", error);
            }
        }
    }

    fn import_prefab_into_registry(&mut self) {
        self.documents[self.active_tab].finish_stroke();
        let file = rfd::FileDialog::new()
            .add_filter("Prefab", &["ron"])
            .pick_file();

        let Some(source_path) = file else {
            return;
        };

        if let Err(error) = prefab::load_prefab_asset(&source_path) {
            self.status_message = format!("Import failed: {}", error);
            return;
        }

        let prefabs_dir =
            prefab::ensure_prefabs_dir().unwrap_or_else(|_| PathBuf::from(prefab::PREFABS_DIR));
        let filename = source_path
            .file_name()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("imported-prefab.ron"));
        let destination = prefabs_dir.join(filename);

        let same_file = Self::paths_match(&source_path, &destination);
        if !same_file {
            if let Err(error) = fs::copy(&source_path, &destination) {
                self.status_message = format!("Import failed: {}", error);
                return;
            }
        }

        self.reload_prefabs();
        self.select_prefab_by_path(&destination);
        self.status_message = format!(
            "Imported prefab {}.",
            destination
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("prefab")
        );
    }

    fn paths_match(a: &Path, b: &Path) -> bool {
        if a == b {
            return true;
        }

        match (a.canonicalize(), b.canonicalize()) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        }
    }

    fn handle_inspector_response(&mut self, response: InspectorResponse) {
        if let Some(index) = response
            .select_prefab_index
            .filter(|index| *index < self.prefab_assets.len())
        {
            self.selected_prefab = Some(index);
        }
        if response.new_prefab_requested {
            self.new_prefab_document();
        }
        if response.import_prefab_requested {
            self.import_prefab_into_registry();
        }
        if let Some(index) = response
            .edit_prefab_index
            .filter(|index| *index < self.prefab_assets.len())
        {
            if let Some(prefab) = self.prefab_assets.get(index) {
                self.open_document_from_path(prefab.path.clone());
            }
        }
        if let Some(index) = response
            .delete_prefab_index
            .filter(|index| *index < self.prefab_assets.len())
        {
            self.begin_prefab_delete_flow(index);
        }
    }

    fn begin_prefab_delete_flow(&mut self, index: usize) {
        let Some(prefab) = self.prefab_assets.get(index) else {
            return;
        };

        let name = prefab.file_stem_name().to_string();
        self.prefab_delete_dialog.open_for(&name);
        self.pending_prefab_deletion = Some(PendingPrefabDeletion {
            path: prefab.path.clone(),
            name,
        });
    }

    fn resolve_prefab_delete_action(&mut self, action: PrefabDeleteDialogAction) {
        match action {
            PrefabDeleteDialogAction::None => {}
            PrefabDeleteDialogAction::Cancel => {
                self.pending_prefab_deletion = None;
            }
            PrefabDeleteDialogAction::Delete => {
                let Some(pending) = self.pending_prefab_deletion.take() else {
                    return;
                };
                match self.delete_prefab(&pending.path, &pending.name) {
                    Ok(()) => {
                        self.status_message = format!("Deleted prefab {}.", pending.name);
                    }
                    Err(error) => {
                        self.status_message = error;
                    }
                }
            }
        }
    }

    fn delete_prefab(&mut self, path: &Path, name: &str) -> Result<(), String> {
        let mut open_indices = Vec::new();
        for (index, doc) in self.documents.iter().enumerate() {
            let Some(doc_path) = doc.path.as_ref() else {
                continue;
            };
            if !Self::paths_match(doc_path, path) {
                continue;
            }
            if doc.dirty {
                return Err(format!(
                    "Close or save open prefab tabs for {} before deleting it.",
                    name
                ));
            }
            open_indices.push(index);
        }

        for index in open_indices.into_iter().rev() {
            self.close_tab_now(index);
        }

        fs::remove_file(path)
            .map_err(|error| format!("Delete failed for {}: {}", path.display(), error))?;
        self.reload_prefabs();
        Ok(())
    }

    fn save_document_at(&mut self, index: usize, prompt_for_path: bool) -> bool {
        let Some(doc) = self.documents.get_mut(index) else {
            return false;
        };
        doc.finish_stroke();
        let path = if prompt_for_path {
            self.prompt_save_path_for_document(index)
        } else {
            self.documents[index]
                .path
                .clone()
                .or_else(|| self.prompt_save_path_for_document(index))
        };
        let Some(path) = path else {
            return false;
        };

        let filename = path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("map")
            .to_owned();

        let kind = self.documents[index].kind();
        match self.documents[index].save_as(path.clone()) {
            Ok(()) => {
                self.documents[index].clear_history();
                if kind == DocumentKind::Prefab {
                    self.reload_prefabs();
                    self.select_prefab_by_path(&path);
                }
                info!("Saved {}: {}", kind.noun(), path.display());
                self.status_message = format!("Saved {}.", filename);
                true
            }
            Err(e) => {
                warn!("Failed to save {}: {}", path.display(), e);
                self.status_message = format!("Save failed: {}", e);
                false
            }
        }
    }

    fn save_document(&mut self) {
        let _ = self.save_document_at(self.active_tab, false);
    }

    fn save_document_as(&mut self) {
        let _ = self.save_document_at(self.active_tab, true);
    }

    fn prompt_save_path_for_document(&self, index: usize) -> Option<PathBuf> {
        let doc = &self.documents[index];
        let suggested = doc.suggested_filename();

        match doc.kind() {
            DocumentKind::Map => {
                let mut dialog = rfd::FileDialog::new()
                    .add_filter("Map", &["map"])
                    .set_file_name(&suggested);

                if let Some(parent) = doc.path.as_ref().and_then(|p| p.parent()) {
                    dialog = dialog.set_directory(parent);
                }

                dialog.save_file().map(Self::ensure_map_extension)
            }
            DocumentKind::Prefab => {
                let directory = prefab::ensure_prefabs_dir()
                    .unwrap_or_else(|_| PathBuf::from(prefab::PREFABS_DIR));
                rfd::FileDialog::new()
                    .add_filter("Prefab", &["ron"])
                    .set_directory(directory)
                    .set_file_name(&suggested)
                    .save_file()
                    .map(Self::ensure_prefab_save_path)
            }
        }
    }

    fn ensure_map_extension(mut path: PathBuf) -> PathBuf {
        let has_map_ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("map"))
            .unwrap_or(false);
        if !has_map_ext {
            path.set_extension("map");
        }
        path
    }

    fn ensure_prefab_save_path(mut path: PathBuf) -> PathBuf {
        let prefabs_dir =
            prefab::ensure_prefabs_dir().unwrap_or_else(|_| PathBuf::from(prefab::PREFABS_DIR));
        if !path.starts_with(&prefabs_dir) {
            let file_name = path
                .file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("untitled-prefab.ron"));
            path = prefabs_dir.join(file_name);
        }

        let has_ron_ext = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("ron"))
            .unwrap_or(false);
        if !has_ron_ext {
            path.set_extension("ron");
        }
        path
    }

    fn close_tab(&mut self, index: usize) {
        if self.pending_unsaved_changes.is_some() || index >= self.documents.len() {
            return;
        }
        self.documents[index].finish_stroke();
        if self.documents[index].dirty {
            self.begin_unsaved_changes_flow(PendingDiscardAction::CloseTab { index }, index);
            return;
        }
        self.close_tab_now(index);
    }

    fn close_tab_now(&mut self, index: usize) {
        self.documents[self.active_tab].finish_stroke();
        if self.documents.len() <= 1 {
            // Don't close the last tab — replace with a fresh document
            self.documents[0] = MapDocument::new(50, 50);
            self.active_tab = 0;
            self.clear_edit_anchors();
            return;
        }

        self.documents.remove(index);
        if self.active_tab > index {
            self.active_tab -= 1;
        } else if self.active_tab >= self.documents.len() {
            self.active_tab = self.documents.len() - 1;
        }
        self.clear_edit_anchors();
    }

    fn begin_unsaved_changes_flow(&mut self, action: PendingDiscardAction, document_index: usize) {
        let Some(doc) = self.documents.get(document_index) else {
            return;
        };
        self.unsaved_changes_dialog
            .open_for(&doc.tab_display_name());
        self.pending_unsaved_changes = Some(PendingUnsavedChanges {
            document_index,
            action,
        });
    }

    fn dirty_document_indices_for_window_close(&mut self) -> Vec<usize> {
        self.documents[self.active_tab].finish_stroke();

        let mut dirty_docs = Vec::new();
        if self.documents[self.active_tab].dirty {
            dirty_docs.push(self.active_tab);
        }
        dirty_docs.extend(
            self.documents
                .iter()
                .enumerate()
                .filter(|(index, doc)| *index != self.active_tab && doc.dirty)
                .map(|(index, _)| index),
        );
        dirty_docs
    }

    fn request_window_close(&mut self, ctx: &egui::Context) {
        if self.pending_unsaved_changes.is_some() || self.unsaved_changes_dialog.is_open() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            return;
        }

        let dirty_docs = self.dirty_document_indices_for_window_close();
        if dirty_docs.is_empty() {
            return;
        }

        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        self.begin_unsaved_changes_flow(
            PendingDiscardAction::CloseWindow {
                remaining_docs: dirty_docs.clone(),
            },
            dirty_docs[0],
        );
    }

    fn resolve_unsaved_changes_action(
        &mut self,
        ctx: &egui::Context,
        action: UnsavedChangesDialogAction,
    ) {
        match action {
            UnsavedChangesDialogAction::None => {}
            UnsavedChangesDialogAction::Cancel => {
                self.pending_unsaved_changes = None;
                self.allow_window_close = false;
            }
            UnsavedChangesDialogAction::Save => {
                let Some(pending) = self.pending_unsaved_changes.take() else {
                    return;
                };
                if self.save_document_at(pending.document_index, false) {
                    self.advance_pending_discard_action(ctx, pending);
                }
            }
            UnsavedChangesDialogAction::Discard => {
                let Some(pending) = self.pending_unsaved_changes.take() else {
                    return;
                };
                self.advance_pending_discard_action(ctx, pending);
            }
        }
    }

    fn advance_pending_discard_action(
        &mut self,
        ctx: &egui::Context,
        pending: PendingUnsavedChanges,
    ) {
        match pending.action {
            PendingDiscardAction::CloseTab { index } => {
                self.close_tab_now(index);
            }
            PendingDiscardAction::CloseWindow { mut remaining_docs } => {
                remaining_docs.retain(|&index| index != pending.document_index);
                if let Some(&next_index) = remaining_docs.first() {
                    self.begin_unsaved_changes_flow(
                        PendingDiscardAction::CloseWindow { remaining_docs },
                        next_index,
                    );
                } else {
                    self.allow_window_close = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
    }

    fn undo_active_document(&mut self) {
        let doc = &mut self.documents[self.active_tab];
        if doc.undo() {
            self.status_message = "Undo".to_string();
        }
    }

    fn redo_active_document(&mut self) {
        let doc = &mut self.documents[self.active_tab];
        if doc.redo() {
            self.status_message = "Redo".to_string();
        }
    }

    fn paint_layer_line(
        doc: &mut MapDocument,
        layer: PaintLayer,
        paint_value: u16,
        start: (u16, u16),
        end: (u16, u16),
    ) {
        doc.begin_layer_stroke(layer, paint_value);
        let (mut x0, mut y0) = (start.0 as i32, start.1 as i32);
        let (x1, y1) = (end.0 as i32, end.1 as i32);

        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if x0 >= 0 && y0 >= 0 {
                doc.paint_layer_stroke_tile(layer, x0 as u16, y0 as u16, paint_value);
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
        doc.finish_stroke();
    }

    fn paint_layer_shape(
        doc: &mut MapDocument,
        layer: PaintLayer,
        paint_value: u16,
        shape_kind: ShapeKind,
        start: (u16, u16),
        end: (u16, u16),
    ) {
        if paint_value == 0 {
            return;
        }

        let points = shape::outline_points(shape_kind, start, end);
        doc.begin_layer_stroke(layer, paint_value);
        for (x, y) in points {
            if x < 0 || y < 0 {
                continue;
            }
            let (x, y) = (x as u16, y as u16);
            if x < doc.map.width && y < doc.map.height {
                doc.paint_layer_stroke_tile(layer, x, y, paint_value);
            }
        }
        doc.finish_stroke();
    }

    fn paint_layer_fill(
        doc: &mut MapDocument,
        layer: PaintLayer,
        paint_value: u16,
        start: (u16, u16),
    ) {
        if paint_value == 0 || start.0 >= doc.map.width || start.1 >= doc.map.height {
            return;
        }

        let width = doc.map.width as usize;
        let height = doc.map.height as usize;
        let start_idx = start.1 as usize * width + start.0 as usize;
        let target_value = Self::tile_value_for_layer(&doc.map.tiles[start_idx], layer);

        if target_value == paint_value {
            return;
        }

        let mut visited = vec![false; width * height];
        let mut queue = VecDeque::new();
        visited[start_idx] = true;
        queue.push_back(start);

        doc.begin_layer_stroke(layer, paint_value);

        while let Some((col, row)) = queue.pop_front() {
            let idx = row as usize * width + col as usize;
            if Self::tile_value_for_layer(&doc.map.tiles[idx], layer) != target_value {
                continue;
            }

            doc.paint_layer_stroke_tile(layer, col, row, paint_value);

            let neighbors = [
                (col.checked_sub(1), Some(row)),
                (col.checked_add(1).filter(|&c| c < doc.map.width), Some(row)),
                (Some(col), row.checked_sub(1)),
                (
                    Some(col),
                    row.checked_add(1).filter(|&r| r < doc.map.height),
                ),
            ];

            for (ncol, nrow) in neighbors {
                let (Some(ncol), Some(nrow)) = (ncol, nrow) else {
                    continue;
                };

                let nidx = nrow as usize * width + ncol as usize;
                if visited[nidx] {
                    continue;
                }
                visited[nidx] = true;

                if Self::tile_value_for_layer(&doc.map.tiles[nidx], layer) == target_value {
                    queue.push_back((ncol, nrow));
                }
            }
        }

        doc.finish_stroke();
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let keyboard_captured =
            ctx.wants_keyboard_input() || ctx.memory(|memory| memory.focused().is_some());
        let (
            new,
            open,
            save,
            save_as,
            close,
            undo,
            redo,
            export,
            tool,
            toggle_ground_wall_layer,
            toggle_wall_target_side,
            toggle_layer,
            toggle_tab_overlay,
            keyboard_zoom,
            reset_zoom,
        ) = ctx.input(|i| {
            let cmd = i.modifiers.command;
            let shift = i.modifiers.shift;

            // Tool shortcuts (no modifiers)
            let tool = if !keyboard_captured && !cmd && !shift {
                if i.key_pressed(egui::Key::V) {
                    Some(Tool::Select)
                } else if i.key_pressed(egui::Key::B) {
                    Some(Tool::Pencil)
                } else if i.key_pressed(egui::Key::P) {
                    Some(Tool::Stamp)
                } else if i.key_pressed(egui::Key::G) {
                    Some(Tool::Fill)
                } else if i.key_pressed(egui::Key::L) {
                    Some(Tool::Line)
                } else if i.key_pressed(egui::Key::U) {
                    Some(Tool::Shape)
                } else if i.key_pressed(egui::Key::E) {
                    Some(Tool::Eraser)
                } else if i.key_pressed(egui::Key::I) {
                    Some(Tool::Eyedropper)
                } else {
                    None
                }
            } else {
                None
            };

            // Paint layer toggle (T): ground <-> remembered wall side.
            let toggle_ground_wall_layer =
                !keyboard_captured && !cmd && !shift && i.key_pressed(egui::Key::T);

            // Wall target toggle (Q): left <-> right when in wall mode.
            let toggle_wall_target_side =
                !keyboard_captured && !cmd && !shift && i.key_pressed(egui::Key::Q);

            // Layer toggle shortcuts (Cmd+1/2/3/4)
            let toggle_layer = if cmd && !shift {
                if i.key_pressed(egui::Key::Num1) {
                    Some(1)
                } else if i.key_pressed(egui::Key::Num2) {
                    Some(2)
                } else if i.key_pressed(egui::Key::Num3) {
                    Some(3)
                } else if i.key_pressed(egui::Key::Num4) {
                    Some(4)
                } else {
                    None
                }
            } else {
                None
            };

            // Collision overlay toggle (Tab)
            let toggle_tab_overlay =
                !keyboard_captured && !cmd && !shift && i.key_pressed(egui::Key::Tab);

            // Keyboard zoom (Cmd+/-, snap to 25%; Cmd+0 reset)
            let keyboard_zoom: Option<i8> = if cmd && !shift {
                if i.key_pressed(egui::Key::Minus) {
                    Some(-1)
                } else if i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals) {
                    Some(1)
                } else {
                    None
                }
            } else {
                None
            };
            let reset_zoom = cmd && !shift && i.key_pressed(egui::Key::Num0);
            let save = cmd && !shift && i.key_pressed(egui::Key::S);
            let save_as = cmd && shift && i.key_pressed(egui::Key::S);
            let undo = cmd && !shift && i.key_pressed(egui::Key::Z);
            let redo = cmd && shift && i.key_pressed(egui::Key::Z);

            (
                cmd && !shift && i.key_pressed(egui::Key::N),
                cmd && !shift && i.key_pressed(egui::Key::O),
                save,
                save_as,
                cmd && i.key_pressed(egui::Key::W),
                undo,
                redo,
                cmd && i.key_pressed(egui::Key::E),
                tool,
                toggle_ground_wall_layer,
                toggle_wall_target_side,
                toggle_layer,
                toggle_tab_overlay,
                keyboard_zoom,
                reset_zoom,
            )
        });

        if let Some(t) = tool {
            self.set_active_tool(t);
        }
        if toggle_ground_wall_layer {
            match self.active_paint_layer {
                PaintLayer::Ground => {
                    self.set_active_paint_layer(self.last_wall_paint_layer);
                    self.status_message = match self.active_paint_layer {
                        PaintLayer::LeftWall => "Paint layer: Wall (Left).".to_string(),
                        PaintLayer::RightWall => "Paint layer: Wall (Right).".to_string(),
                        PaintLayer::Ground => unreachable!(),
                    };
                }
                PaintLayer::LeftWall | PaintLayer::RightWall => {
                    self.set_active_paint_layer(PaintLayer::Ground);
                    self.status_message = "Paint layer: Ground.".to_string();
                }
            }
        }
        if toggle_wall_target_side {
            match self.active_paint_layer {
                PaintLayer::LeftWall => {
                    self.set_active_paint_layer(PaintLayer::RightWall);
                    self.status_message = "Wall target: Right".to_string();
                }
                PaintLayer::RightWall => {
                    self.set_active_paint_layer(PaintLayer::LeftWall);
                    self.status_message = "Wall target: Left".to_string();
                }
                PaintLayer::Ground => {}
            }
        }
        if let Some(layer) = toggle_layer {
            match layer {
                1 => self.layer_visibility.ground = !self.layer_visibility.ground,
                2 => self.layer_visibility.left_wall = !self.layer_visibility.left_wall,
                3 => self.layer_visibility.right_wall = !self.layer_visibility.right_wall,
                4 => self.show_grid = !self.show_grid,
                _ => {}
            }
        }
        let tab_shortcut_blocked = self.export_dialog.is_open()
            || self.new_map_size_dialog.is_open()
            || self.status_bar.is_size_dialog_open();
        if toggle_tab_overlay && !tab_shortcut_blocked {
            if self.sotp_data.is_some() {
                self.show_collision_overlay = !self.show_collision_overlay;
            } else {
                self.show_collision_overlay = false;
                self.status_message = "Collision overlay unavailable: SOTP.DAT not loaded.".into();
            }
        }
        if let Some(dir) = keyboard_zoom {
            Self::snap_zoom(&mut self.documents[self.active_tab].camera, dir);
        }
        if reset_zoom {
            let camera = &mut self.documents[self.active_tab].camera;
            let old_zoom = camera.zoom;
            if old_zoom != 1.0 {
                camera.offset *= 1.0 / old_zoom;
                camera.zoom = 1.0;
            }
        }
        if new {
            self.new_document();
        }
        if open {
            self.open_map_document();
        }
        if close {
            self.close_tab(self.active_tab);
        }
        if undo {
            self.undo_active_document();
        }
        if redo {
            self.redo_active_document();
        }
        if save_as {
            self.save_document_as();
        }
        if save {
            self.save_document();
        }
        if export {
            let doc_name = self.documents[self.active_tab].display_name();
            self.export_dialog.open_for(&doc_name);
        }
    }

    /// Handle files dropped onto the window from the OS.
    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped: Vec<_> = ctx.input(|i| i.raw.dropped_files.clone());
        for file in dropped {
            if let Some(path) = file.path {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let supported = ext.eq_ignore_ascii_case("map") || ext.eq_ignore_ascii_case("ron");
                if !supported {
                    continue;
                }
                self.open_document_from_path(path);
            }
        }
    }

    /// Snap zoom to the nearest 25% boundary in the given direction.
    fn snap_zoom(camera: &mut crate::document::Camera, dir: i8) {
        let old_zoom = camera.zoom;
        let pct = (old_zoom * 100.0).round();
        let new_pct = if dir > 0 {
            (pct / 25.0).floor() * 25.0 + 25.0
        } else {
            (pct / 25.0).ceil() * 25.0 - 25.0
        };
        let new_zoom = (new_pct / 100.0).clamp(0.25, 4.0);
        if new_zoom != old_zoom {
            camera.offset *= new_zoom / old_zoom;
            camera.zoom = new_zoom;
        }
    }
}

impl eframe::App for EditorApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.043, 0.047, 0.055, 1.0] // matches bg (#0b0c0e)
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Deferred atlas uploads
        self.try_upload_atlas(ctx);
        self.try_upload_wall_atlas(ctx);
        self.try_upload_tab_overlay_texture(ctx);

        let close_requested = ctx.input(|i| i.viewport().close_requested());
        if close_requested {
            if self.allow_window_close {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                self.request_window_close(ctx);
            }
        } else {
            self.allow_window_close = false;
        }

        WindowFrame::show(ctx);

        if !self.unsaved_changes_dialog.is_open() && !self.prefab_delete_dialog.is_open() {
            self.handle_keyboard_shortcuts(ctx);
            self.handle_dropped_files(ctx);
        }

        TitleBarPanel::show(ctx, frame);

        let tab_action = self.tab_bar.show(ctx, &self.documents, self.active_tab);
        match tab_action {
            TabBarAction::CloseTab(i) => self.close_tab(i),
            TabBarAction::SwitchTab(i) => {
                self.documents[self.active_tab].finish_stroke();
                self.active_tab = i;
                self.clear_edit_anchors();
                self.tab_bar.ensure_tab_visible(&self.documents, i);
            }
            TabBarAction::None => {}
        }

        let doc = &self.documents[self.active_tab];
        let can_undo = doc.can_undo();
        let can_redo = doc.can_redo();
        let current_zoom = doc.camera.zoom;
        let current_file_label = {
            let base = doc
                .path
                .as_ref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| doc.display_name());
            if doc.dirty { format!("{base} *") } else { base }
        };
        let alt_held = ctx.input(|i| i.modifiers.alt);
        let effective_tool = if alt_held {
            Tool::Eyedropper
        } else {
            self.active_tool
        };
        let status_action = self.status_bar.show(
            ctx,
            &doc.map,
            &current_file_label,
            effective_tool,
            self.hover_tile,
            current_zoom,
            &self.status_message,
        );
        match status_action {
            StatusBarAction::ZoomIn => {
                Self::snap_zoom(&mut self.documents[self.active_tab].camera, 1);
            }
            StatusBarAction::ZoomOut => {
                Self::snap_zoom(&mut self.documents[self.active_tab].camera, -1);
            }
            StatusBarAction::SetDimensions(w, h) => {
                let kind = self.documents[self.active_tab].kind();
                if let Err(err) = self.documents[self.active_tab].set_dimensions(w, h) {
                    self.status_message = err;
                } else {
                    self.status_message = format!("Resized {} to {}x{}", kind.noun(), w, h);
                }
            }
            StatusBarAction::None => {}
        }

        let toolbar_tool = if alt_held {
            Tool::Eyedropper
        } else {
            self.active_tool
        };
        let mut requested_tool = toolbar_tool;
        let toolbar_action = ToolbarPanel::show(
            ctx,
            &mut requested_tool,
            &mut self.active_shape_kind,
            can_undo,
            can_redo,
        );
        if !alt_held || requested_tool != toolbar_tool {
            self.set_active_tool(requested_tool);
        }
        match toolbar_action {
            ToolbarAction::NewFile => self.new_document(),
            ToolbarAction::OpenFile => self.open_document(),
            ToolbarAction::SaveFile => self.save_document(),
            ToolbarAction::Undo => self.undo_active_document(),
            ToolbarAction::Redo => self.redo_active_document(),
            ToolbarAction::Export => {
                let doc_name = self.documents[self.active_tab].display_name();
                self.export_dialog.open_for(&doc_name);
            }
            ToolbarAction::None => {}
        }

        let new_document_title = match self.pending_new_document_kind {
            DocumentKind::Map => "New Map",
            DocumentKind::Prefab => "New Prefab",
        };
        let create_button = match self.pending_new_document_kind {
            DocumentKind::Map => "Create",
            DocumentKind::Prefab => "Create Prefab",
        };
        if let Some((width, height)) = self.new_map_size_dialog.show(
            ctx,
            "new_map_size_dialog",
            new_document_title,
            create_button,
            None,
        ) {
            self.create_document_with_dimensions(width, height);
        }

        let prefab_delete_action = self.prefab_delete_dialog.show(ctx);
        self.resolve_prefab_delete_action(prefab_delete_action);

        // Export dialog
        {
            let doc = &self.documents[self.active_tab];
            let export_action = self
                .export_dialog
                .show(ctx, &doc.map, self.wall_atlas.as_ref());
            match export_action {
                ExportDialogAction::Export {
                    path,
                    zoom,
                    bg_color,
                    tab_map,
                } => {
                    let filename = path
                        .file_name()
                        .and_then(|f| f.to_str())
                        .unwrap_or("file")
                        .to_owned();
                    self.status_message = format!("Exporting {}...", filename);

                    let mut ok = false;
                    if let (Some(ta), Some(wa)) = (&self.tile_atlas, &self.wall_atlas) {
                        if let Err(e) =
                            crate::export::export_map_png(&path, &doc.map, ta, wa, zoom, bg_color)
                        {
                            warn!("Export failed: {}", e);
                            self.status_message = format!("Export failed: {}", e);
                        } else {
                            info!("Exported to {}", path.display());
                            ok = true;
                        }
                    } else {
                        warn!("Cannot export: atlases not loaded");
                        self.status_message = "Export failed: atlases not loaded".into();
                    }
                    // Export tab map alongside if enabled
                    if let Some(tm) = tab_map {
                        if let Some(sotp) = &self.sotp_data {
                            let tm_path = crate::export::tab_map_path(&path);
                            if let Err(e) = crate::export::export_tab_map_png(
                                &tm_path,
                                &doc.map,
                                sotp,
                                tm.zoom,
                                tm.bg_color,
                            ) {
                                warn!("Tab map export failed: {}", e);
                                self.status_message = format!("Tab map export failed: {}", e);
                                ok = false;
                            } else {
                                info!("Exported tab map to {}", tm_path.display());
                            }
                        } else {
                            warn!("Cannot export tab map: SOTP data not loaded");
                        }
                    }
                    if ok {
                        self.status_message = format!("Exported {}.", filename);
                    }
                }
                ExportDialogAction::None => {}
            }
        }

        // Inspector: tile palette + tab map
        let (requested_layer, inspector_response) = {
            let doc = &self.documents[self.active_tab];
            let mut requested = self.active_paint_layer;
            let response = InspectorPanel::show(
                ctx,
                &doc.map,
                self.sotp_data.as_deref(),
                self.tile_atlas.as_ref(),
                self.atlas_texture.as_ref(),
                self.wall_atlas.as_ref(),
                self.wall_texture.as_ref(),
                &mut requested,
                &mut self.selected_ground_tile,
                &mut self.selected_wall_tile,
                &mut self.reveal_ground_tile_in_palette,
                &mut self.reveal_wall_tile_in_palette,
                &self.prefab_assets,
                self.selected_prefab,
                &mut self.prefab_search,
                self.active_tool,
            );
            (requested, response)
        };
        if requested_layer != self.active_paint_layer {
            self.set_active_paint_layer(requested_layer);
        }
        self.handle_inspector_response(inspector_response);

        // Viewport needs mutable access to camera for panning
        if self.sotp_data.is_none() {
            self.show_collision_overlay = false;
        }

        let stamp_prefab = self.selected_prefab_map().cloned();
        let vp_result = {
            let doc = &mut self.documents[self.active_tab];
            ViewportPanel::show(
                ctx,
                &doc.map,
                &mut doc.camera,
                effective_tool,
                self.active_shape_kind,
                self.active_paint_layer,
                self.selected_ground_tile,
                self.selected_wall_tile,
                self.line_tool_start_tile,
                self.shape_tool_start_tile,
                stamp_prefab.as_ref(),
                self.tile_atlas.as_ref(),
                self.atlas_texture.as_ref(),
                self.wall_atlas.as_ref(),
                self.wall_texture.as_ref(),
                self.tab_overlay_texture.as_ref(),
                &mut self.layer_visibility,
                &mut self.show_grid,
                &mut self.show_collision_overlay,
                self.sotp_data.as_deref(),
            )
        };
        if let Some(tile) = vp_result.hover_tile {
            self.hover_tile = tile;
        }
        if let Some(tile) = vp_result.clicked_tile {
            self.selected_tile = Some(tile);
        }
        if let Some(pick) = vp_result.eyedropper_pick {
            match pick {
                EyedropperPick::Ground(tile_id) => {
                    self.selected_ground_tile = tile_id;
                    if tile_id != 0 {
                        self.reveal_ground_tile_in_palette = Some(tile_id);
                    }
                    self.set_active_paint_layer(PaintLayer::Ground);
                    self.status_message = format!("Picked ground tile #{}.", tile_id);
                }
                EyedropperPick::LeftWall(tile_id) => {
                    self.selected_wall_tile = tile_id;
                    if tile_id != 0 {
                        self.reveal_wall_tile_in_palette = Some(tile_id);
                    }
                    self.set_active_paint_layer(PaintLayer::LeftWall);
                    self.status_message =
                        format!("Picked wall #{} (left).", self.selected_wall_tile);
                }
                EyedropperPick::RightWall(tile_id) => {
                    self.selected_wall_tile = tile_id;
                    if tile_id != 0 {
                        self.reveal_wall_tile_in_palette = Some(tile_id);
                    }
                    self.set_active_paint_layer(PaintLayer::RightWall);
                    self.status_message =
                        format!("Picked wall #{} (right).", self.selected_wall_tile);
                }
            }
        }

        let paint_layer = self.active_paint_layer;
        let selected_paint_tile = self.selected_tile_for_layer(paint_layer);

        let cancel_shape_or_line = ctx.input(|i| {
            i.key_pressed(egui::Key::Escape)
                || i.pointer.button_pressed(egui::PointerButton::Secondary)
        });
        if cancel_shape_or_line {
            self.line_tool_start_tile = None;
            self.shape_tool_start_tile = None;
        }

        let doc = &mut self.documents[self.active_tab];
        if self.active_tool == Tool::Pencil {
            if let Some(end) = vp_result.pencil_shift_clicked_tile {
                doc.finish_stroke();
                if let Some(start) = self.last_pencil_click_tile {
                    Self::paint_layer_line(doc, paint_layer, selected_paint_tile, start, end);
                    self.last_pencil_click_tile = Some(end);
                }
            } else if let Some(point) = vp_result.pencil_clicked_tile {
                self.last_pencil_click_tile = Some(point);
            }
        } else if self.active_tool == Tool::Line {
            if let Some(end) = vp_result.line_clicked_tile {
                if let Some(start) = self.line_tool_start_tile {
                    doc.finish_stroke();
                    Self::paint_layer_line(doc, paint_layer, selected_paint_tile, start, end);
                    self.line_tool_start_tile = Some(end);
                } else {
                    self.line_tool_start_tile = Some(end);
                }
            }
        } else if self.active_tool == Tool::Shape {
            if let Some(end) = vp_result.shape_clicked_tile {
                if let Some(start) = self.shape_tool_start_tile {
                    doc.finish_stroke();
                    Self::paint_layer_shape(
                        doc,
                        paint_layer,
                        selected_paint_tile,
                        self.active_shape_kind,
                        start,
                        end,
                    );
                    self.shape_tool_start_tile = None;
                } else {
                    self.shape_tool_start_tile = Some(end);
                }
            }
        } else if self.active_tool == Tool::Fill {
            if let Some((col, row, paint_value)) = vp_result.fill_clicked_tile {
                Self::paint_layer_fill(doc, paint_layer, paint_value, (col, row));
            }
        } else if self.active_tool == Tool::Stamp {
            if let Some(origin) = vp_result.stamp_clicked_tile {
                if let Some(prefab) = stamp_prefab.as_ref() {
                    let result = doc.apply_prefab_stamp(origin, prefab);
                    if result.changed_tiles > 0 {
                        self.status_message = if result.clipped_tiles > 0 {
                            format!(
                                "Placed prefab at {}, {} ({} clipped tiles).",
                                origin.0, origin.1, result.clipped_tiles
                            )
                        } else {
                            format!("Placed prefab at {}, {}.", origin.0, origin.1)
                        };
                    }
                } else {
                    self.status_message = "Select a prefab before placing.".to_string();
                }
            }
        }
        if let Some((col, row, paint_value)) = vp_result.painted_tile {
            doc.paint_layer_stroke_tile(paint_layer, col, row, paint_value);
        }
        let primary_down = ctx.input(|i| i.pointer.button_down(egui::PointerButton::Primary));
        if !primary_down {
            doc.finish_stroke();
        }

        let unsaved_changes_action = self.unsaved_changes_dialog.show(ctx);
        self.resolve_unsaved_changes_action(ctx, unsaved_changes_action);
    }
}
