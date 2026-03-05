use std::path::PathBuf;

use eframe::egui;
use tracing::warn;

use crate::map_list::MapMetadataHint;

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

#[derive(Clone, Copy)]
#[allow(dead_code)]
enum EditChange {
    Ground { idx: usize, old: u16, new: u16 },
    LeftWall { idx: usize, old: u16, new: u16 },
    RightWall { idx: usize, old: u16, new: u16 },
}

impl EditChange {
    fn apply_undo(&self, map: &mut map::Map) {
        let (idx, value, layer) = match self {
            EditChange::Ground { idx, old, .. } => (*idx, *old, 0u8),
            EditChange::LeftWall { idx, old, .. } => (*idx, *old, 1u8),
            EditChange::RightWall { idx, old, .. } => (*idx, *old, 2u8),
        };
        let Some(tile) = map.tiles.get_mut(idx) else {
            return;
        };
        match layer {
            0 => tile.ground = value,
            1 => tile.left_wall = value,
            _ => tile.right_wall = value,
        }
    }

    fn apply_redo(&self, map: &mut map::Map) {
        let (idx, value, layer) = match self {
            EditChange::Ground { idx, new, .. } => (*idx, *new, 0u8),
            EditChange::LeftWall { idx, new, .. } => (*idx, *new, 1u8),
            EditChange::RightWall { idx, new, .. } => (*idx, *new, 2u8),
        };
        let Some(tile) = map.tiles.get_mut(idx) else {
            return;
        };
        match layer {
            0 => tile.ground = value,
            1 => tile.left_wall = value,
            _ => tile.right_wall = value,
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

pub struct MapDocument {
    pub map: map::Map,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub map_id_hint: Option<u32>,
    pub map_name_hint: Option<String>,
    pub camera: Camera,
    undo_stack: Vec<Vec<EditChange>>,
    redo_stack: Vec<Vec<EditChange>>,
    pending_stroke: Option<PendingStroke>,
}

impl MapDocument {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            map: map::Map::new(width, height),
            path: None,
            dirty: false,
            map_id_hint: None,
            map_name_hint: None,
            camera: Camera::default(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            pending_stroke: None,
        }
    }

    pub fn open(path: PathBuf, metadata_hint: Option<MapMetadataHint>) -> std::io::Result<Self> {
        let mut map = map::Map::load(&path)?;

        let map_id_hint = metadata_hint.as_ref().map(|hint| hint.map_id);
        let map_name_hint = metadata_hint.as_ref().map(|hint| hint.map_name.clone());

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
            map,
            path: Some(path),
            dirty: false,
            map_id_hint,
            map_name_hint,
            camera: Camera::default(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            pending_stroke: None,
        })
    }

    pub fn save_as(&mut self, path: PathBuf) -> std::io::Result<()> {
        self.finish_stroke();
        self.map.save(&path)?;
        self.path = Some(path);
        self.dirty = false;
        Ok(())
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
                "Unable to resize map to {}x{} ({} tiles): {}",
                width, height, new_count, err
            ));
        }
        new_tiles.resize(new_count, map::Tile::default());

        // Preserve tile data in linear order; changing dimensions only changes
        // how `(col,row)` indices map onto the same flat buffer.
        let copy_count = old_count.min(new_count);
        new_tiles[..copy_count].copy_from_slice(&self.map.tiles[..copy_count]);

        self.map.width = width;
        self.map.height = height;
        self.map.tiles = new_tiles;
        self.camera = Camera::default();
        self.dirty = true;
        self.undo_stack.clear();
        self.redo_stack.clear();
        Ok(())
    }

    pub fn display_name(&self) -> String {
        if let Some(name) = &self.map_name_hint {
            if let Some(map_id) = self.map_id_hint {
                return format!("{map_id} - {name}");
            }
            return name.clone();
        }

        self.tab_display_name()
    }

    pub fn tab_display_name(&self) -> String {
        if let Some(name) = &self.map_name_hint {
            return name.clone();
        }

        match &self.path {
            Some(p) => p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string(),
            None => "Untitled".to_string(),
        }
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

        self.undo_stack.push(changes);
        self.redo_stack.clear();
        self.dirty = true;
        true
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
        let mut doc = MapDocument::new(2, 2);
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
        let mut doc = MapDocument::new(3, 2);
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
        let mut doc = MapDocument::new(20, 20);
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
        let mut doc = MapDocument::new(20, 20);
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
    fn wall_stroke_undo_redo_roundtrip() {
        let mut doc = MapDocument::new(2, 2);

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
}
