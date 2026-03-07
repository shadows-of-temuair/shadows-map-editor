use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use tracing::warn;

pub const PREFABS_DIR: &str = "prefabs";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OccupiedBounds {
    pub min_col: u16,
    pub min_row: u16,
    pub max_col: u16,
    pub max_row: u16,
}

impl OccupiedBounds {
    pub fn width(self) -> u16 {
        self.max_col - self.min_col + 1
    }

    pub fn height(self) -> u16 {
        self.max_row - self.min_row + 1
    }
}

#[derive(Clone, Debug)]
pub struct PrefabAsset {
    pub path: PathBuf,
    pub name: String,
    pub map: map::Map,
}

pub fn tile_is_occupied(tile: &map::Tile) -> bool {
    tile.ground != 0 || tile.left_wall != 0 || tile.right_wall != 0
}

pub fn occupied_bounds(map: &map::Map) -> Option<OccupiedBounds> {
    let mut min_col = u16::MAX;
    let mut min_row = u16::MAX;
    let mut max_col = 0u16;
    let mut max_row = 0u16;
    let mut found = false;

    for row in 0..map.height {
        for col in 0..map.width {
            let idx = row as usize * map.width as usize + col as usize;
            let tile = map.tiles[idx];
            if !tile_is_occupied(&tile) {
                continue;
            }

            found = true;
            min_col = min_col.min(col);
            min_row = min_row.min(row);
            max_col = max_col.max(col);
            max_row = max_row.max(row);
        }
    }

    found.then_some(OccupiedBounds {
        min_col,
        min_row,
        max_col,
        max_row,
    })
}

pub fn placement_anchor(map: &map::Map) -> (u16, u16) {
    occupied_bounds(map)
        .map(|bounds| {
            (
                ((u32::from(bounds.min_col) + u32::from(bounds.max_col)) / 2) as u16,
                ((u32::from(bounds.min_row) + u32::from(bounds.max_row)) / 2) as u16,
            )
        })
        .unwrap_or((0, 0))
}

pub fn centered_canvas_offset(old_extent: u16, new_extent: u16) -> i32 {
    i32::from(new_extent / 2) - i32::from(old_extent / 2)
}

pub fn centered_canvas_tile_loss(map: &map::Map, new_width: u16, new_height: u16) -> usize {
    let col_offset = centered_canvas_offset(map.width, new_width);
    let row_offset = centered_canvas_offset(map.height, new_height);
    let mut lost_tiles = 0usize;

    for row in 0..map.height {
        for col in 0..map.width {
            let idx = row as usize * map.width as usize + col as usize;
            if !tile_is_occupied(&map.tiles[idx]) {
                continue;
            }

            let new_col = i32::from(col) + col_offset;
            let new_row = i32::from(row) + row_offset;
            if new_col < 0
                || new_row < 0
                || new_col >= i32::from(new_width)
                || new_row >= i32::from(new_height)
            {
                lost_tiles += 1;
            }
        }
    }

    lost_tiles
}

impl PrefabAsset {
    pub fn file_stem_name(&self) -> &str {
        self.path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or(self.name.as_str())
    }

    pub fn occupied_bounds(&self) -> Option<OccupiedBounds> {
        occupied_bounds(&self.map)
    }

    pub fn occupied_dimensions(&self) -> (u16, u16) {
        self.occupied_bounds()
            .map(|bounds| (bounds.width(), bounds.height()))
            .unwrap_or((0, 0))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrefabFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub width: u16,
    pub height: u16,
    #[serde(default)]
    pub tiles: Vec<PrefabTile>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct PrefabTile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ground: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left_wall: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right_wall: Option<u16>,
}

impl PrefabFile {
    pub fn from_map(map: &map::Map, name: Option<String>) -> Self {
        Self {
            name,
            width: map.width,
            height: map.height,
            tiles: map
                .tiles
                .iter()
                .map(|tile| PrefabTile {
                    ground: (tile.ground != 0).then_some(tile.ground),
                    left_wall: (tile.left_wall != 0).then_some(tile.left_wall),
                    right_wall: (tile.right_wall != 0).then_some(tile.right_wall),
                })
                .collect(),
        }
    }

