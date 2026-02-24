use std::path::PathBuf;

use eframe::egui;
use tracing::{info, warn};

use crate::document::{LayerVisibility, MapDocument};
use crate::panels::{
    ExportDialog, ExportDialogAction, InspectorPanel, StatusBarAction, StatusBarPanel,
    TabBarAction, TabBarPanel, TitleBarPanel, Tool, ToolbarAction, ToolbarPanel, ViewportPanel,
};
use crate::theme;

pub struct EditorApp {
    documents: Vec<MapDocument>,
    active_tab: usize,
    active_tool: Tool,
    tab_bar: TabBarPanel,
    layer_visibility: LayerVisibility,
    show_grid: bool,
    tile_atlas: Option<render::TileAtlas>,
    atlas_texture: Option<egui::TextureHandle>,
    wall_atlas: Option<render::SpriteAtlas>,
    wall_texture: Option<egui::TextureHandle>,
    sotp_data: Option<Vec<u8>>,
    hover_tile: (u16, u16),
    selected_tile: Option<(u16, u16)>,
    export_dialog: ExportDialog,
    atlas_needs_upload: bool,
    wall_atlas_needs_upload: bool,
}

impl EditorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply_theme(&cc.egui_ctx);

        // Build atlases CPU-side; texture uploads are deferred to first frame
        // because the renderer hasn't reported the real GPU max texture size yet.
        let (tile_atlas, wall_atlas, sotp_data) = Self::load_assets();

        Self {
            documents: vec![MapDocument::new(50, 50)],
            active_tab: 0,
            active_tool: Tool::Select,
            tab_bar: TabBarPanel::default(),
            layer_visibility: LayerVisibility::default(),
            show_grid: true,
            atlas_needs_upload: tile_atlas.is_some(),
            wall_atlas_needs_upload: wall_atlas.is_some(),
            tile_atlas,
            wall_atlas,
            sotp_data,
            atlas_texture: None,
            wall_texture: None,
            hover_tile: (0, 0),
            selected_tile: None,
            export_dialog: ExportDialog::default(),
        }
    }

    /// Load all assets from the archive: tile atlas, wall sprite atlas, and SOTP collision data.
    fn load_assets() -> (Option<render::TileAtlas>, Option<render::SpriteAtlas>, Option<Vec<u8>>) {
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

        let pal_data = match pool.get("legend.pal") {
            Some(data) => data,
            None => {
                warn!("legend.pal not found in asset archives");
                return (None, None, None);
            }
        };
        let palette = match render::Palette::from_bytes(pal_data) {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to parse palette: {}", e);
                return (None, None, None);
            }
        };

        // Ground tile atlas
        let tile_atlas = match pool.get("TILEA.BMP") {
            Some(tile_data) => match render::TileAtlas::from_raw(tile_data, &palette, 56, 27) {
                Ok(atlas) => {
                    let (w, h) = atlas.dimensions();
                    info!(
                        "Built tile atlas: {}x{} ({} tiles)",
                        w,
                        h,
                        atlas.tile_count()
                    );
                    Some(atlas)
                }
                Err(e) => {
                    warn!("Failed to build tile atlas: {}", e);
                    None
                }
            },
            None => {
                warn!("TILEA.BMP not found in asset archives");
                None
            }
        };

        // Wall sprite atlas from HPF files
        let wall_atlas = Self::load_wall_atlas(&pool, &palette);

        // SOTP collision data
        let sotp_data = pool
            .get("SOTP.DAT")
            .or_else(|| pool.get("sotp.dat"))
            .map(|data| {
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
    fn load_wall_atlas(
        pool: &archive::AssetPool,
        palette: &render::Palette,
    ) -> Option<render::SpriteAtlas> {
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
                Some(bytes) => {
                    match render::HpfSprite::decode(bytes) {
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
                    }
                }
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

        match render::SpriteAtlas::build(&sprites, palette, 28) {
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

    fn new_document(&mut self) {
        self.documents.push(MapDocument::new(50, 50));
        self.active_tab = self.documents.len() - 1;
    }

    fn open_document(&mut self) {
        let file = rfd::FileDialog::new()
            .add_filter("Map", &["map"])
            .pick_file();

        if let Some(path) = file {
            // Check if already open — just switch to it
            for (i, doc) in self.documents.iter().enumerate() {
                if doc.path.as_ref() == Some(&path) {
                    self.active_tab = i;
                    return;
                }
            }

            match MapDocument::open(path.clone()) {
                Ok(doc) => {
                    self.documents.push(doc);
                    self.active_tab = self.documents.len() - 1;
                    info!("Opened map: {}", path.display());
                }
                Err(e) => {
                    warn!("Failed to open {}: {}", path.display(), e);
                }
            }
        }
    }

    fn save_document(&mut self) {
        if self.documents[self.active_tab].path.is_some() {
            if let Err(e) = self.documents[self.active_tab].save() {
                warn!("Failed to save: {}", e);
            }
        } else {
            self.save_document_as();
        }
    }

    fn save_document_as(&mut self) {
        let file = rfd::FileDialog::new()
            .add_filter("Map", &["map"])
            .save_file();

        if let Some(path) = file {
            match self.documents[self.active_tab].save_as(path.clone()) {
                Ok(()) => info!("Saved map: {}", path.display()),
                Err(e) => warn!("Failed to save as {}: {}", path.display(), e),
            }
        }
    }

    fn close_tab(&mut self, index: usize) {
        if self.documents.len() <= 1 {
            // Don't close the last tab — replace with a fresh document
            self.documents[0] = MapDocument::new(50, 50);
            self.active_tab = 0;
            return;
        }

        self.documents.remove(index);
        if self.active_tab > index {
            self.active_tab -= 1;
        } else if self.active_tab >= self.documents.len() {
            self.active_tab = self.documents.len() - 1;
        }
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let (new, open, save, save_as, close, export, tool, toggle_layer, keyboard_zoom, reset_zoom) =
            ctx.input(|i| {
                let cmd = i.modifiers.command;
                let shift = i.modifiers.shift;

                // Tool shortcuts (no modifiers) — only Select is enabled for now
                let tool = if !cmd && !shift {
                    if i.key_pressed(egui::Key::V) {
                        Some(Tool::Select)
                    } else {
                        None
                    }
                } else {
                    None
                };

                // Layer/grid toggle shortcuts (Cmd+1/2/3/4)
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

                // Keyboard zoom (Cmd+/-, snap to 25%; Cmd+0 reset)
                let keyboard_zoom: Option<i8> = if cmd && !shift {
                    if i.key_pressed(egui::Key::Minus) {
                        Some(-1)
                    } else if i.key_pressed(egui::Key::Plus)
                        || i.key_pressed(egui::Key::Equals)
                    {
                        Some(1)
                    } else {
                        None
                    }
                } else {
                    None
                };
                let reset_zoom = cmd && !shift && i.key_pressed(egui::Key::Num0);

                (
                    cmd && i.key_pressed(egui::Key::N),
                    cmd && i.key_pressed(egui::Key::O),
                    false, // Save disabled for now
                    false, // Save As disabled for now
                    cmd && i.key_pressed(egui::Key::W),
                    cmd && i.key_pressed(egui::Key::E),
                    tool,
                    toggle_layer,
                    keyboard_zoom,
                    reset_zoom,
                )
            });

        if let Some(t) = tool {
            self.active_tool = t;
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
            self.open_document();
        }
        if close {
            self.close_tab(self.active_tab);
        }
        if save_as {
            self.save_document_as();
        } else if save {
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
                if !ext.eq_ignore_ascii_case("map") {
                    continue;
                }
                // Check if already open — just switch to it
                let already_open = self.documents.iter().position(|doc| {
                    doc.path.as_ref() == Some(&path)
                });
                if let Some(i) = already_open {
                    self.active_tab = i;
                    continue;
                }
                match MapDocument::open(path.clone()) {
                    Ok(doc) => {
                        self.documents.push(doc);
                        self.active_tab = self.documents.len() - 1;
                        info!("Opened dropped map: {}", path.display());
                    }
                    Err(e) => {
                        warn!("Failed to open dropped file {}: {}", path.display(), e);
                    }
                }
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

        self.handle_keyboard_shortcuts(ctx);
        self.handle_dropped_files(ctx);

        TitleBarPanel::show(ctx, frame);

        let tab_action = self.tab_bar.show(ctx, &self.documents, self.active_tab);
        match tab_action {
            TabBarAction::CloseTab(i) => self.close_tab(i),
            TabBarAction::SwitchTab(i) => {
                self.active_tab = i;
                self.tab_bar.ensure_tab_visible(&self.documents, i);
            }
            TabBarAction::None => {}
        }

        let doc = &self.documents[self.active_tab];
        let current_zoom = doc.camera.zoom;
        let status_action =
            StatusBarPanel::show(ctx, &doc.map, self.active_tool, self.hover_tile, current_zoom);
        match status_action {
            StatusBarAction::ZoomIn => {
                Self::snap_zoom(&mut self.documents[self.active_tab].camera, 1);
            }
            StatusBarAction::ZoomOut => {
                Self::snap_zoom(&mut self.documents[self.active_tab].camera, -1);
            }
            StatusBarAction::SetDimensions(w, h) => {
                self.documents[self.active_tab].set_dimensions(w, h);
            }
            StatusBarAction::None => {}
        }

        let toolbar_action = ToolbarPanel::show(ctx, &mut self.active_tool);
        match toolbar_action {
            ToolbarAction::NewFile => self.new_document(),
            ToolbarAction::OpenFile => self.open_document(),
            ToolbarAction::Export => {
                let doc_name = self.documents[self.active_tab].display_name();
                self.export_dialog.open_for(&doc_name);
            }
            ToolbarAction::None => {}
        }

        // Export dialog
        {
            let doc = &self.documents[self.active_tab];
            let export_action =
                self.export_dialog
                    .show(ctx, &doc.map, self.wall_atlas.as_ref());
            match export_action {
                ExportDialogAction::Export {
                    path,
                    zoom,
                    bg_color,
                    tab_map,
                } => {
                    if let (Some(ta), Some(wa)) = (&self.tile_atlas, &self.wall_atlas) {
                        if let Err(e) =
                            crate::export::export_map_png(&path, &doc.map, ta, wa, zoom, bg_color)
                        {
                            warn!("Export failed: {}", e);
                        } else {
                            info!("Exported to {}", path.display());
                        }
                    } else {
                        warn!("Cannot export: atlases not loaded");
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
                            } else {
                                info!("Exported tab map to {}", tm_path.display());
                            }
                        } else {
                            warn!("Cannot export tab map: SOTP data not loaded");
                        }
                    }
                }
                ExportDialogAction::None => {}
            }
        }

        // Inspector: tileset + tab map
        {
            let doc = &self.documents[self.active_tab];
            InspectorPanel::show(ctx, &doc.map, self.sotp_data.as_deref());
        }

        // Viewport needs mutable access to camera for panning
        let doc = &mut self.documents[self.active_tab];
        let vp_result = ViewportPanel::show(
            ctx,
            &doc.map,
            &mut doc.camera,
            self.tile_atlas.as_ref(),
            self.atlas_texture.as_ref(),
            self.wall_atlas.as_ref(),
            self.wall_texture.as_ref(),
            &mut self.layer_visibility,
            &mut self.show_grid,
        );
        if let Some(tile) = vp_result.hover_tile {
            self.hover_tile = tile;
        }
        if let Some(tile) = vp_result.clicked_tile {
            self.selected_tile = Some(tile);
        }
    }
}
