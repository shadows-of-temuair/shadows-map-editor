use std::path::PathBuf;

use eframe::egui;
use tracing::warn;

use crate::{
    map_list::MapMetadataHint,
    prefab::{
        PrefabFile, centered_canvas_offset, load_prefab_asset, placement_anchor,
        sanitize_prefab_name, trimmed_map,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaintLayer {
    Ground,
    LeftWall,
    RightWall,
}

impl PaintLayer {
    fn read_from_tile(self, tile: &map::Tile) -> u16 {
        match self {
            PaintLayer::Ground => tile.ground,
            PaintLayer::LeftWall => tile.left_wall,
            PaintLayer::RightWall => tile.right_wall,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentKind {
    Map,
    Prefab,
}

impl DocumentKind {
    pub fn noun(self) -> &'static str {
        match self {
            DocumentKind::Map => "map",
            DocumentKind::Prefab => "prefab",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileSelection {
    pub start_col: u16,
    pub start_row: u16,
    pub end_col: u16,
    pub end_row: u16,
}

impl TileSelection {
    pub fn from_points(start: (u16, u16), end: (u16, u16)) -> Self {
        Self {
            start_col: start.0,
            start_row: start.1,
            end_col: end.0,
            end_row: end.1,
        }
    }

    pub fn normalized_bounds(self) -> (u16, u16, u16, u16) {
        (
            self.start_col.min(self.end_col),
            self.start_row.min(self.end_row),
            self.start_col.max(self.end_col),
            self.start_row.max(self.end_row),
        )
    }

    pub fn dimensions(self) -> (u16, u16) {
        let (min_col, min_row, max_col, max_row) = self.normalized_bounds();
        (max_col - min_col + 1, max_row - min_row + 1)
    }

    pub fn contains(self, tile: (u16, u16)) -> bool {
        let (min_col, min_row, max_col, max_row) = self.normalized_bounds();
        tile.0 >= min_col && tile.0 <= max_col && tile.1 >= min_row && tile.1 <= max_row
    }

    pub fn from_top_left_size(top_left: (u16, u16), width: u16, height: u16) -> Self {
        Self {
            start_col: top_left.0,
            start_row: top_left.1,
            end_col: top_left.0 + width.saturating_sub(1),
            end_row: top_left.1 + height.saturating_sub(1),
        }
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
enum EditChange {
    Ground {
        idx: usize,
        old: u16,
        new: u16,
    },
    LeftWall {
        idx: usize,
        old: u16,
        new: u16,
    },
    RightWall {
        idx: usize,
        old: u16,
        new: u16,
    },
    Tile {
        idx: usize,
        old: map::Tile,
        new: map::Tile,
    },
}

impl EditChange {
    fn apply_undo(&self, map: &mut map::Map) {
        let Some(tile) = map.tiles.get_mut(self.idx()) else {
            return;
        };

        match self {
            EditChange::Ground { old, .. } => tile.ground = *old,
            EditChange::LeftWall { old, .. } => tile.left_wall = *old,
            EditChange::RightWall { old, .. } => tile.right_wall = *old,
            EditChange::Tile { old, .. } => *tile = *old,
        }
    }

    fn apply_redo(&self, map: &mut map::Map) {
        let Some(tile) = map.tiles.get_mut(self.idx()) else {
            return;
        };

        match self {
            EditChange::Ground { new, .. } => tile.ground = *new,
            EditChange::LeftWall { new, .. } => tile.left_wall = *new,
            EditChange::RightWall { new, .. } => tile.right_wall = *new,
            EditChange::Tile { new, .. } => *tile = *new,
        }
    }

    fn idx(&self) -> usize {
        match self {
            EditChange::Ground { idx, .. }
            | EditChange::LeftWall { idx, .. }
            | EditChange::RightWall { idx, .. }
            | EditChange::Tile { idx, .. } => *idx,
        }
    }
}

struct PendingStroke {
    layer: PaintLayer,
    paint_value: u16,
    original_values: std::collections::BTreeMap<usize, u16>,
}

pub struct Camera {
    pub offset: egui::Vec2,
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            offset: egui::Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayerVisibility {
    pub ground: bool,
    pub left_wall: bool,
    pub right_wall: bool,
}

impl Default for LayerVisibility {
    fn default() -> Self {
        Self {
            ground: true,
            left_wall: true,
            right_wall: true,
        }
    }
}

impl LayerVisibility {
    pub fn any(self) -> bool {
        self.ground || self.left_wall || self.right_wall
    }

    pub fn all(self) -> bool {
        self.ground && self.left_wall && self.right_wall
    }
}

pub struct StampResult {
    pub changed_tiles: usize,
    pub clipped_tiles: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrimCanvasResult {
    Empty,
    Unchanged,
    Trimmed { width: u16, height: u16 },
}

pub struct MapDocument {
    kind: DocumentKind,
    pub map: map::Map,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub id_hint: Option<u32>,
    pub name_hint: Option<String>,
    pub camera: Camera,
    selection: Option<TileSelection>,
    undo_stack: Vec<Vec<EditChange>>,
    redo_stack: Vec<Vec<EditChange>>,
    pending_stroke: Option<PendingStroke>,
}

impl MapDocument {
    pub fn new(width: u16, height: u16) -> Self {
        Self::new_map(width, height)
    }

    pub fn new_map(width: u16, height: u16) -> Self {
        Self::new_with_kind(DocumentKind::Map, width, height)
    }

    pub fn new_prefab(width: u16, height: u16) -> Self {
        Self::new_with_kind(DocumentKind::Prefab, width, height)
    }

    fn new_with_kind(kind: DocumentKind, width: u16, height: u16) -> Self {
        Self {
            kind,
            map: map::Map::new(width, height),
            path: None,
            dirty: false,
            id_hint: None,
            name_hint: None,
            camera: Camera::default(),
            selection: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            pending_stroke: None,
        }
    }

    pub fn open_map(
        path: PathBuf,
        metadata_hint: Option<MapMetadataHint>,
    ) -> std::io::Result<Self> {
        let mut map = map::Map::load(&path)?;

        let id_hint = metadata_hint.as_ref().map(|hint| hint.map_id);
        let name_hint = metadata_hint.as_ref().map(|hint| hint.map_name.clone());

        if let Some((width, height)) = metadata_hint.and_then(|hint| hint.map_size) {
            let hinted_count = width as usize * height as usize;
            if hinted_count == map.tiles.len() {
                map.width = width;
                map.height = height;
            } else {
                warn!(
                    "Ignoring maps.ron size hint {}x{} for {}: tile count mismatch (hint={}, actual={})",
                    width,
                    height,
                    path.display(),
                    hinted_count,
                    map.tiles.len()
                );
            }
        }

        Ok(Self {
            kind: DocumentKind::Map,
            map,
            path: Some(path),
            dirty: false,
            id_hint,
            name_hint,
            camera: Camera::default(),
            selection: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            pending_stroke: None,
        })
    }

    pub fn open_prefab(path: PathBuf) -> std::io::Result<Self> {
        let asset = load_prefab_asset(&path)?;
        Ok(Self {
            kind: DocumentKind::Prefab,
            map: asset.map,
            path: Some(path),
            dirty: false,
            id_hint: None,
            name_hint: Some(asset.name),
            camera: Camera::default(),
            selection: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            pending_stroke: None,
        })
    }

    pub fn save_as(&mut self, path: PathBuf) -> std::io::Result<()> {
        self.finish_stroke();

        match self.kind {
            DocumentKind::Map => {
                self.map.save(&path)?;
            }
            DocumentKind::Prefab => {
                let file_name = path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(ToOwned::to_owned)
                    .or_else(|| self.name_hint.clone());
                let prefab = PrefabFile::from_map(&self.map, file_name);
                prefab.save(&path)?;
                self.name_hint = path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(|stem| stem.to_string());
            }
        }

        self.path = Some(path);
        self.dirty = false;
        Ok(())
    }

    pub fn kind(&self) -> DocumentKind {
        self.kind
    }

    pub fn set_dimensions(&mut self, width: u16, height: u16) -> Result<(), String> {
        self.finish_stroke();
        if width == 0 || height == 0 {
            return Err("Width and height must both be at least 1.".to_string());
        }

        let old_count = self.map.tiles.len();
        let new_count = width as usize * height as usize;

        let mut new_tiles = Vec::new();
        if let Err(err) = new_tiles.try_reserve_exact(new_count) {
            return Err(format!(
                "Unable to resize {} to {}x{} ({} tiles): {}",
                self.kind.noun(),
                width,
                height,
                new_count,
                err
            ));
        }
        new_tiles.resize(new_count, map::Tile::default());

        let copy_count = old_count.min(new_count);
        new_tiles[..copy_count].copy_from_slice(&self.map.tiles[..copy_count]);

        self.map.width = width;
        self.map.height = height;
        self.map.tiles = new_tiles;
        self.camera = Camera::default();
        self.selection = None;
        self.dirty = true;
        self.undo_stack.clear();
        self.redo_stack.clear();
        Ok(())
    }

    pub fn resize_canvas_centered(&mut self, width: u16, height: u16) -> Result<(), String> {
        self.finish_stroke();
        if width == 0 || height == 0 {
            return Err("Width and height must both be at least 1.".to_string());
        }

        let new_count = width as usize * height as usize;
        let mut new_tiles = Vec::new();
        if let Err(err) = new_tiles.try_reserve_exact(new_count) {
            return Err(format!(
                "Unable to resize {} canvas to {}x{} ({} tiles): {}",
                self.kind.noun(),
                width,
                height,
                new_count,
                err
            ));
        }
        new_tiles.resize(new_count, map::Tile::default());

        let old_width = self.map.width;
        let old_height = self.map.height;
        let old_tiles = self.map.tiles.clone();
        let col_offset = centered_canvas_offset(old_width, width);
        let row_offset = centered_canvas_offset(old_height, height);

        for old_row in 0..old_height {
            for old_col in 0..old_width {
                let new_col = i32::from(old_col) + col_offset;
                let new_row = i32::from(old_row) + row_offset;
                if new_col < 0
                    || new_row < 0
                    || new_col >= i32::from(width)
                    || new_row >= i32::from(height)
                {
                    continue;
                }

                let src_idx = old_row as usize * old_width as usize + old_col as usize;
                let dst_idx = new_row as usize * width as usize + new_col as usize;
                new_tiles[dst_idx] = old_tiles[src_idx];
            }
        }

        self.map.width = width;
        self.map.height = height;
        self.map.tiles = new_tiles;
        self.camera = Camera::default();
        self.selection = None;
        self.dirty = true;
        self.undo_stack.clear();
        self.redo_stack.clear();
        Ok(())
    }

    pub fn trim_canvas_to_content(&mut self) -> Result<TrimCanvasResult, String> {
        self.finish_stroke();

        let Some(trimmed) = trimmed_map(&self.map, true) else {
            return Ok(TrimCanvasResult::Empty);
        };

        if trimmed.width == self.map.width && trimmed.height == self.map.height {
            return Ok(TrimCanvasResult::Unchanged);
        }

        let width = trimmed.width;
        let height = trimmed.height;
        self.map = trimmed;
        self.camera = Camera::default();
        self.selection = None;
        self.dirty = true;
        self.undo_stack.clear();
        self.redo_stack.clear();
        Ok(TrimCanvasResult::Trimmed { width, height })
    }

    pub fn selection(&self) -> Option<TileSelection> {
        self.selection
    }

    pub fn set_selection(&mut self, selection: Option<TileSelection>) {
        self.selection = selection;
    }

    #[cfg(test)]
    pub fn clear_selection_layer(&mut self, selection: TileSelection, layer: PaintLayer) -> usize {
        let _ = self.finish_stroke();

        let (min_col, min_row, max_col, max_row) = selection.normalized_bounds();
        let mut cleared_tiles = 0usize;

        for row in min_row..=max_row {
            for col in min_col..=max_col {
                if self.paint_layer_stroke_tile(layer, col, row, 0) {
                    cleared_tiles += 1;
                }
            }
        }

        let _ = self.finish_stroke();
        cleared_tiles
    }

    pub fn selection_map(&self, selection: TileSelection) -> map::Map {
        let (min_col, min_row, max_col, max_row) = selection.normalized_bounds();
        let width = max_col - min_col + 1;
        let height = max_row - min_row + 1;
        let mut selected = map::Map::new(width, height);

        for row_offset in 0..height {
            for col_offset in 0..width {
                let src_col = min_col + col_offset;
                let src_row = min_row + row_offset;
                let src_idx = src_row as usize * self.map.width as usize + src_col as usize;
                let dst_idx = row_offset as usize * width as usize + col_offset as usize;
                selected.tiles[dst_idx] = self.map.tiles[src_idx];
            }
        }

        selected
    }

    pub fn selection_map_for_visible_layers(
        &self,
        selection: TileSelection,
        visibility: LayerVisibility,
    ) -> map::Map {
        let mut selected = self.selection_map(selection);
        for tile in &mut selected.tiles {
            if !visibility.ground {
                tile.ground = 0;
            }
            if !visibility.left_wall {
                tile.left_wall = 0;
            }
            if !visibility.right_wall {
                tile.right_wall = 0;
            }
        }
        selected
    }

    pub fn clear_selection_visible_layers(
        &mut self,
        selection: TileSelection,
        visibility: LayerVisibility,
    ) -> usize {
        let _ = self.finish_stroke();
        if !visibility.any() {
            return 0;
        }

        let (min_col, min_row, max_col, max_row) = selection.normalized_bounds();
        let mut changes = Vec::new();

        for row in min_row..=max_row {
            for col in min_col..=max_col {
                let idx = row as usize * self.map.width as usize + col as usize;
                let old_tile = self.map.tiles[idx];
                let mut new_tile = old_tile;

                if visibility.ground {
                    new_tile.ground = 0;
                }
                if visibility.left_wall {
                    new_tile.left_wall = 0;
                }
                if visibility.right_wall {
                    new_tile.right_wall = 0;
                }

                if old_tile == new_tile {
                    continue;
                }

                self.map.tiles[idx] = new_tile;
                changes.push(EditChange::Tile {
                    idx,
                    old: old_tile,
                    new: new_tile,
                });
            }
        }

        let changed_tiles = changes.len();
        let _ = self.push_history(changes);
        changed_tiles
    }

    #[cfg(test)]
    pub fn paste_map_region(&mut self, top_left: (u16, u16), source: &map::Map) -> usize {
        let _ = self.finish_stroke();

        let mut changes = Vec::new();

        for src_row in 0..source.height {
            for src_col in 0..source.width {
                let dst_col = u32::from(top_left.0) + u32::from(src_col);
                let dst_row = u32::from(top_left.1) + u32::from(src_row);
                if dst_col >= u32::from(self.map.width) || dst_row >= u32::from(self.map.height) {
                    continue;
                }

                let src_idx = src_row as usize * source.width as usize + src_col as usize;
                let dst_idx = dst_row as usize * self.map.width as usize + dst_col as usize;
                let old_tile = self.map.tiles[dst_idx];
                let new_tile = source.tiles[src_idx];
                if old_tile == new_tile {
                    continue;
                }

                self.map.tiles[dst_idx] = new_tile;
                changes.push(EditChange::Tile {
                    idx: dst_idx,
                    old: old_tile,
                    new: new_tile,
                });
            }
        }

        let changed_tiles = changes.len();
        let _ = self.push_history(changes);
        changed_tiles
    }

    pub fn paste_visible_layers(
        &mut self,
        top_left: (u16, u16),
        source: &map::Map,
        visibility: LayerVisibility,
    ) -> usize {
        let _ = self.finish_stroke();
        if !visibility.any() {
            return 0;
        }

        let mut changes = Vec::new();

        for src_row in 0..source.height {
            for src_col in 0..source.width {
                let dst_col = u32::from(top_left.0) + u32::from(src_col);
                let dst_row = u32::from(top_left.1) + u32::from(src_row);
                if dst_col >= u32::from(self.map.width) || dst_row >= u32::from(self.map.height) {
                    continue;
                }

                let src_idx = src_row as usize * source.width as usize + src_col as usize;
                let dst_idx = dst_row as usize * self.map.width as usize + dst_col as usize;
                let old_tile = self.map.tiles[dst_idx];
                let mut new_tile = old_tile;
                let source_tile = source.tiles[src_idx];

                if visibility.ground {
                    new_tile.ground = source_tile.ground;
                }
                if visibility.left_wall {
                    new_tile.left_wall = source_tile.left_wall;
                }
                if visibility.right_wall {
                    new_tile.right_wall = source_tile.right_wall;
                }

                if old_tile == new_tile {
                    continue;
                }

                self.map.tiles[dst_idx] = new_tile;
                changes.push(EditChange::Tile {
                    idx: dst_idx,
                    old: old_tile,
                    new: new_tile,
                });
            }
        }

        let changed_tiles = changes.len();
        let _ = self.push_history(changes);
        changed_tiles
    }

    pub fn move_selection_visible_layers(
        &mut self,
        selection: TileSelection,
        top_left: (u16, u16),
        visibility: LayerVisibility,
    ) -> usize {
        let _ = self.finish_stroke();
        if !visibility.any() {
            return 0;
        }

        let original_tiles = self.map.tiles.clone();
        let mut new_tiles = original_tiles.clone();
        let source = self.selection_map_for_visible_layers(selection, visibility);
        let transparent_zero = !visibility.all();
        let (min_col, min_row, max_col, max_row) = selection.normalized_bounds();

        for row in min_row..=max_row {
            for col in min_col..=max_col {
                let idx = row as usize * self.map.width as usize + col as usize;
                let mut new_tile = new_tiles[idx];
                if visibility.ground {
                    new_tile.ground = 0;
                }
                if visibility.left_wall {
                    new_tile.left_wall = 0;
                }
                if visibility.right_wall {
                    new_tile.right_wall = 0;
                }
                new_tiles[idx] = new_tile;
            }
        }

        for src_row in 0..source.height {
            for src_col in 0..source.width {
                let dst_col = top_left.0 + src_col;
                let dst_row = top_left.1 + src_row;
                let src_idx = src_row as usize * source.width as usize + src_col as usize;
                let dst_idx = dst_row as usize * self.map.width as usize + dst_col as usize;
                let source_tile = source.tiles[src_idx];
                let mut new_tile = new_tiles[dst_idx];

                if visibility.ground && (!transparent_zero || source_tile.ground != 0) {
                    new_tile.ground = source_tile.ground;
                }
                if visibility.left_wall && (!transparent_zero || source_tile.left_wall != 0) {
                    new_tile.left_wall = source_tile.left_wall;
                }
                if visibility.right_wall && (!transparent_zero || source_tile.right_wall != 0) {
                    new_tile.right_wall = source_tile.right_wall;
                }

                new_tiles[dst_idx] = new_tile;
            }
        }

        let mut changes = Vec::new();
        for (idx, (old_tile, new_tile)) in original_tiles.iter().zip(new_tiles.iter()).enumerate() {
            if old_tile == new_tile {
                continue;
            }

            self.map.tiles[idx] = *new_tile;
            changes.push(EditChange::Tile {
                idx,
                old: *old_tile,
                new: *new_tile,
            });
        }

        let changed_tiles = changes.len();
        let _ = self.push_history(changes);
        changed_tiles
    }

    #[cfg(test)]
    pub fn move_selection_to(&mut self, selection: TileSelection, top_left: (u16, u16)) -> usize {
        self.move_selection_visible_layers(selection, top_left, LayerVisibility::default())
    }

    pub fn display_name(&self) -> String {
        match self.kind {
            DocumentKind::Map => {
                if let Some(name) = &self.name_hint {
                    if let Some(map_id) = self.id_hint {
                        return format!("{map_id} - {name}");
                    }
                    return name.clone();
                }
                self.tab_display_name()
            }
            DocumentKind::Prefab => format!("Prefab {}", self.prefab_name()),
        }
    }

    pub fn tab_display_name(&self) -> String {
        match self.kind {
            DocumentKind::Map => {
                if let Some(name) = &self.name_hint {
                    return name.clone();
                }

                match &self.path {
                    Some(path) => path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("Unknown")
                        .to_string(),
                    None => String::from("Untitled"),
                }
            }
            DocumentKind::Prefab => format!("prefab: {}", self.prefab_name()),
        }
    }

    pub fn prefab_name(&self) -> String {
        self.name_hint
            .clone()
            .or_else(|| {
                self.path
                    .as_ref()
                    .and_then(|path| path.file_stem())
                    .and_then(|stem| stem.to_str())
                    .map(ToOwned::to_owned)
            })
            .unwrap_or_else(|| String::from("Untitled"))
    }

    pub fn suggested_filename(&self) -> String {
        match self.kind {
            DocumentKind::Map => {
                let mut name = self
                    .path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| {
                        let base = self.display_name();
                        if base.eq_ignore_ascii_case("Untitled") {
                            String::from("untitled.map")
                        } else {
                            format!("{base}.map")
                        }
                    });
                if !name.to_ascii_lowercase().ends_with(".map") {
                    name.push_str(".map");
                }
                name
            }
            DocumentKind::Prefab => {
                let base = sanitize_prefab_name(&self.prefab_name());
                if base.is_empty() {
                    String::from("untitled-prefab.ron")
                } else {
                    format!("{base}.ron")
                }
            }
        }
    }

    pub fn update_prefab_path(&mut self, path: PathBuf) {
        if self.kind != DocumentKind::Prefab {
            return;
        }

        self.name_hint = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(|stem| stem.to_string());
        self.path = Some(path);
    }

    pub fn clear_history(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.pending_stroke = None;
        self.dirty = false;
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn begin_layer_stroke(&mut self, layer: PaintLayer, paint_value: u16) {
        match self.pending_stroke.as_ref() {
            Some(stroke) if stroke.paint_value == paint_value && stroke.layer == layer => {}
            Some(_) => {
                self.finish_stroke();
                self.pending_stroke = Some(PendingStroke {
                    layer,
                    paint_value,
                    original_values: std::collections::BTreeMap::new(),
                });
            }
            None => {
                self.pending_stroke = Some(PendingStroke {
                    layer,
                    paint_value,
                    original_values: std::collections::BTreeMap::new(),
                });
            }
        }
    }

    pub fn paint_layer_stroke_tile(
        &mut self,
        layer: PaintLayer,
        col: u16,
        row: u16,
        paint_value: u16,
    ) -> bool {
        if col >= self.map.width || row >= self.map.height {
            return false;
        }

        self.begin_layer_stroke(layer, paint_value);
        let idx = row as usize * self.map.width as usize + col as usize;
        let tile = &mut self.map.tiles[idx];
        let current = layer.read_from_tile(tile);

        if current == paint_value {
            return false;
        }

        if let Some(stroke) = self.pending_stroke.as_mut() {
            stroke.original_values.entry(idx).or_insert(current);
        }

        match layer {
            PaintLayer::Ground => tile.ground = paint_value,
            PaintLayer::LeftWall => tile.left_wall = paint_value,
            PaintLayer::RightWall => tile.right_wall = paint_value,
        }
        self.dirty = true;
        true
    }

    pub fn finish_stroke(&mut self) -> bool {
        let Some(stroke) = self.pending_stroke.take() else {
            return false;
        };
        if stroke.original_values.is_empty() {
            return false;
        }

        let changes = stroke
            .original_values
            .into_iter()
            .map(|(idx, old)| match stroke.layer {
                PaintLayer::Ground => EditChange::Ground {
                    idx,
                    old,
                    new: stroke.paint_value,
                },
                PaintLayer::LeftWall => EditChange::LeftWall {
                    idx,
                    old,
                    new: stroke.paint_value,
                },
                PaintLayer::RightWall => EditChange::RightWall {
                    idx,
                    old,
                    new: stroke.paint_value,
                },
            })
            .collect::<Vec<_>>();

        self.push_history(changes)
    }

    fn push_history(&mut self, changes: Vec<EditChange>) -> bool {
        if changes.is_empty() {
            return false;
        }
        self.undo_stack.push(changes);
        self.redo_stack.clear();
        self.dirty = true;
        true
    }

    pub fn apply_prefab_stamp(&mut self, origin: (u16, u16), prefab: &map::Map) -> StampResult {
        self.finish_stroke();

        let mut changes = Vec::new();
        let mut clipped_tiles = 0usize;
        let anchor = placement_anchor(prefab);

        for prefab_row in 0..prefab.height {
            for prefab_col in 0..prefab.width {
                let source_idx = prefab_row as usize * prefab.width as usize + prefab_col as usize;
                let source_tile = prefab.tiles[source_idx];
                if source_tile.ground == 0
                    && source_tile.left_wall == 0
                    && source_tile.right_wall == 0
                {
                    continue;
                }

                let dst_col = i32::from(origin.0) + i32::from(prefab_col) - i32::from(anchor.0);
                let dst_row = i32::from(origin.1) + i32::from(prefab_row) - i32::from(anchor.1);
                if dst_col < 0
                    || dst_row < 0
                    || dst_col >= i32::from(self.map.width)
                    || dst_row >= i32::from(self.map.height)
                {
                    clipped_tiles += 1;
                    continue;
                }

                let dst_col = dst_col as usize;
                let dst_row = dst_row as usize;
                let dst_idx = dst_row * self.map.width as usize + dst_col;
                let old_tile = self.map.tiles[dst_idx];
                let mut new_tile = old_tile;

                if source_tile.ground != 0 {
                    new_tile.ground = source_tile.ground;
                }
                if source_tile.left_wall != 0 {
                    new_tile.left_wall = source_tile.left_wall;
                }
                if source_tile.right_wall != 0 {
                    new_tile.right_wall = source_tile.right_wall;
                }

                if new_tile == old_tile {
                    continue;
                }

                self.map.tiles[dst_idx] = new_tile;
                changes.push(EditChange::Tile {
                    idx: dst_idx,
                    old: old_tile,
                    new: new_tile,
                });
            }
        }

        let changed_tiles = changes.len();
        self.push_history(changes);

        StampResult {
            changed_tiles,
            clipped_tiles,
        }
    }

    pub fn undo(&mut self) -> bool {
        self.finish_stroke();

        let Some(batch) = self.undo_stack.pop() else {
            return false;
        };

        for change in &batch {
            change.apply_undo(&mut self.map);
        }
        self.redo_stack.push(batch);
        self.dirty = !self.undo_stack.is_empty();
        true
    }

    pub fn redo(&mut self) -> bool {
        self.finish_stroke();

        let Some(batch) = self.redo_stack.pop() else {
            return false;
        };

        for change in &batch {
            change.apply_redo(&mut self.map);
        }
        self.undo_stack.push(batch);
        self.dirty = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_grows_and_preserves_linear_prefix() {
        let mut doc = MapDocument::new_map(2, 2);
        doc.map.tiles[0].ground = 10;
        doc.map.tiles[1].ground = 11;
        doc.map.tiles[2].ground = 12;
        doc.map.tiles[3].ground = 13;

        doc.set_dimensions(3, 3).unwrap();

        assert_eq!(doc.map.width, 3);
        assert_eq!(doc.map.height, 3);
        assert_eq!(doc.map.tiles.len(), 9);
        assert_eq!(doc.map.tiles[0].ground, 10);
        assert_eq!(doc.map.tiles[1].ground, 11);
        assert_eq!(doc.map.tiles[2].ground, 12);
        assert_eq!(doc.map.tiles[3].ground, 13);
        assert_eq!(doc.map.tiles[8].ground, 0);
    }

    #[test]
    fn resize_shrinks_and_truncates_from_end() {
        let mut doc = MapDocument::new_map(3, 2);
        for (idx, tile) in doc.map.tiles.iter_mut().enumerate() {
            tile.ground = idx as u16 + 1;
        }

        doc.set_dimensions(2, 2).unwrap();

        assert_eq!(doc.map.width, 2);
        assert_eq!(doc.map.height, 2);
        assert_eq!(doc.map.tiles.len(), 4);
        assert_eq!(doc.map.tiles[0].ground, 1);
        assert_eq!(doc.map.tiles[1].ground, 2);
        assert_eq!(doc.map.tiles[2].ground, 3);
        assert_eq!(doc.map.tiles[3].ground, 4);
    }

    #[test]
    fn resize_same_tile_count_preserves_all_tiles() {
        let mut doc = MapDocument::new_map(20, 20);
        for (idx, tile) in doc.map.tiles.iter_mut().enumerate() {
            let id = idx as u16 + 1;
            tile.ground = id;
            tile.left_wall = id + 1000;
            tile.right_wall = id + 2000;
        }

        doc.set_dimensions(5, 80).unwrap();

        assert_eq!(doc.map.width, 5);
        assert_eq!(doc.map.height, 80);
        assert_eq!(doc.map.tiles.len(), 400);
        for (idx, tile) in doc.map.tiles.iter().enumerate() {
            let id = idx as u16 + 1;
            assert_eq!(tile.ground, id);
            assert_eq!(tile.left_wall, id + 1000);
            assert_eq!(tile.right_wall, id + 2000);
        }
    }

    #[test]
    fn resize_shrink_then_regrow_keeps_truncated_prefix() {
        let mut doc = MapDocument::new_map(20, 20);
        for (idx, tile) in doc.map.tiles.iter_mut().enumerate() {
            tile.ground = idx as u16 + 1;
        }

        doc.set_dimensions(15, 5).unwrap();
        doc.set_dimensions(5, 80).unwrap();

        assert_eq!(doc.map.tiles.len(), 400);
        for idx in 0..75 {
            assert_eq!(doc.map.tiles[idx].ground, idx as u16 + 1);
        }
        for idx in 75..400 {
            assert_eq!(doc.map.tiles[idx].ground, 0);
        }
    }

    #[test]
    fn resize_canvas_centered_grows_prefab_around_existing_tiles() {
        let mut doc = MapDocument::new_prefab(2, 2);
        doc.map.tiles[0].ground = 1;
        doc.map.tiles[1].ground = 2;
        doc.map.tiles[2].left_wall = 3;
        doc.map.tiles[3].right_wall = 4;

        doc.resize_canvas_centered(4, 4).unwrap();

        assert_eq!(doc.map.width, 4);
        assert_eq!(doc.map.height, 4);
        assert_eq!(doc.map.tiles[1 * 4 + 1].ground, 1);
        assert_eq!(doc.map.tiles[1 * 4 + 2].ground, 2);
        assert_eq!(doc.map.tiles[2 * 4 + 1].left_wall, 3);
        assert_eq!(doc.map.tiles[2 * 4 + 2].right_wall, 4);
    }

    #[test]
    fn resize_canvas_centered_shrinks_prefab_and_clips_edges() {
        let mut doc = MapDocument::new_prefab(4, 4);
        doc.map.tiles[0].ground = 1;
        doc.map.tiles[1 * 4 + 1].ground = 2;
        doc.map.tiles[2 * 4 + 2].left_wall = 3;
        doc.map.tiles[3 * 4 + 3].right_wall = 4;

        doc.resize_canvas_centered(2, 2).unwrap();

        assert_eq!(doc.map.width, 2);
        assert_eq!(doc.map.height, 2);
        assert_eq!(doc.map.tiles[0].ground, 2);
        assert_eq!(doc.map.tiles[3].left_wall, 3);
        assert!(doc.map.tiles.iter().all(|tile| tile.right_wall != 4));
        assert!(doc.map.tiles.iter().all(|tile| tile.ground != 1));
    }

    #[test]
    fn trim_canvas_to_content_removes_empty_border_without_losing_tiles() {
        let mut doc = MapDocument::new_prefab(5, 5);
        doc.map.tiles[1 * 5 + 2].ground = 7;
        doc.map.tiles[2 * 5 + 3].left_wall = 11;
        doc.map.tiles[3 * 5 + 4].right_wall = 13;

        let result = doc.trim_canvas_to_content().unwrap();

        assert_eq!(
            result,
            TrimCanvasResult::Trimmed {
                width: 3,
                height: 3
            }
        );
        assert_eq!(doc.map.width, 3);
        assert_eq!(doc.map.height, 3);
        assert_eq!(doc.map.tiles[0].ground, 7);
        assert_eq!(doc.map.tiles[1 * 3 + 1].left_wall, 11);
        assert_eq!(doc.map.tiles[2 * 3 + 2].right_wall, 13);
    }

    #[test]
    fn trim_canvas_to_content_reports_empty_prefab_without_resizing() {
        let mut doc = MapDocument::new_prefab(6, 6);

        let result = doc.trim_canvas_to_content().unwrap();

        assert_eq!(result, TrimCanvasResult::Empty);
        assert_eq!(doc.map.width, 6);
        assert_eq!(doc.map.height, 6);
    }

    #[test]
    fn wall_stroke_undo_redo_roundtrip() {
        let mut doc = MapDocument::new_map(2, 2);

        assert!(doc.paint_layer_stroke_tile(PaintLayer::LeftWall, 0, 0, 12));
        assert!(doc.paint_layer_stroke_tile(PaintLayer::LeftWall, 1, 0, 12));
        assert!(doc.finish_stroke());
        assert_eq!(doc.map.tiles[0].left_wall, 12);
        assert_eq!(doc.map.tiles[1].left_wall, 12);

        assert!(doc.undo());
        assert_eq!(doc.map.tiles[0].left_wall, 0);
        assert_eq!(doc.map.tiles[1].left_wall, 0);

        assert!(doc.redo());
        assert_eq!(doc.map.tiles[0].left_wall, 12);
        assert_eq!(doc.map.tiles[1].left_wall, 12);
    }

    #[test]
    fn clear_selection_layer_only_affects_target_layer_and_supports_undo_redo() {
        let mut doc = MapDocument::new_map(4, 4);

        for row in 1..=2 {
            for col in 1..=2 {
                let idx = row as usize * doc.map.width as usize + col as usize;
                doc.map.tiles[idx].ground = 10;
                doc.map.tiles[idx].left_wall = 20;
                doc.map.tiles[idx].right_wall = 30;
            }
        }

        doc.map.tiles[0].ground = 99;
        doc.map.tiles[0].left_wall = 98;
        doc.map.tiles[0].right_wall = 97;

        let selection = TileSelection::from_points((2, 2), (1, 1));
        assert_eq!(
            doc.clear_selection_layer(selection, PaintLayer::LeftWall),
            4
        );

        for row in 1..=2 {
            for col in 1..=2 {
                let idx = row as usize * doc.map.width as usize + col as usize;
                assert_eq!(doc.map.tiles[idx].ground, 10);
                assert_eq!(doc.map.tiles[idx].left_wall, 0);
                assert_eq!(doc.map.tiles[idx].right_wall, 30);
            }
        }
        assert_eq!(doc.map.tiles[0].ground, 99);
        assert_eq!(doc.map.tiles[0].left_wall, 98);
        assert_eq!(doc.map.tiles[0].right_wall, 97);

        assert!(doc.undo());
        for row in 1..=2 {
            for col in 1..=2 {
                let idx = row as usize * doc.map.width as usize + col as usize;
                assert_eq!(doc.map.tiles[idx].left_wall, 20);
            }
        }

        assert!(doc.redo());
        for row in 1..=2 {
            for col in 1..=2 {
                let idx = row as usize * doc.map.width as usize + col as usize;
                assert_eq!(doc.map.tiles[idx].left_wall, 0);
            }
        }
    }

    #[test]
    fn selection_map_and_paste_region_roundtrip_full_tiles() {
        let mut doc = MapDocument::new_map(4, 4);

        doc.map.tiles[1 * 4 + 1] = map::Tile {
            ground: 1,
            left_wall: 2,
            right_wall: 3,
        };
        doc.map.tiles[1 * 4 + 2] = map::Tile {
            ground: 4,
            left_wall: 0,
            right_wall: 5,
        };
        doc.map.tiles[2 * 4 + 1] = map::Tile::default();
        doc.map.tiles[2 * 4 + 2] = map::Tile {
            ground: 6,
            left_wall: 7,
            right_wall: 8,
        };

        let selection = TileSelection::from_points((1, 1), (2, 2));
        let copied = doc.selection_map(selection);
        assert_eq!(copied.width, 2);
        assert_eq!(copied.height, 2);
        assert_eq!(
            copied.tiles[0],
            map::Tile {
                ground: 1,
                left_wall: 2,
                right_wall: 3,
            }
        );
        assert_eq!(
            copied.tiles[1],
            map::Tile {
                ground: 4,
                left_wall: 0,
                right_wall: 5,
            }
        );
        assert_eq!(copied.tiles[2], map::Tile::default());
        assert_eq!(
            copied.tiles[3],
            map::Tile {
                ground: 6,
                left_wall: 7,
                right_wall: 8,
            }
        );

        for idx in [0usize, 1, 4, 5] {
            doc.map.tiles[idx] = map::Tile {
                ground: 99,
                left_wall: 98,
                right_wall: 97,
            };
        }

        assert_eq!(doc.paste_map_region((0, 0), &copied), 4);
        assert_eq!(
            doc.map.tiles[0],
            map::Tile {
                ground: 1,
                left_wall: 2,
                right_wall: 3,
            }
        );
        assert_eq!(
            doc.map.tiles[1],
            map::Tile {
                ground: 4,
                left_wall: 0,
                right_wall: 5,
            }
        );
        assert_eq!(doc.map.tiles[4], map::Tile::default());
        assert_eq!(
            doc.map.tiles[5],
            map::Tile {
                ground: 6,
                left_wall: 7,
                right_wall: 8,
            }
        );

        assert!(doc.undo());
        for idx in [0usize, 1, 4, 5] {
            assert_eq!(
                doc.map.tiles[idx],
                map::Tile {
                    ground: 99,
                    left_wall: 98,
                    right_wall: 97,
                }
            );
        }

        assert!(doc.redo());
        assert_eq!(doc.map.tiles[4], map::Tile::default());
        assert_eq!(doc.map.tiles[5].right_wall, 8);
    }

    #[test]
    fn move_selection_to_repositions_full_rectangle_with_undo_redo() {
        let mut doc = MapDocument::new_map(5, 4);
        doc.map.tiles[1 * 5 + 1] = map::Tile {
            ground: 1,
            left_wall: 2,
            right_wall: 3,
        };
        doc.map.tiles[1 * 5 + 2] = map::Tile {
            ground: 4,
            left_wall: 5,
            right_wall: 6,
        };
        doc.map.tiles[2 * 5 + 1] = map::Tile {
            ground: 7,
            left_wall: 8,
            right_wall: 9,
        };
        doc.map.tiles[2 * 5 + 2] = map::Tile::default();

        let selection = TileSelection::from_points((1, 1), (2, 2));
        assert_eq!(doc.move_selection_to(selection, (2, 0)), 5);

        assert_eq!(doc.map.tiles[1 * 5 + 1], map::Tile::default());
        assert_eq!(
            doc.map.tiles[1 * 5 + 2],
            map::Tile {
                ground: 7,
                left_wall: 8,
                right_wall: 9,
            }
        );
        assert_eq!(
            doc.map.tiles[2],
            map::Tile {
                ground: 1,
                left_wall: 2,
                right_wall: 3,
            }
        );
        assert_eq!(
            doc.map.tiles[3],
            map::Tile {
                ground: 4,
                left_wall: 5,
                right_wall: 6,
            }
        );
        assert_eq!(
            doc.map.tiles[7],
            map::Tile {
                ground: 7,
                left_wall: 8,
                right_wall: 9,
            }
        );

        assert!(doc.undo());
        assert_eq!(
            doc.map.tiles[1 * 5 + 1],
            map::Tile {
                ground: 1,
                left_wall: 2,
                right_wall: 3,
            }
        );
        assert_eq!(
            doc.map.tiles[1 * 5 + 2],
            map::Tile {
                ground: 4,
                left_wall: 5,
                right_wall: 6,
            }
        );

        assert!(doc.redo());
        assert_eq!(doc.map.tiles[1 * 5 + 1], map::Tile::default());
        assert_eq!(doc.map.tiles[2].ground, 1);
        assert_eq!(doc.map.tiles[7].left_wall, 8);
    }

    #[test]
    fn move_selection_visible_layers_preserves_ground_when_moving_walls_only() {
        let mut doc = MapDocument::new_map(5, 4);
        doc.map.tiles[1 * 5 + 1] = map::Tile {
            ground: 11,
            left_wall: 21,
            right_wall: 31,
        };
        doc.map.tiles[1 * 5 + 2] = map::Tile {
            ground: 12,
            left_wall: 22,
            right_wall: 32,
        };
        doc.map.tiles[2] = map::Tile {
            ground: 90,
            left_wall: 0,
            right_wall: 0,
        };
        doc.map.tiles[3] = map::Tile {
            ground: 91,
            left_wall: 0,
            right_wall: 0,
        };

        let selection = TileSelection::from_points((1, 1), (2, 1));
        let moved = doc.move_selection_visible_layers(
            selection,
            (2, 0),
            LayerVisibility {
                ground: false,
                left_wall: true,
                right_wall: true,
            },
        );
        assert_eq!(moved, 4);

        assert_eq!(
            doc.map.tiles[1 * 5 + 1],
            map::Tile {
                ground: 11,
                left_wall: 0,
                right_wall: 0,
            }
        );
        assert_eq!(
            doc.map.tiles[1 * 5 + 2],
            map::Tile {
                ground: 12,
                left_wall: 0,
                right_wall: 0,
            }
        );
        assert_eq!(
            doc.map.tiles[2],
            map::Tile {
                ground: 90,
                left_wall: 21,
                right_wall: 31,
            }
        );
        assert_eq!(
            doc.map.tiles[3],
            map::Tile {
                ground: 91,
                left_wall: 22,
                right_wall: 32,
            }
        );

        assert!(doc.undo());
        assert_eq!(
            doc.map.tiles[1 * 5 + 1],
            map::Tile {
                ground: 11,
                left_wall: 21,
                right_wall: 31,
            }
        );
        assert_eq!(doc.map.tiles[2].ground, 90);
        assert_eq!(doc.map.tiles[2].left_wall, 0);

        assert!(doc.redo());
        assert_eq!(doc.map.tiles[1 * 5 + 1].ground, 11);
        assert_eq!(doc.map.tiles[1 * 5 + 1].left_wall, 0);
        assert_eq!(doc.map.tiles[2].ground, 90);
        assert_eq!(doc.map.tiles[2].right_wall, 31);
    }

    #[test]
    fn move_selection_visible_layers_treats_empty_subset_tiles_as_transparent() {
        let mut doc = MapDocument::new_map(5, 3);
        doc.map.tiles[1 * 5] = map::Tile {
            ground: 1,
            left_wall: 10,
            right_wall: 20,
        };
        doc.map.tiles[1 * 5 + 1] = map::Tile {
            ground: 2,
            left_wall: 0,
            right_wall: 0,
        };
        doc.map.tiles[3] = map::Tile {
            ground: 50,
            left_wall: 60,
            right_wall: 70,
        };

        let selection = TileSelection::from_points((0, 1), (1, 1));
        let moved = doc.move_selection_visible_layers(
            selection,
            (2, 0),
            LayerVisibility {
                ground: false,
                left_wall: true,
                right_wall: true,
            },
        );
        assert_eq!(moved, 2);

        assert_eq!(
            doc.map.tiles[1 * 5],
            map::Tile {
                ground: 1,
                left_wall: 0,
                right_wall: 0,
            }
        );
        assert_eq!(
            doc.map.tiles[2],
            map::Tile {
                ground: 0,
                left_wall: 10,
                right_wall: 20,
            }
        );
        assert_eq!(
            doc.map.tiles[3],
            map::Tile {
                ground: 50,
                left_wall: 60,
                right_wall: 70,
            }
        );

        assert!(doc.undo());
        assert_eq!(doc.map.tiles[1 * 5].left_wall, 10);
        assert_eq!(doc.map.tiles[2].left_wall, 0);
        assert_eq!(doc.map.tiles[3].right_wall, 70);
    }

    #[test]
    fn visible_layer_selection_copy_paste_and_clear_preserve_hidden_layers() {
        let mut doc = MapDocument::new_map(4, 3);
        doc.map.tiles[1 * 4 + 1] = map::Tile {
            ground: 10,
            left_wall: 20,
            right_wall: 30,
        };
        doc.map.tiles[1 * 4 + 2] = map::Tile {
            ground: 11,
            left_wall: 21,
            right_wall: 31,
        };

        let selection = TileSelection::from_points((1, 1), (2, 1));
        let visible = LayerVisibility {
            ground: false,
            left_wall: true,
            right_wall: false,
        };

        let copied = doc.selection_map_for_visible_layers(selection, visible);
        assert_eq!(copied.tiles[0].ground, 0);
        assert_eq!(copied.tiles[0].left_wall, 20);
        assert_eq!(copied.tiles[0].right_wall, 0);

        doc.map.tiles[0] = map::Tile {
            ground: 99,
            left_wall: 98,
            right_wall: 97,
        };
        assert_eq!(doc.paste_visible_layers((0, 0), &copied, visible), 2);
        assert_eq!(
            doc.map.tiles[0],
            map::Tile {
                ground: 99,
                left_wall: 20,
                right_wall: 97,
            }
        );
        assert_eq!(doc.map.tiles[1].left_wall, 21);

        assert_eq!(doc.clear_selection_visible_layers(selection, visible), 2);
        assert_eq!(doc.map.tiles[1 * 4 + 1].ground, 10);
        assert_eq!(doc.map.tiles[1 * 4 + 1].left_wall, 0);
        assert_eq!(doc.map.tiles[1 * 4 + 1].right_wall, 30);
    }

    #[test]
    fn prefab_stamp_roundtrip_supports_undo_redo() {
        let mut doc = MapDocument::new_map(4, 4);
        doc.map.tiles[0].ground = 3;

        let mut prefab = map::Map::new(2, 1);
        prefab.tiles[0].left_wall = 100;
        prefab.tiles[1].right_wall = 101;

        let result = doc.apply_prefab_stamp((1, 1), &prefab);
        assert_eq!(result.changed_tiles, 2);
        assert_eq!(result.clipped_tiles, 0);
        assert_eq!(doc.map.tiles[5].left_wall, 100);
        assert_eq!(doc.map.tiles[6].right_wall, 101);

        assert!(doc.undo());
        assert_eq!(doc.map.tiles[5].left_wall, 0);
        assert_eq!(doc.map.tiles[6].right_wall, 0);

        assert!(doc.redo());
        assert_eq!(doc.map.tiles[5].left_wall, 100);
        assert_eq!(doc.map.tiles[6].right_wall, 101);
    }

    #[test]
    fn prefab_stamp_centers_on_occupied_bounds() {
        let mut doc = MapDocument::new_map(8, 8);
        let mut prefab = map::Map::new(6, 6);
        prefab.tiles[1 * 6 + 2].ground = 7;
        prefab.tiles[3 * 6 + 4].left_wall = 108;

        let result = doc.apply_prefab_stamp((3, 3), &prefab);

        assert_eq!(result.changed_tiles, 2);
        assert_eq!(result.clipped_tiles, 0);
        assert_eq!(doc.map.tiles[2 * 8 + 2].ground, 7);
        assert_eq!(doc.map.tiles[4 * 8 + 4].left_wall, 108);
    }
}
