use std::{
    collections::VecDeque,
    fs,
    path::Path,
    path::PathBuf,
    sync::mpsc::{self, Receiver},
};

use eframe::egui;
use tracing::{info, warn};

mod selection;

use self::selection::{SelectionClipboard, SelectionDragMode};
use crate::document::{DocumentKind, LayerVisibility, MapDocument, PaintLayer, TileSelection};
use crate::map_list::{MapList, MapMetadataHint};
use crate::palette_lookup::LoadedPaletteLookup;
use crate::panels::{
    AssetSetupDialog, AssetSetupDialogAction, ExportDialog, ExportDialogAction, EyedropperPick,
    InspectorPanel, InspectorResponse, MapSizeDialog, PrefabCreateDialog, PrefabCreateDialogAction,
    PrefabDeleteDialog, PrefabDeleteDialogAction, SelectionMovePreview, StatusBarAction,
    StatusBarPanel, TabBarAction, TabBarPanel, TitleBarPanel, Tool, ToolbarAction, ToolbarPanel,
    UnsavedChangesDialog, UnsavedChangesDialogAction, ViewportPanel, WindowFrame,
};
use crate::prefab::{self, PrefabAsset};
use crate::shape::{self, ShapeKind};
use crate::theme;

const SELECTION_CLIPBOARD_SENTINEL: &str = "__shadows_map_editor_selection__";

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

struct PendingPrefabRename {
    path: PathBuf,
}

struct PendingPrefabCreation {
    map: map::Map,
}

struct LoadedAssets {
    tile_atlas: Option<render::TileAtlas>,
    wall_atlas: Option<render::SpriteAtlas>,
    sotp_data: Option<Vec<u8>>,
}

enum AssetImportProgressEvent {
    Progress {
        copied: usize,
        total: usize,
        file_name: String,
    },
    Finished {
        copied: usize,
    },
    Failed {
        error: String,
    },
}

struct AssetImportTask {
    receiver: Receiver<AssetImportProgressEvent>,
    copied: usize,
    total: usize,
    current_file: Option<String>,
    just_started: bool,
}

enum AssetLoadProgressEvent {
    Status {
        message: String,
        completed_steps: usize,
        total_steps: usize,
    },
    Finished(LoadedAssets),
    Failed(String),
}

struct AssetLoadTask {
    receiver: Receiver<AssetLoadProgressEvent>,
    completed_steps: usize,
    total_steps: usize,
}

enum AtlasBuildResult {
    Tile(Option<render::TileAtlas>),
    Wall(Option<render::SpriteAtlas>),
}

