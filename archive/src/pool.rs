use std::collections::HashMap;
use std::io;
use std::path::Path;

use crate::FileArchive;

/// Index into a specific archive and entry within it.
#[derive(Debug, Clone, Copy)]
struct AssetRef {
    archive: usize,
    entry: usize,
}

/// A flattened view over multiple `.dat` archives.
///
/// All archives in a directory are loaded and their entries merged into a single
/// namespace. Archives are loaded in alphabetical order; when names collide, the
/// last archive alphabetically wins (e.g. `z.dat` takes priority over `a.dat`).
pub struct AssetPool {
    archives: Vec<FileArchive>,
    index: HashMap<String, AssetRef>,
}

impl AssetPool {
    /// Loads every `.dat` file in `dir`, sorted alphabetically.
    pub fn load(dir: impl AsRef<Path>) -> io::Result<Self> {
        let dir = dir.as_ref();

        let mut dat_files: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("dat") {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();

        dat_files.sort();

        let mut archives = Vec::with_capacity(dat_files.len());
        let mut index = HashMap::new();

        for path in &dat_files {
            let archive = FileArchive::open(path)?;
            let archive_idx = archives.len();

            for (entry_idx, entry) in archive.entries().iter().enumerate() {
                // Later archives (alphabetically) overwrite earlier ones
                index.insert(
                    entry.name.clone(),
                    AssetRef {
                        archive: archive_idx,
                        entry: entry_idx,
                    },
                );
            }

            archives.push(archive);
        }

        Ok(Self { archives, index })
    }

    /// Returns the raw bytes of an asset by file name, or `None` if not found.
    pub fn get(&self, name: &str) -> Option<&[u8]> {
        let asset_ref = self.index.get(name)?;
        let archive = &self.archives[asset_ref.archive];
        let entry = &archive.entries()[asset_ref.entry];
        archive.get(&entry.name)
    }

    /// Returns `true` if an asset with this name exists in any archive.
    pub fn contains(&self, name: &str) -> bool {
        self.index.contains_key(name)
    }

    /// Total number of unique asset names across all archives.
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Whether the pool contains no assets.
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Returns an iterator over all unique asset names in the pool.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.index.keys().map(|s| s.as_str())
    }

    /// Number of loaded archives.
    pub fn archive_count(&self) -> usize {
        self.archives.len()
    }
}
