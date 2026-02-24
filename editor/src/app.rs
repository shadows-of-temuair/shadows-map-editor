use std::path::PathBuf;

use eframe::egui;
use tracing::{info, warn};

use crate::document::{LayerVisibility, MapDocument};
use crate::panels::{
    InspectorPanel, StatusBarAction, StatusBarPanel, TabBarAction, TabBarPanel, TitleBarPanel,
    Tool, ToolbarAction, ToolbarPanel, ViewportPanel,
};
use crate::theme;

pub struct EditorApp {
    documents: Vec<MapDocument>,
    active_tab: usize,
    active_tool: Tool,
    inspector: InspectorPanel,
    layer_visibility: LayerVisibility,
    show_grid: bool,
    tile_atlas: Option<render::TileAtlas>,
    atlas_texture: Option<egui::TextureHandle>,
    hover_tile: (u16, u16),
    atlas_needs_upload: bool,
}

impl EditorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply_theme(&cc.egui_ctx);

        // Build atlas CPU-side; texture upload is deferred to first frame
        // because the renderer hasn't reported the real GPU max texture size yet.
        let tile_atlas = Self::load_atlas_data();

        Self {
            documents: vec![MapDocument::new(50, 50)],
            active_tab: 0,
            active_tool: Tool::Pencil,
            inspector: InspectorPanel::default(),
            layer_visibility: LayerVisibility::default(),
            show_grid: true,
            atlas_needs_upload: tile_atlas.is_some(),
            tile_atlas,
            atlas_texture: None,
            hover_tile: (0, 0),
        }
    }

    /// Build the tile atlas from archive assets (CPU-side only, no GPU upload).
    fn load_atlas_data() -> Option<render::TileAtlas> {
        let assets_dir = PathBuf::from("assets");

        let pool = match archive::AssetPool::load(&assets_dir) {
            Ok(pool) => pool,
            Err(e) => {
                warn!(
                    "Could not load asset archives from {}: {}",
                    assets_dir.display(),
                    e
                );
                return None;
            }
        };

        let pal_data = match pool.get("legend.pal") {
            Some(data) => data,
            None => {
                warn!("legend.pal not found in asset archives");
                return None;
            }
        };
        let palette = match render::Palette::from_bytes(pal_data) {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to parse palette: {}", e);
                return None;
            }
        };

        let tile_data = match pool.get("TILEA.BMP") {
            Some(data) => data,
            None => {
                warn!("TILEA.BMP not found in asset archives");
                return None;
            }
        };

        match render::TileAtlas::from_raw(tile_data, &palette, 56, 27) {
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
        }
    }

    /// Upload the atlas texture to the GPU on the first frame, when the renderer
    /// has reported the actual max texture size.
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
        let (new, open, save, save_as, close, tool, toggle_layer) = ctx.input(|i| {
            let cmd = i.modifiers.command;
            let shift = i.modifiers.shift;

            // Tool shortcuts (no modifiers)
            let tool = if !cmd && !shift {
                if i.key_pressed(egui::Key::V) {
                    Some(Tool::Select)
                } else if i.key_pressed(egui::Key::B) {
                    Some(Tool::Pencil)
                } else if i.key_pressed(egui::Key::E) {
                    Some(Tool::Eraser)
                } else if i.key_pressed(egui::Key::G) {
                    Some(Tool::Fill)
                } else if i.key_pressed(egui::Key::I) {
                    Some(Tool::Eyedropper)
                } else if i.key_pressed(egui::Key::R) {
                    Some(Tool::Rectangle)
                } else {
                    None
                }
            } else {
                None
            };

            // Layer toggle shortcuts (Cmd+1/2/3)
            let toggle_layer = if cmd && !shift {
                if i.key_pressed(egui::Key::Num1) {
                    Some(1)
                } else if i.key_pressed(egui::Key::Num2) {
                    Some(2)
                } else if i.key_pressed(egui::Key::Num3) {
                    Some(3)
                } else {
                    None
                }
            } else {
                None
            };

            (
                cmd && i.key_pressed(egui::Key::N),
                cmd && i.key_pressed(egui::Key::O),
                cmd && !shift && i.key_pressed(egui::Key::S),
                cmd && shift && i.key_pressed(egui::Key::S),
                cmd && i.key_pressed(egui::Key::W),
                tool,
                toggle_layer,
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
                _ => {}
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
    }
}

impl eframe::App for EditorApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.043, 0.047, 0.055, 1.0] // matches bg (#0b0c0e)
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Deferred atlas upload — renderer has set the real GPU max texture size by now
        self.try_upload_atlas(ctx);

        self.handle_keyboard_shortcuts(ctx);

        TitleBarPanel::show(ctx, frame);

        let tab_action = TabBarPanel::show(ctx, &self.documents, self.active_tab);
        match tab_action {
            TabBarAction::NewTab => self.new_document(),
            TabBarAction::CloseTab(i) => self.close_tab(i),
            TabBarAction::SwitchTab(i) => self.active_tab = i,
            TabBarAction::None => {}
        }

        let doc = &self.documents[self.active_tab];
        let current_zoom = doc.camera.zoom;
        let status_action =
            StatusBarPanel::show(ctx, &doc.map, self.active_tool, self.hover_tile, current_zoom);
        match status_action {
            StatusBarAction::ZoomIn => {
                let new_zoom = (current_zoom + 0.20).min(4.0);
                let doc = &mut self.documents[self.active_tab];
                doc.camera.offset *= new_zoom / current_zoom;
                doc.camera.zoom = new_zoom;
            }
            StatusBarAction::ZoomOut => {
                let new_zoom = (current_zoom - 0.20).max(0.25);
                let doc = &mut self.documents[self.active_tab];
                doc.camera.offset *= new_zoom / current_zoom;
                doc.camera.zoom = new_zoom;
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
            ToolbarAction::SaveFile => self.save_document(),
            ToolbarAction::None => {}
        }
        self.inspector.show(ctx);

        // Viewport needs mutable access to camera for panning
        let doc = &mut self.documents[self.active_tab];
        let hover = ViewportPanel::show(
            ctx,
            &doc.map,
            &mut doc.camera,
            self.tile_atlas.as_ref(),
            self.atlas_texture.as_ref(),
            &mut self.layer_visibility,
            &mut self.show_grid,
        );
        if let Some(tile) = hover {
            self.hover_tile = tile;
        }
    }
}
