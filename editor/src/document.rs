use std::path::PathBuf;

use eframe::egui;

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
}

impl MapDocument {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            map: map::Map::new(width, height),
            path: None,
            dirty: false,
            camera: Camera::default(),
        }
    }

    pub fn open(path: PathBuf) -> std::io::Result<Self> {
        let map = map::Map::load(&path)?;
        Ok(Self {
            map,
            path: Some(path),
            dirty: false,
            camera: Camera::default(),
        })
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        if let Some(ref path) = self.path {
            self.map.save(path)?;
            self.dirty = false;
        }
        Ok(())
    }

    pub fn save_as(&mut self, path: PathBuf) -> std::io::Result<()> {
        self.map.save(&path)?;
        self.path = Some(path);
        self.dirty = false;
        Ok(())
    }

    pub fn set_dimensions(&mut self, width: u16, height: u16) {
        self.map.width = width;
        self.map.height = height;
        self.camera = Camera::default();
        self.dirty = true;
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
}
