use std::{collections::HashMap, io, path::Path};

use serde::Deserialize;
use tracing::{info, warn};

#[derive(Clone, Debug)]
pub struct MapMetadataHint {
    pub map_id: u32,
    pub map_name: String,
    pub map_size: Option<(u16, u16)>,
}

pub struct MapList {
    by_id: HashMap<u32, MapMetadataHint>,
}

impl MapList {
    pub fn load_if_exists(path: impl AsRef<Path>) -> Option<Self> {
        let path = path.as_ref();
        let contents = match std::fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return None,
            Err(error) => {
                warn!(
                    "Failed to read map metadata from {}: {}",
                    path.display(),
                    error
                );
                return None;
            }
        };

        let parsed: RonMapList = match ron::from_str(&contents) {
            Ok(parsed) => parsed,
            Err(error) => {
                warn!(
                    "Failed to parse map metadata from {}: {}",
                    path.display(),
                    error
                );
                return None;
            }
        };

        let mut by_id = HashMap::with_capacity(parsed.maps.len());
        for entry in parsed.maps {
            let map_size = match entry.map_size {
                RonMapSize::Pair(size) => Some(size),
                RonMapSize::Text(_) => None,
            };

            by_id.insert(
                entry.map_id,
                MapMetadataHint {
                    map_id: entry.map_id,
                    map_name: entry.map_name,
                    map_size,
                },
            );
        }

        info!(
            "Loaded map metadata for {} entries from {}",
            by_id.len(),
            path.display()
        );
        Some(Self { by_id })
    }

    pub fn hint_for_path(&self, path: &Path) -> Option<MapMetadataHint> {
        let map_id = Self::extract_map_id(path)?;
        self.by_id.get(&map_id).cloned()
    }

    pub fn extract_map_id(path: &Path) -> Option<u32> {
        let stem = path.file_stem()?.to_str()?;
        let lower = stem.to_ascii_lowercase();
        let rest = lower.strip_prefix("lod")?;
        let digits: String = rest.chars().take_while(|ch| ch.is_ascii_digit()).collect();
        if digits.is_empty() {
            return None;
        }
        digits.parse::<u32>().ok()
    }
}

#[derive(Deserialize)]
struct RonMapList {
    maps: Vec<RonMapEntry>,
}

#[derive(Deserialize)]
struct RonMapEntry {
    map_id: u32,
    map_name: String,
    map_size: RonMapSize,
}

#[derive(Deserialize)]
#[serde(untagged)]
#[allow(dead_code)]
enum RonMapSize {
    Pair((u16, u16)),
    Text(String),
}

#[cfg(test)]
mod tests {
    use super::{MapList, RonMapList, RonMapSize};
    use std::path::Path;

    #[test]
    fn extract_map_id_from_lod_prefix_digits() {
        assert_eq!(MapList::extract_map_id(Path::new("LOD185.MAP")), Some(185));
        assert_eq!(MapList::extract_map_id(Path::new("lod300.map")), Some(300));
        assert_eq!(
            MapList::extract_map_id(Path::new("lod500-test.map")),
            Some(500)
        );
        assert_eq!(
            MapList::extract_map_id(Path::new("lod-whatever-43434u59837598239820.map")),
            None
        );
        assert_eq!(MapList::extract_map_id(Path::new("001.map")), None);
        assert_eq!(MapList::extract_map_id(Path::new("foo_lod200.map")), None);
        assert_eq!(MapList::extract_map_id(Path::new("foo.map")), None);
    }

    #[test]
    fn ron_map_size_accepts_tuple_and_string() {
        let parsed: RonMapList = ron::from_str(
            r#"(
                maps: [
                    (map_id: 1, map_name: "A", map_size: (50, 50)),
                    (map_id: 2, map_name: "B", map_size: "-100100"),
                ],
            )"#,
        )
        .expect("RON should parse with mixed map_size formats");

        assert_eq!(parsed.maps.len(), 2);
        assert!(matches!(
            parsed.maps[0].map_size,
            RonMapSize::Pair((50, 50))
        ));
        assert!(matches!(parsed.maps[1].map_size, RonMapSize::Text(_)));
    }
}