const ASSET_LOAD_STAGE_COUNT: usize = 5;
const ASSET_SETUP_PROMPT_STATUS: &str = "Assets not found, asking user to setup";

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
    selected_ground_tile: u16,
    selected_wall_tile: u16,
    reveal_ground_tile_in_palette: Option<u16>,
    reveal_wall_tile_in_palette: Option<u16>,
    selection_clipboard: Option<SelectionClipboard>,
    paste_preview_active: bool,
    selection_drag_start_tile: Option<(u16, u16)>,
    selection_drag_mode: Option<SelectionDragMode>,
    last_pencil_click_tile: Option<(u16, u16)>,
    line_tool_start_tile: Option<(u16, u16)>,
    shape_tool_start_tile: Option<(u16, u16)>,
    export_dialog: ExportDialog,
    new_map_size_dialog: MapSizeDialog,
    pending_new_document_kind: DocumentKind,
    asset_setup_dialog: AssetSetupDialog,
    asset_import_task: Option<AssetImportTask>,
    asset_load_task: Option<AssetLoadTask>,
    unsaved_changes_dialog: UnsavedChangesDialog,
    pending_unsaved_changes: Option<PendingUnsavedChanges>,
    prefab_create_dialog: PrefabCreateDialog,
    pending_prefab_creation: Option<PendingPrefabCreation>,
    prefab_delete_dialog: PrefabDeleteDialog,
    pending_prefab_deletion: Option<PendingPrefabDeletion>,
    pending_prefab_rename: Option<PendingPrefabRename>,
    prefab_rename_buffer: String,
    prefab_rename_should_focus: bool,
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

        let needs_asset_setup = Self::assets_dir_missing_or_empty();
        let map_list = MapList::load_if_exists("maps.ron");
        let prefab_assets = prefab::load_prefab_assets().unwrap_or_default();
        let selected_prefab = (!prefab_assets.is_empty()).then_some(0);
        let mut app = Self {
            documents: vec![MapDocument::new(50, 50)],
            active_tab: 0,
            active_tool: Tool::Select,
            active_shape_kind: ShapeKind::Rect,
            active_paint_layer: PaintLayer::Ground,
            last_wall_paint_layer: PaintLayer::LeftWall,
            tab_bar: TabBarPanel::default(),
            status_bar: StatusBarPanel::default(),
            layer_visibility: LayerVisibility::default(),
            show_grid: true,
            show_collision_overlay: false,
            atlas_needs_upload: false,
            wall_atlas_needs_upload: false,
            tile_atlas: None,
            wall_atlas: None,
            sotp_data: None,
            map_list,
            prefab_assets,
            selected_prefab,
            prefab_search: String::new(),
            atlas_texture: None,
            wall_texture: None,
            tab_overlay_texture: None,
            hover_tile: (0, 0),
            selected_ground_tile: 0,
            selected_wall_tile: 0,
            reveal_ground_tile_in_palette: None,
            reveal_wall_tile_in_palette: None,
            selection_clipboard: None,
            paste_preview_active: false,
            selection_drag_start_tile: None,
            selection_drag_mode: None,
            last_pencil_click_tile: None,
            line_tool_start_tile: None,
            shape_tool_start_tile: None,
            export_dialog: ExportDialog::default(),
            new_map_size_dialog: MapSizeDialog::default(),
            pending_new_document_kind: DocumentKind::Map,
            asset_setup_dialog: {
                let mut dialog = AssetSetupDialog::default();
                if needs_asset_setup {
                    dialog.open();
                }
                dialog
            },
            asset_import_task: None,
            asset_load_task: None,
            unsaved_changes_dialog: UnsavedChangesDialog::default(),
            pending_unsaved_changes: None,
            prefab_create_dialog: PrefabCreateDialog::default(),
            pending_prefab_creation: None,
            prefab_delete_dialog: PrefabDeleteDialog::default(),
            pending_prefab_deletion: None,
            pending_prefab_rename: None,
            prefab_rename_buffer: String::new(),
            prefab_rename_should_focus: false,
            allow_window_close: false,
            status_message: if needs_asset_setup {
                String::from(ASSET_SETUP_PROMPT_STATUS)
            } else {
                format!("Loading Dark Ages assets... 0/{ASSET_LOAD_STAGE_COUNT}")
            },
            tab_overlay_texture_needs_upload: true,
        };

        if !needs_asset_setup {
            app.start_asset_loading();
        }

        app
    }

    fn clear_edit_anchors(&mut self) {
        self.paste_preview_active = false;
        self.selection_drag_start_tile = None;
        self.selection_drag_mode = None;
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

    fn renaming_prefab_index(&self) -> Option<usize> {
        let pending = self.pending_prefab_rename.as_ref()?;
        self.prefab_assets
            .iter()
            .position(|prefab| Self::paths_match(&prefab.path, &pending.path))
    }

    fn set_active_tool(&mut self, tool: Tool) {
        if self.active_tool != tool {
            self.active_tool = tool;
            self.selection_drag_start_tile = None;
            self.selection_drag_mode = None;
            if tool != Tool::Select {
                self.paste_preview_active = false;
            }
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

    fn enter_prefab_edit_mode(&mut self) {
        self.set_active_tool(Tool::Pencil);
        self.set_active_paint_layer(self.last_wall_paint_layer);
    }

    fn selected_tile_for_layer(&self, layer: PaintLayer) -> u16 {
        match layer {
            PaintLayer::Ground => self.selected_ground_tile,
            PaintLayer::LeftWall | PaintLayer::RightWall => self.selected_wall_tile,
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

    fn is_dat_file(path: &Path) -> bool {
        path.is_file()
            && path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("dat"))
                .unwrap_or(false)
    }

    fn assets_dir_missing_or_empty() -> bool {
        let assets_dir = PathBuf::from("assets");
        let Ok(entries) = fs::read_dir(assets_dir) else {
            return true;
        };

        !entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .any(|path| Self::is_dat_file(&path))
    }

    /// Load all assets from the archive: tile atlas, wall sprite atlas, and SOTP collision data.
    fn load_assets_with_progress<F>(mut progress: F) -> Result<LoadedAssets, String>
    where
        F: FnMut(String, usize, usize),
    {
        let assets_dir = PathBuf::from("assets");
        progress(
            "Loading assets: archives (1/5)".to_string(),
            0,
            ASSET_LOAD_STAGE_COUNT,
        );

        let pool = match archive::AssetPool::load(&assets_dir) {
            Ok(pool) => pool,
            Err(e) => {
                return Err(format!(
                    "Could not load asset archives from {}: {}",
                    assets_dir.display(),
                    e
                ));
            }
        };

        progress(
            "Loading assets: palettes (2/5)".to_string(),
            1,
            ASSET_LOAD_STAGE_COUNT,
        );
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

        progress(
            "Loading assets: tile and wall atlases (3/5)".to_string(),
            2,
            ASSET_LOAD_STAGE_COUNT,
        );
        let (tile_atlas, wall_atlas) = std::thread::scope(|scope| {
            let pool = &pool;
            let legacy_palette = legacy_palette.as_ref();
            let ground_palette_lookup = ground_palette_lookup.as_ref();
            let wall_palette_lookup = wall_palette_lookup.as_ref();
            let (sender, receiver) = mpsc::channel();

            let tile_sender = sender.clone();
            scope.spawn(move || {
                let atlas = Self::build_tile_atlas(pool, legacy_palette, ground_palette_lookup);
                let _ = tile_sender.send(AtlasBuildResult::Tile(atlas));
            });

            scope.spawn(move || {
                let atlas =
                    Self::build_wall_sprite_atlas(pool, legacy_palette, wall_palette_lookup);
                let _ = sender.send(AtlasBuildResult::Wall(atlas));
            });

            let mut tile_atlas = None;
            let mut wall_atlas = None;
            let mut tile_finished = false;
            let mut wall_finished = false;

            for _ in 0..2 {
                match receiver.recv() {
                    Ok(AtlasBuildResult::Tile(atlas)) => {
                        tile_atlas = atlas;
                        tile_finished = true;
                        if !wall_finished {
                            progress(
                                "Loading assets: wall atlas (4/5)".to_string(),
                                3,
                                ASSET_LOAD_STAGE_COUNT,
                            );
                        }
                    }
                    Ok(AtlasBuildResult::Wall(atlas)) => {
                        wall_atlas = atlas;
                        wall_finished = true;
                        if !tile_finished {
                            progress(
                                "Loading assets: tile atlas (4/5)".to_string(),
                                3,
                                ASSET_LOAD_STAGE_COUNT,
                            );
                        }
                    }
                    Err(_) => break,
                }
            }

            (tile_atlas, wall_atlas)
        });

        // SOTP collision data
        progress(
            "Loading assets: collision data (5/5)".to_string(),
            4,
            ASSET_LOAD_STAGE_COUNT,
        );
        let sotp_data = Self::get_pool_asset_case_insensitive(&pool, "SOTP.DAT").map(|data| {
            info!("Loaded SOTP.DAT ({} bytes)", data.len());
            data.to_vec()
        });
        if sotp_data.is_none() {
            warn!("SOTP.DAT not found in asset archives");
        }

        Ok(LoadedAssets {
            tile_atlas,
            wall_atlas,
            sotp_data,
        })
    }

    fn build_tile_atlas(
        pool: &archive::AssetPool,
        legacy_palette: Option<&render::Palette>,
        ground_palette_lookup: Option<&LoadedPaletteLookup>,
    ) -> Option<render::TileAtlas> {
        match Self::get_pool_asset_case_insensitive(pool, "TILEA.BMP") {
            Some(tile_data) => {
                let atlas_result = if let Some(lookup) = ground_palette_lookup {
                    match legacy_palette.or_else(|| lookup.fallback_palette()) {
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
                } else if let Some(palette) = legacy_palette {
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
                    Some(Err(error)) => {
                        warn!("Failed to build tile atlas: {}", error);
                        None
                    }
                    None => None,
                }
            }
            None => {
                warn!("TILEA.BMP not found in asset archives");
                None
            }
        }
    }

    fn build_wall_sprite_atlas(
        pool: &archive::AssetPool,
        legacy_palette: Option<&render::Palette>,
        wall_palette_lookup: Option<&LoadedPaletteLookup>,
    ) -> Option<render::SpriteAtlas> {
        if let Some(lookup) = wall_palette_lookup {
            match legacy_palette.or_else(|| lookup.fallback_palette()) {
                Some(default_palette) => {
                    info!("Rendering STC wall sprites using stc palette-table mode");
                    Self::load_wall_atlas(pool, |wall_id| {
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
        } else if let Some(palette) = legacy_palette {
            info!("Rendering STC wall sprites using legacy legend palette");
            Self::load_wall_atlas(pool, |_| palette)
        } else {
            warn!("No usable palette found for STC wall sprite rendering");
            None
        }
    }

    fn default_wall_tile_selection(wall_atlas: Option<&render::SpriteAtlas>) -> u16 {
        wall_atlas
            .and_then(|atlas| {
                (1..atlas.sprite_count())
                    .find(|&id| atlas.sprite_rect(id).is_some())
                    .map(|id| id.min(u16::MAX as u32) as u16)
            })
            .unwrap_or(0)
    }

    fn apply_loaded_assets(&mut self, assets: LoadedAssets) {
        self.atlas_needs_upload = assets.tile_atlas.is_some();
        self.wall_atlas_needs_upload = assets.wall_atlas.is_some();
        self.tile_atlas = assets.tile_atlas;
        self.wall_atlas = assets.wall_atlas;
        self.sotp_data = assets.sotp_data;
        self.atlas_texture = None;
        self.wall_texture = None;

        if self.selected_ground_tile == 0 && self.tile_atlas.is_some() {
            self.selected_ground_tile = 1;
        }
        if self.selected_wall_tile == 0 {
            self.selected_wall_tile = Self::default_wall_tile_selection(self.wall_atlas.as_ref());
        }
        if self.sotp_data.is_none() {
            self.show_collision_overlay = false;
        }
    }

    fn start_asset_loading(&mut self) {
        if self.asset_load_task.is_some() {
            return;
        }

        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let send_progress = |message: String, completed_steps: usize, total_steps: usize| {
                let _ = sender.send(AssetLoadProgressEvent::Status {
                    message,
                    completed_steps,
                    total_steps,
                });
            };

            match Self::load_assets_with_progress(send_progress) {
                Ok(assets) => {
                    let _ = sender.send(AssetLoadProgressEvent::Finished(assets));
                }
                Err(error) => {
                    let _ = sender.send(AssetLoadProgressEvent::Failed(error));
                }
            }
        });

        self.asset_load_task = Some(AssetLoadTask {
            receiver,
            completed_steps: 0,
            total_steps: ASSET_LOAD_STAGE_COUNT,
        });
    }

    fn open_asset_setup_prompt(&mut self) {
        self.asset_setup_dialog.open();
        self.status_message = String::from(ASSET_SETUP_PROMPT_STATUS);
    }

    fn poll_asset_load_progress(&mut self, ctx: &egui::Context) {
        let mut completion = None;

        if let Some(task) = self.asset_load_task.as_mut() {
            while let Ok(event) = task.receiver.try_recv() {
                match event {
                    AssetLoadProgressEvent::Status {
                        message,
                        completed_steps,
                        total_steps,
                    } => {
                        task.completed_steps = completed_steps;
                        task.total_steps = total_steps;
                        self.status_message = message;
                    }
                    AssetLoadProgressEvent::Finished(assets) => {
                        completion = Some(Ok(assets));
                    }
                    AssetLoadProgressEvent::Failed(error) => {
                        completion = Some(Err(error));
                    }
                }
            }
            ctx.request_repaint();
        }

        let Some(result) = completion else {
            return;
        };

        self.asset_load_task = None;
        match result {
            Ok(assets) => {
                let has_any_assets = assets.tile_atlas.is_some()
                    || assets.wall_atlas.is_some()
                    || assets.sotp_data.is_some();
                self.apply_loaded_assets(assets);
                if has_any_assets {
                    self.status_message = String::from("Assets loaded.");
                } else {
                    self.open_asset_setup_prompt();
                }
            }
            Err(error) => {
                warn!("Asset load failed: {}", error);
                self.open_asset_setup_prompt();
            }
        }
    }

    fn start_dark_ages_asset_import(&mut self, source_dir: &Path) -> Result<(), String> {
        let entries = fs::read_dir(source_dir).map_err(|error| {
            format!(
                "Could not read Dark Ages folder {}: {}",
                source_dir.display(),
                error
            )
        })?;

        let assets_dir = PathBuf::from("assets");
        fs::create_dir_all(&assets_dir)
            .map_err(|error| format!("Could not create {}: {}", assets_dir.display(), error))?;

        let mut jobs = Vec::new();
        for entry in entries.filter_map(Result::ok) {
            let source_path = entry.path();
            if !Self::is_dat_file(&source_path) {
                continue;
            }

            let Some(file_name) = source_path.file_name().map(|name| name.to_os_string()) else {
                continue;
            };
            jobs.push((source_path, assets_dir.join(file_name)));
        }

        if jobs.is_empty() {
            return Err(format!(
                "No .dat files were found in {}. Select the Dark Ages install folder that contains the game archives.",
                source_dir.display()
            ));
        }

        let total = jobs.len();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let mut copied = 0usize;
            for (source_path, destination) in jobs {
                let file_name = source_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("archive.dat")
                    .to_string();

                let copy_result = if Self::paths_match(&source_path, &destination) {
                    Ok(0)
                } else {
                    fs::copy(&source_path, &destination)
                };

                if let Err(error) = copy_result {
                    let _ = sender.send(AssetImportProgressEvent::Failed {
                        error: format!(
                            "Failed to copy {} into {}: {}",
                            source_path.display(),
                            destination.display(),
                            error
                        ),
                    });
                    return;
                }

                copied += 1;
                let _ = sender.send(AssetImportProgressEvent::Progress {
                    copied,
                    total,
                    file_name,
                });
            }

            let _ = sender.send(AssetImportProgressEvent::Finished { copied });
        });

        self.asset_import_task = Some(AssetImportTask {
            receiver,
            copied: 0,
            total,
            current_file: None,
            just_started: true,
        });
        self.status_message = String::from("Importing Dark Ages assets...");
        Ok(())
    }

    fn poll_asset_import_progress(&mut self, ctx: &egui::Context) {
        let mut completion = None;

        if let Some(task) = self.asset_import_task.as_mut() {
            if task.just_started {
                task.just_started = false;
                ctx.request_repaint();
                return;
            }

            while let Ok(event) = task.receiver.try_recv() {
                match event {
                    AssetImportProgressEvent::Progress {
                        copied,
                        total,
                        file_name,
                    } => {
                        task.copied = copied;
                        task.total = total;
                        task.current_file = Some(file_name);
                        self.status_message =
                            format!("Importing Dark Ages assets... {copied}/{total}");
                    }
                    AssetImportProgressEvent::Finished { copied } => {
                        completion = Some(Ok(copied));
                    }
                    AssetImportProgressEvent::Failed { error } => {
                        completion = Some(Err(error));
                    }
                }
            }
            ctx.request_repaint();
        }

        let Some(result) = completion else {
            return;
        };

        self.asset_import_task = None;
        match result {
            Ok(copied) => {
                self.status_message = format!(
                    "Imported {} Dark Ages .dat files. Loading assets...",
                    copied
                );
                self.start_asset_loading();
            }
            Err(error) => {
                warn!("Asset import failed: {}", error);
                self.open_asset_setup_prompt();
            }
        }
    }

    fn resolve_asset_setup_action(&mut self, action: AssetSetupDialogAction) {
        match action {
            AssetSetupDialogAction::None => {}
            AssetSetupDialogAction::NotNow => {
                self.status_message = String::from(
                    "Dark Ages assets are missing. Copy the game's .dat files into assets/ to render tiles, walls, and collision data.",
                );
            }
            AssetSetupDialogAction::SelectFolder => {
                let folder = rfd::FileDialog::new()
                    .set_title("Select Dark Ages Folder")
                    .pick_folder();

                let Some(folder) = folder else {
                    self.open_asset_setup_prompt();
                    return;
                };

                self.asset_setup_dialog.close();
                match self.start_dark_ages_asset_import(&folder) {
                    Ok(()) => {}
                    Err(error) => {
                        warn!("Asset import failed: {}", error);
                        self.open_asset_setup_prompt();
                    }
                }
            }
        }
    }

    fn status_progress(&self) -> Option<f32> {
        if let Some(task) = self.asset_import_task.as_ref() {
            let total = task.total.max(1);
            return Some((task.copied as f32 / total as f32).clamp(0.0, 1.0));
        }

        self.asset_load_task.as_ref().map(|task| {
            let total = task.total_steps.max(1);
            (task.completed_steps as f32 / total as f32).clamp(0.0, 1.0)
        })
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
                    self.enter_prefab_edit_mode();
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
                    self.enter_prefab_edit_mode();
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
            .duplicate_prefab_index
            .filter(|index| *index < self.prefab_assets.len())
        {
            match self.duplicate_prefab(index) {
                Ok(path) => {
                    let duplicated_to = path
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .unwrap_or("prefab");
                    self.status_message = format!("Duplicated prefab to {}.", duplicated_to);
                }
                Err(error) => {
                    self.status_message = error;
                }
            }
        }
        if let Some(index) = response
            .start_rename_prefab_index
            .filter(|index| *index < self.prefab_assets.len())
        {
            self.begin_prefab_rename_flow(index);
        }
        if response.submit_prefab_rename {
            self.commit_prefab_rename();
        }
        if response.cancel_prefab_rename {
            self.cancel_prefab_rename();
        }
        if let Some(index) = response
            .delete_prefab_index
            .filter(|index| *index < self.prefab_assets.len())
        {
            self.begin_prefab_delete_flow(index);
        }
    }

    fn begin_prefab_create_flow(&mut self) -> bool {
        let Some(selection) = self.effective_selection_for_any_occupied_tile() else {
            return false;
        };

        self.pending_prefab_creation = Some(PendingPrefabCreation {
            map: self.documents[self.active_tab].selection_map(selection),
        });
        self.prefab_create_dialog.open();
        true
    }

    fn resolve_prefab_create_action(&mut self, action: PrefabCreateDialogAction) {
        match action {
            PrefabCreateDialogAction::None => {}
            PrefabCreateDialogAction::Cancel => {
                self.prefab_create_dialog.close();
                self.pending_prefab_creation = None;
            }
            PrefabCreateDialogAction::Create {
                name,
                include_ground,
            } => match self.create_prefab_from_pending_selection(&name, include_ground) {
                Ok(path) => {
                    let created_name = path
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .unwrap_or("prefab");
                    self.prefab_create_dialog.close();
                    self.pending_prefab_creation = None;
                    self.status_message = format!("Created prefab {}.", created_name);
                }
                Err(error) => {
                    self.prefab_create_dialog
                        .restore_after_error(name, include_ground, error);
                }
            },
        }
    }

    fn create_prefab_from_pending_selection(
        &mut self,
        requested_name: &str,
        include_ground: bool,
    ) -> Result<PathBuf, String> {
        let Some(pending) = self.pending_prefab_creation.as_ref() else {
            return Err("No selection is available to create a prefab.".to_string());
        };

        let sanitized_name = prefab::sanitize_prefab_name(requested_name);
        if sanitized_name.is_empty() {
            return Err("Enter a prefab name.".to_string());
        }
        if self.prefab_assets.iter().any(|prefab| {
            prefab
                .file_stem_name()
                .eq_ignore_ascii_case(&sanitized_name)
        }) {
            return Err(format!("A prefab named {} already exists.", sanitized_name));
        }

        let trimmed = prefab::trimmed_map(&pending.map, include_ground).ok_or_else(|| {
            if include_ground {
                "The selection does not contain any tiles to save.".to_string()
            } else {
                "The selection does not contain any wall tiles unless Include ground is enabled."
                    .to_string()
            }
        })?;

        let prefabs_dir = prefab::ensure_prefabs_dir()
            .map_err(|error| format!("Could not access prefabs/: {}", error))?;
        let destination = prefabs_dir.join(format!("{sanitized_name}.ron"));
        let prefab_file = prefab::PrefabFile::from_map(&trimmed, Some(sanitized_name));
        prefab_file.save(&destination).map_err(|error| {
            format!(
                "Could not create prefab {}: {}",
                destination.display(),
                error
            )
        })?;

        self.reload_prefabs();
        self.select_prefab_by_path(&destination);
        self.open_document_from_path(destination.clone());
        Ok(destination)
    }

    fn begin_prefab_rename_flow(&mut self, index: usize) {
        let Some(prefab) = self.prefab_assets.get(index) else {
            return;
        };

        self.pending_prefab_rename = Some(PendingPrefabRename {
            path: prefab.path.clone(),
        });
        self.prefab_rename_buffer = prefab.file_stem_name().to_string();
        self.prefab_rename_should_focus = true;
    }

    fn cancel_prefab_rename(&mut self) {
        self.pending_prefab_rename = None;
        self.prefab_rename_buffer.clear();
        self.prefab_rename_should_focus = false;
    }

    fn commit_prefab_rename(&mut self) {
        let Some(pending) = self.pending_prefab_rename.as_ref() else {
            return;
        };

        let source_path = pending.path.clone();
        let requested_name = self.prefab_rename_buffer.clone();
        let sanitized_name = prefab::sanitize_prefab_name(&requested_name);
        if Self::prefab_name_conflicts(&self.prefab_assets, &source_path, &sanitized_name) {
            self.status_message = format!("A prefab named {} already exists.", sanitized_name);
            self.cancel_prefab_rename();
            return;
        }

        match self.rename_prefab(&source_path, &requested_name) {
            Ok(path) => {
                let renamed_to = path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("prefab");
                self.status_message = format!("Renamed prefab to {}.", renamed_to);
                self.cancel_prefab_rename();
            }
            Err(error) => {
                self.status_message = error;
                self.prefab_rename_should_focus = true;
            }
        }
    }

    fn begin_prefab_delete_flow(&mut self, index: usize) {
        let Some(prefab) = self.prefab_assets.get(index) else {
            return;
        };
        let prefab_path = prefab.path.clone();
        let name = prefab.file_stem_name().to_string();

        if self
            .pending_prefab_rename
            .as_ref()
            .map(|pending| Self::paths_match(&pending.path, &prefab_path))
            .unwrap_or(false)
        {
            self.cancel_prefab_rename();
        }

        self.prefab_delete_dialog.open_for(&name);
        self.pending_prefab_deletion = Some(PendingPrefabDeletion {
            path: prefab_path,
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

    fn duplicate_prefab(&mut self, index: usize) -> Result<PathBuf, String> {
        let Some(prefab) = self.prefab_assets.get(index) else {
            return Err("Could not find prefab to duplicate.".to_string());
        };

        for doc in &self.documents {
            let Some(doc_path) = doc.path.as_ref() else {
                continue;
            };
            if Self::paths_match(doc_path, &prefab.path) && doc.dirty {
                return Err(format!(
                    "Save open prefab tabs for {} before duplicating it.",
                    prefab.file_stem_name()
                ));
            }
        }

        let duplicate_name =
            Self::next_duplicate_prefab_name(&self.prefab_assets, prefab.file_stem_name());
        let prefabs_dir = prefab::ensure_prefabs_dir()
            .map_err(|error| format!("Could not access prefabs/: {}", error))?;
        let destination = prefabs_dir.join(format!("{duplicate_name}.ron"));

        let mut prefab_file = prefab::PrefabFile::load(&prefab.path)
            .map_err(|error| format!("Could not duplicate {}: {}", prefab.path.display(), error))?;
        prefab_file.name = Some(duplicate_name.clone());
        prefab_file.save(&destination).map_err(|error| {
            format!(
                "Could not write duplicate {}: {}",
                destination.display(),
                error
            )
        })?;

        self.reload_prefabs();
        self.select_prefab_by_path(&destination);
        Ok(destination)
    }

    fn rename_prefab(&mut self, path: &Path, requested_name: &str) -> Result<PathBuf, String> {
        let sanitized_name = prefab::sanitize_prefab_name(requested_name);
        if sanitized_name.is_empty() {
            return Err("Prefab name must contain at least one letter or number.".to_string());
        }
        if Self::prefab_name_conflicts(&self.prefab_assets, path, &sanitized_name) {
            return Err(format!("A prefab named {} already exists.", sanitized_name));
        }

        let prefabs_dir = prefab::ensure_prefabs_dir()
            .map_err(|error| format!("Could not access prefabs/: {}", error))?;
        let destination = prefabs_dir.join(format!("{sanitized_name}.ron"));
        if !Self::paths_match(path, &destination) {
            fs::rename(path, &destination).map_err(|error| {
                format!(
                    "Rename failed for {} -> {}: {}",
                    path.display(),
                    destination.display(),
                    error
                )
            })?;
        }

        self.update_open_prefab_paths(path, &destination);
        self.reload_prefabs();
        self.select_prefab_by_path(&destination);
        Ok(destination)
    }

    fn update_open_prefab_paths(&mut self, old_path: &Path, new_path: &Path) {
        for doc in &mut self.documents {
            let Some(doc_path) = doc.path.as_ref() else {
                continue;
            };
            if !Self::paths_match(doc_path, old_path) {
                continue;
            }
            doc.update_prefab_path(new_path.to_path_buf());
        }
    }

    fn prefab_name_conflicts(
        prefabs: &[PrefabAsset],
        current_path: &Path,
        candidate_name: &str,
    ) -> bool {
        prefabs.iter().any(|prefab| {
            !Self::paths_match(&prefab.path, current_path)
                && prefab.file_stem_name().eq_ignore_ascii_case(candidate_name)
        })
    }

    fn next_duplicate_prefab_name(prefabs: &[PrefabAsset], base_name: &str) -> String {
        let mut suffix = 1usize;
        loop {
            let candidate = format!("{base_name}_{suffix}");
            if !prefabs
                .iter()
                .any(|prefab| prefab.file_stem_name().eq_ignore_ascii_case(&candidate))
            {
                return candidate;
            }
            suffix += 1;
        }
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

        let points = shape::paint_points(shape_kind, start, end);
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
            cut,
            copy,
            paste,
            export,
            tool,
            toggle_ground_wall_layer,
            toggle_wall_target_side,
            toggle_layer,
            toggle_tab_overlay,
            keyboard_zoom,
            reset_zoom,
            delete_selection,
            clear_selection,
        ) = ctx.input(|i| {
            let cmd = i.modifiers.command;
            let shift = i.modifiers.shift;
            let cut_event = i.events.iter().any(|event| matches!(event, egui::Event::Cut));
            let copy_event = i.events.iter().any(|event| matches!(event, egui::Event::Copy));
            let paste_event = i
                .events
                .iter()
                .any(|event| matches!(event, egui::Event::Paste(_)));

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
            let toggle_tab_overlay = !cmd && !shift && i.key_pressed(egui::Key::Tab);

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
            let delete_selection = !keyboard_captured
                && !cmd
                && (i.key_pressed(egui::Key::Backspace) || i.key_pressed(egui::Key::Delete));
            let clear_selection = !keyboard_captured && i.key_pressed(egui::Key::Escape);
            let save = cmd && !shift && i.key_pressed(egui::Key::S);
            let save_as = cmd && shift && i.key_pressed(egui::Key::S);
            let undo = cmd && !shift && i.key_pressed(egui::Key::Z);
            let redo = cmd && shift && i.key_pressed(egui::Key::Z);
            let cut = !keyboard_captured && (cut_event || (cmd && !shift && i.key_pressed(egui::Key::X)));
            let copy =
                !keyboard_captured && (copy_event || (cmd && !shift && i.key_pressed(egui::Key::C)));
            let paste =
                !keyboard_captured && (paste_event || (cmd && !shift && i.key_pressed(egui::Key::V)));

            (
                cmd && !shift && i.key_pressed(egui::Key::N),
                cmd && !shift && i.key_pressed(egui::Key::O),
                save,
                save_as,
                cmd && i.key_pressed(egui::Key::W),
                undo,
                redo,
                cut,
                copy,
                paste,
                cmd && i.key_pressed(egui::Key::E),
                tool,
                toggle_ground_wall_layer,
                toggle_wall_target_side,
                toggle_layer,
                toggle_tab_overlay,
                keyboard_zoom,
                reset_zoom,
                delete_selection,
                clear_selection,
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
        let tab_shortcut_blocked = self.asset_setup_dialog.is_open()
            || self.export_dialog.is_open()
            || self.new_map_size_dialog.is_open()
            || self.prefab_create_dialog.is_open()
            || self.prefab_delete_dialog.is_open()
            || self.unsaved_changes_dialog.is_open()
            || self.status_bar.is_size_dialog_open()
            || self.renaming_prefab_index().is_some();
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
        let shift_held = ctx.input(|i| i.modifiers.shift);
        let action_layers = self.selection_action_layers(shift_held);
        if cut {
            if self.cut_active_selection_to_clipboard(action_layers) {
                ctx.copy_text(SELECTION_CLIPBOARD_SENTINEL.to_string());
            }
        }
        if copy {
            if self.copy_active_selection_to_clipboard(action_layers) {
                ctx.copy_text(SELECTION_CLIPBOARD_SENTINEL.to_string());
            }
        }
        if paste && self.selection_clipboard.is_some() {
            self.paste_preview_active = true;
            self.status_message = "Paste preview active. Click to place.".to_string();
        }
        if delete_selection && self.active_tool == Tool::Select {
            let _ = self.clear_active_selection_layers(action_layers);
        }
        if clear_selection {
            let cleared_selection = self.clear_active_selection();
            if self.paste_preview_active {
                self.paste_preview_active = false;
                self.status_message = if cleared_selection {
                    "Paste canceled and selection cleared.".to_string()
                } else {
                    "Paste canceled.".to_string()
                };
            }
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
        self.poll_asset_import_progress(ctx);
        self.poll_asset_load_progress(ctx);

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

        if !self.unsaved_changes_dialog.is_open()
            && !self.prefab_delete_dialog.is_open()
            && !self.asset_setup_dialog.is_open()
            && self.asset_import_task.is_none()
        {
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
        let selection_dimensions = doc.selection().map(|selection| selection.dimensions());
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
            doc.kind(),
            &current_file_label,
            effective_tool,
            self.hover_tile,
            selection_dimensions,
            current_zoom,
            &self.status_message,
            self.status_progress(),
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
                let resize_result = match kind {
                    DocumentKind::Map => self.documents[self.active_tab].set_dimensions(w, h),
                    DocumentKind::Prefab => {
                        self.documents[self.active_tab].resize_canvas_centered(w, h)
                    }
                };
                if let Err(err) = resize_result {
                    self.status_message = err;
                } else {
                    self.status_message = match kind {
                        DocumentKind::Map => format!("Resized map to {}x{}", w, h),
                        DocumentKind::Prefab => {
                            format!("Resized prefab canvas to {}x{}", w, h)
                        }
                    };
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
            crate::panels::MapSizeDialogMode::Standard,
        ) {
            self.create_document_with_dimensions(width, height);
        }

        let asset_setup_action = self.asset_setup_dialog.show(ctx);
        self.resolve_asset_setup_action(asset_setup_action);

        let prefab_create_action = self.prefab_create_dialog.show(ctx);
        self.resolve_prefab_create_action(prefab_create_action);

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
            let renaming_prefab_index = self.renaming_prefab_index();
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
                renaming_prefab_index,
                &mut self.prefab_rename_buffer,
                &mut self.prefab_rename_should_focus,
                self.active_tool,
            );
            (requested, response)
        };
        if requested_layer != self.active_paint_layer {
            self.set_active_paint_layer(requested_layer);
        }
        let inspector_panel_resize_active = inspector_response.panel_resize_active;
        self.handle_inspector_response(inspector_response);

        // Viewport needs mutable access to camera for panning
        if self.sotp_data.is_none() {
            self.show_collision_overlay = false;
        }

        let stamp_prefab = self.selected_prefab_map().cloned();
        let shift_held = ctx.input(|i| i.modifiers.shift);
        let selection_action_layers = self.selection_action_layers(shift_held);
        let selection_move_layers = self.selection_move_layers();
        let selection_duplicate_layers = self.selection_duplicate_layers();
        let duplicate_drag_active = shift_held
            && matches!(
                self.selection_drag_mode,
                Some(SelectionDragMode::Moving { .. })
            );
        let selection_move_active = matches!(
            self.selection_drag_mode,
            Some(SelectionDragMode::Moving { .. })
        );
        let selection_move_preview = match (
            self.selection_drag_mode.as_ref(),
            self.documents[self.active_tab].selection(),
        ) {
            (
                Some(SelectionDragMode::Moving {
                    original_selection,
                    preview_map,
                    ..
                }),
                Some(selection),
            ) => {
                let (min_col, min_row, _, _) = selection.normalized_bounds();
                Some(SelectionMovePreview {
                    map: preview_map,
                    top_left: (min_col, min_row),
                    layers: if duplicate_drag_active {
                        selection_duplicate_layers
                    } else {
                        selection_move_layers
                    },
                    ignore_overwrite_region: Some(*original_selection),
                })
            }
            _ => None,
        };
        let paste_preview = if self.paste_preview_active {
            self.selection_clipboard
                .as_ref()
                .map(|clipboard| SelectionMovePreview {
                    map: &clipboard.map,
                    top_left: (0, 0),
                    layers: clipboard.layers,
                    ignore_overwrite_region: None,
                })
        } else {
            None
        };
        let modal_open = self.new_map_size_dialog.is_open()
            || self.asset_setup_dialog.is_open()
            || self.prefab_create_dialog.is_open()
            || self.prefab_delete_dialog.is_open()
            || self.export_dialog.is_open()
            || self.unsaved_changes_dialog.is_open();
        let vp_result = {
            let doc = &mut self.documents[self.active_tab];
            let current_selection = doc.selection();
            ViewportPanel::show(
                ctx,
                &doc.map,
                &mut doc.camera,
                !modal_open && !inspector_panel_resize_active,
                effective_tool,
                self.active_shape_kind,
                self.active_paint_layer,
                self.selected_ground_tile,
                self.selected_wall_tile,
                self.line_tool_start_tile,
                self.shape_tool_start_tile,
                current_selection,
                selection_move_active,
                self.selection_drag_start_tile,
                selection_move_preview,
                paste_preview,
                self.paste_preview_active.then_some(self.hover_tile),
                self.selection_clipboard.is_some(),
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
        if let Some(tile) = vp_result.selection_drag_started {
            let current_selection = self.documents[self.active_tab].selection();
            self.selection_drag_start_tile = Some(tile);
            if let Some(selection) = current_selection.filter(|selection| selection.contains(tile))
            {
                let (min_col, min_row, _, _) = selection.normalized_bounds();
                self.selection_drag_mode = Some(SelectionDragMode::Moving {
                    original_selection: selection,
                    grab_offset: (tile.0 - min_col, tile.1 - min_row),
                    preview_map: self.documents[self.active_tab].selection_map(selection),
                    created_from_empty: false,
                });
            } else {
                let single_tile_selection = TileSelection::from_points(tile, tile);
                let convenience_drag_layers = if shift_held {
                    selection_duplicate_layers
                } else {
                    selection_move_layers
                };
                let start_single_tile_drag = current_selection.is_none()
                    && Self::tile_has_selection_layers(
                        &self.documents[self.active_tab].map,
                        tile,
                        convenience_drag_layers,
                    );
                if start_single_tile_drag {
                    self.documents[self.active_tab].set_selection(Some(single_tile_selection));
                    self.selection_drag_mode = Some(SelectionDragMode::Moving {
                        original_selection: single_tile_selection,
                        grab_offset: (0, 0),
                        preview_map: self.documents[self.active_tab]
                            .selection_map(single_tile_selection),
                        created_from_empty: true,
                    });
                } else {
                    self.selection_drag_mode = Some(SelectionDragMode::Selecting);
                    self.documents[self.active_tab].set_selection(None);
                }
            }
        }
        if let Some(tile) = vp_result.selection_drag_tile {
            if let (Some(start), Some(mode)) = (
                self.selection_drag_start_tile,
                self.selection_drag_mode.as_ref(),
            ) {
                match mode {
                    SelectionDragMode::Selecting => {
                        if tile != start {
                            self.documents[self.active_tab]
                                .set_selection(Some(TileSelection::from_points(start, tile)));
                        } else {
                            self.documents[self.active_tab].set_selection(None);
                        }
                    }
                    SelectionDragMode::Moving {
                        original_selection,
                        grab_offset,
                        ..
                    } => {
                        let top_left = {
                            let map = &self.documents[self.active_tab].map;
                            Self::moved_selection_top_left(
                                map,
                                *original_selection,
                                *grab_offset,
                                tile,
                            )
                        };
                        let (width, height) = original_selection.dimensions();
                        self.documents[self.active_tab].set_selection(Some(
                            TileSelection::from_top_left_size(top_left, width, height),
                        ));
                    }
                }
            }
        }
        if vp_result.clear_selection_requested {
            self.documents[self.active_tab].set_selection(None);
        }
        if vp_result.cut_selection_requested {
            if self.cut_active_selection_to_clipboard(selection_action_layers) {
                ctx.copy_text(SELECTION_CLIPBOARD_SENTINEL.to_string());
            }
        }
        if vp_result.copy_selection_requested {
            if self.copy_active_selection_to_clipboard(selection_action_layers) {
                ctx.copy_text(SELECTION_CLIPBOARD_SENTINEL.to_string());
            }
        }
        if vp_result.activate_paste_preview && self.selection_clipboard.is_some() {
            self.paste_preview_active = true;
            self.status_message = "Paste preview active. Click to place.".to_string();
        }
        if vp_result.cancel_paste_preview {
            self.paste_preview_active = false;
            self.status_message = "Paste canceled.".to_string();
        }
        if let Some(tile) = vp_result.paste_preview_clicked_tile {
            let keep_preview_active = ctx.input(|i| i.modifiers.shift);
            let _ = self.paste_selection_clipboard_at(tile, keep_preview_active);
        }
        if vp_result.create_prefab_requested {
            let _ = self.create_prefab_from_active_selection();
        }
        if vp_result.delete_selection_requested {
            let _ = self.clear_active_selection_layers(selection_action_layers);
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
            if alt_held && self.active_tool == Tool::Select {
                self.set_active_tool(Tool::Pencil);
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
        if self.active_tool == Tool::Select && !primary_down {
            if let Some(SelectionDragMode::Moving {
                original_selection,
                created_from_empty,
                ..
            }) = self.selection_drag_mode.as_ref()
            {
                let original_selection = *original_selection;
                let created_from_empty = *created_from_empty;
                let preview_selection = doc.selection().unwrap_or(original_selection);
                let (preview_min_col, preview_min_row, _, _) =
                    preview_selection.normalized_bounds();
                let (original_min_col, original_min_row, _, _) =
                    original_selection.normalized_bounds();
                if preview_min_col != original_min_col || preview_min_row != original_min_row {
                    if shift_held {
                        let source = doc.selection_map_for_visible_layers(
                            original_selection,
                            selection_duplicate_layers,
                        );
                        let (width, height) = original_selection.dimensions();
                        let changed = doc.paste_visible_layers(
                            (preview_min_col, preview_min_row),
                            &source,
                            selection_duplicate_layers,
                        );
                        doc.set_selection(Some(TileSelection::from_top_left_size(
                            (preview_min_col, preview_min_row),
                            width,
                            height,
                        )));
                        if changed > 0 {
                            self.status_message = format!(
                                "Duplicated selection to {}, {}.",
                                preview_min_col, preview_min_row
                            );
                        }
                    } else {
                        let changed = doc.move_selection_visible_layers(
                            original_selection,
                            (preview_min_col, preview_min_row),
                            selection_move_layers,
                        );
                        if changed > 0 {
                            self.status_message = format!(
                                "Moved selection to {}, {}.",
                                preview_min_col, preview_min_row
                            );
                        }
                    }
                } else {
                    if created_from_empty {
                        doc.set_selection(None);
                    } else {
                        doc.set_selection(Some(original_selection));
                    }
                }
            }
            self.selection_drag_start_tile = None;
            self.selection_drag_mode = None;
        } else if self.active_tool != Tool::Select {
            self.selection_drag_start_tile = None;
            self.selection_drag_mode = None;
        }
        if !primary_down {
            doc.finish_stroke();
        }

        let unsaved_changes_action = self.unsaved_changes_dialog.show(ctx);
        self.resolve_unsaved_changes_action(ctx, unsaved_changes_action);
    }
}
