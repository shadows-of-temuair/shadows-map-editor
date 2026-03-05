use std::path::PathBuf;

use eframe::egui;

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

struct PendingGroundStroke {
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
    pub camera: Camera,
    undo_stack: Vec<Vec<EditChange>>,
    redo_stack: Vec<Vec<EditChange>>,
    pending_ground_stroke: Option<PendingGroundStroke>,
}

impl MapDocument {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            map: map::Map::new(width, height),
            path: None,
            dirty: false,
            camera: Camera::default(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            pending_ground_stroke: None,
        }
    }

    pub fn open(path: PathBuf) -> std::io::Result<Self> {
        let map = map::Map::load(&path)?;
        Ok(Self {
            map,
            path: Some(path),
            dirty: false,
            camera: Camera::default(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            pending_ground_stroke: None,
        })
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        self.finish_ground_stroke();
        if let Some(ref path) = self.path {
            self.map.save(path)?;
            self.dirty = false;
        }
        Ok(())
    }

    pub fn save_as(&mut self, path: PathBuf) -> std::io::Result<()> {
        self.finish_ground_stroke();
        self.map.save(&path)?;
        self.path = Some(path);
        self.dirty = false;
        Ok(())
    }

    pub fn set_dimensions(&mut self, width: u16, height: u16) {
        self.finish_ground_stroke();
        self.map.width = width;
        self.map.height = height;
        self.camera = Camera::default();
        self.dirty = true;
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn display_name(&self) -> String {
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
        self.pending_ground_stroke = None;
        self.dirty = false;
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn begin_ground_stroke(&mut self, paint_value: u16) {
        match self.pending_ground_stroke.as_ref() {
            Some(stroke) if stroke.paint_value == paint_value => {}
            Some(_) => {
                self.finish_ground_stroke();
                self.pending_ground_stroke = Some(PendingGroundStroke {
                    paint_value,
                    original_values: std::collections::BTreeMap::new(),
                });
            }
            None => {
                self.pending_ground_stroke = Some(PendingGroundStroke {
                    paint_value,
                    original_values: std::collections::BTreeMap::new(),
                });
            }
        }
    }

    pub fn paint_ground_stroke_tile(&mut self, col: u16, row: u16, paint_value: u16) -> bool {
        if col >= self.map.width || row >= self.map.height {
            return false;
        }

        self.begin_ground_stroke(paint_value);
        let idx = row as usize * self.map.width as usize + col as usize;
        let tile = &mut self.map.tiles[idx];

        if tile.ground == paint_value {
            return false;
        }

        if let Some(stroke) = self.pending_ground_stroke.as_mut() {
            stroke.original_values.entry(idx).or_insert(tile.ground);
        }

        tile.ground = paint_value;
        self.dirty = true;
        true
    }

    pub fn finish_ground_stroke(&mut self) -> bool {
        let Some(stroke) = self.pending_ground_stroke.take() else {
            return false;
        };
        if stroke.original_values.is_empty() {
            return false;
        }

        let changes = stroke
            .original_values
            .into_iter()
            .map(|(idx, old)| EditChange::Ground {
                idx,
                old,
                new: stroke.paint_value,
            })
            .collect::<Vec<_>>();

        self.undo_stack.push(changes);
        self.redo_stack.clear();
        self.dirty = true;
        true
    }

    pub fn undo(&mut self) -> bool {
        self.finish_ground_stroke();

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
        self.finish_ground_stroke();

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