    pub fn into_map(self) -> io::Result<map::Map> {
        let expected = self.width as usize * self.height as usize;
        if self.tiles.len() != expected {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "prefab tile count mismatch: expected {}, found {}",
                    expected,
                    self.tiles.len()
                ),
            ));
        }

        Ok(map::Map {
            width: self.width,
            height: self.height,
            tiles: self
                .tiles
                .into_iter()
                .map(|tile| map::Tile {
                    ground: tile.ground.unwrap_or(0),
                    left_wall: tile.left_wall.unwrap_or(0),
                    right_wall: tile.right_wall.unwrap_or(0),
                })
                .collect(),
        })
    }

    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let source = fs::read_to_string(path)?;
        ron::from_str(&source).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to parse {}: {}", path.display(), error),
            )
        })
    }

    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let pretty = ron::ser::PrettyConfig::new()
            .new_line("\n")
            .indentor("  ")
            .separate_tuple_members(true)
            .enumerate_arrays(true);
        let serialized = ron::ser::to_string_pretty(self, pretty).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to serialize {}: {}", path.display(), error),
            )
        })?;
        fs::write(path, serialized)
    }
}

pub fn ensure_prefabs_dir() -> io::Result<PathBuf> {
    let dir = PathBuf::from(PREFABS_DIR);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn load_prefab_asset(path: impl AsRef<Path>) -> io::Result<PrefabAsset> {
    let path = path.as_ref().to_path_buf();
    let file = PrefabFile::load(&path)?;
    let name = file.name.clone().unwrap_or_else(|| {
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("untitled-prefab")
            .to_string()
    });
    let map = file.into_map()?;
    Ok(PrefabAsset { path, name, map })
}

pub fn load_prefab_assets() -> io::Result<Vec<PrefabAsset>> {
    let dir = ensure_prefabs_dir()?;
    let mut assets = Vec::new();

    for entry in fs::read_dir(&dir)? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let path = entry.path();
        let is_ron = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("ron"))
            .unwrap_or(false);
        if !is_ron {
            continue;
        }

        match load_prefab_asset(&path) {
            Ok(asset) => assets.push(asset),
            Err(error) => warn!("Skipping prefab {}: {}", path.display(), error),
        }
    }

    assets.sort_by(|a, b| {
        a.file_stem_name()
            .to_lowercase()
            .cmp(&b.file_stem_name().to_lowercase())
    });
    Ok(assets)
}

pub fn sanitize_prefab_name(input: &str) -> String {
    let mut sanitized = String::with_capacity(input.len());
    let mut last_was_dash = false;

    for ch in input.chars() {
        let valid = ch.is_ascii_alphanumeric();
        if valid {
            sanitized.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash {
            sanitized.push('-');
            last_was_dash = true;
        }
    }

    sanitized.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefab_roundtrip_preserves_non_zero_tiles() {
        let mut map = map::Map::new(2, 2);
        map.tiles[0].ground = 7;
        map.tiles[1].left_wall = 88;
        map.tiles[3].right_wall = 91;

        let prefab = PrefabFile::from_map(&map, Some(String::from("Tree")));
        let rebuilt = prefab.into_map().unwrap();

        assert_eq!(rebuilt.width, 2);
        assert_eq!(rebuilt.height, 2);
        assert_eq!(rebuilt.tiles[0].ground, 7);
        assert_eq!(rebuilt.tiles[1].left_wall, 88);
        assert_eq!(rebuilt.tiles[3].right_wall, 91);
    }

    #[test]
    fn sanitize_prefab_name_normalizes_spacing() {
        assert_eq!(sanitize_prefab_name(" Big Tree 01 "), "big-tree-01");
    }

    #[test]
    fn occupied_dimensions_ignore_empty_canvas_border() {
        let mut asset = PrefabAsset {
            path: PathBuf::from("prefabs/tree.ron"),
            name: String::from("Tree"),
            map: map::Map::new(5, 5),
        };
        asset.map.tiles[1 * 5 + 2].ground = 7;
        asset.map.tiles[3 * 5 + 4].left_wall = 10;

        assert_eq!(asset.occupied_dimensions(), (3, 3));
    }

    #[test]
    fn placement_anchor_uses_occupied_bounds_center() {
        let mut map = map::Map::new(6, 6);
        map.tiles[1 * 6 + 2].ground = 7;
        map.tiles[3 * 6 + 4].left_wall = 10;

        assert_eq!(placement_anchor(&map), (3, 2));
    }

    #[test]
    fn centered_canvas_tile_loss_counts_clipped_non_empty_tiles() {
        let mut map = map::Map::new(4, 4);
        map.tiles[0].ground = 1;
        map.tiles[5].ground = 2;
        map.tiles[10].left_wall = 3;
        map.tiles[15].right_wall = 4;

        assert_eq!(centered_canvas_tile_loss(&map, 2, 2), 2);
        assert_eq!(centered_canvas_tile_loss(&map, 6, 6), 0);
    }
}
