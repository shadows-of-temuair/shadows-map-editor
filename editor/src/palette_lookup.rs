use std::collections::HashMap;

use archive::AssetPool;
use render::Palette;
use tracing::warn;

#[derive(Default)]
struct PaletteTable {
    entries: HashMap<u32, u32>,
    overrides: HashMap<u32, u32>,
}

impl PaletteTable {
    fn merge_from_bytes(&mut self, data: &[u8]) {
        let text = String::from_utf8_lossy(data);
        for line in text.lines() {
            match parse_table_line(line) {
                Some(TableLine::Override { id, palette }) => {
                    self.overrides.insert(id, palette);
                }
                Some(TableLine::Range {
                    start,
                    end,
                    palette,
                }) => {
                    for id in start..=end {
                        self.entries.insert(id, palette);
                    }
                }
                None => {}
            }
        }
    }

    fn palette_number_for_id(&self, id: u32) -> u32 {
        self.overrides
            .get(&id)
            .copied()
            .or_else(|| self.entries.get(&id).copied())
            .unwrap_or(0)
    }

    fn mapping_count(&self) -> usize {
        self.overrides.len() + self.entries.len()
    }

    fn is_empty(&self) -> bool {
        self.overrides.is_empty() && self.entries.is_empty()
    }
}

enum TableLine {
    Override { id: u32, palette: u32 },
    Range { start: u32, end: u32, palette: u32 },
}

fn parse_table_line(line: &str) -> Option<TableLine> {
    let fields: Vec<_> = line.split_whitespace().collect();
    match fields.as_slice() {
        [id, palette] => {
            let id = parse_non_negative_u32(id)?;
            let palette = parse_non_negative_u32(palette)?;
            Some(TableLine::Override { id, palette })
        }
        [start, end_or_palette, palette_or_override] => {
            let start = parse_non_negative_u32(start)?;
            let end_or_palette = parse_non_negative_u32(end_or_palette)?;
            let palette_or_override = palette_or_override.parse::<i32>().ok()?;

            if palette_or_override < 0 {
                // -1/-2 are DALib gender-specific overrides, not used by map
                // assets; ignore these entries.
                return None;
            }

            let palette = palette_or_override as u32;
            if end_or_palette < start {
                return None;
            }

            Some(TableLine::Range {
                start,
                end: end_or_palette,
                palette,
            })
        }
        _ => None,
    }
}

fn parse_non_negative_u32(value: &str) -> Option<u32> {
    let parsed = value.parse::<i32>().ok()?;
    (parsed >= 0).then_some(parsed as u32)
}

fn parse_palette_id(name: &str, prefix: &str) -> Option<u32> {
    let name_lower = name.to_ascii_lowercase();
    let prefix_lower = prefix.to_ascii_lowercase();
    if !name_lower.starts_with(&prefix_lower) || !name_lower.ends_with(".pal") {
        return None;
    }

    let stem = &name_lower[..name_lower.len().saturating_sub(4)];
    let suffix = &stem[prefix_lower.len()..];
    if suffix.is_empty() {
        return None;
    }

    suffix.parse::<u32>().ok()
}

fn collect_matching_names<'a>(pool: &'a AssetPool, prefix: &str, extension: &str) -> Vec<&'a str> {
    let prefix_lower = prefix.to_ascii_lowercase();
    let extension_lower = extension.to_ascii_lowercase();

    let mut names: Vec<_> = pool
        .names()
        .filter(|name| {
            let name_lower = name.to_ascii_lowercase();
            name_lower.starts_with(&prefix_lower) && name_lower.ends_with(&extension_lower)
        })
        .collect();

    names.sort_unstable_by_key(|name| name.to_ascii_lowercase());
    names
}

pub struct LoadedPaletteLookup {
    table: PaletteTable,
    palettes: HashMap<u32, Palette>,
    fallback_palette_id: u32,
}

impl LoadedPaletteLookup {
    /// Attempts to load a palette-table lookup by prefix (e.g. `mpt`).
    ///
    /// Returns `None` when no usable table/palette set is found.
    pub fn from_pool(pool: &AssetPool, prefix: &str) -> Option<Self> {
        let table_names = collect_matching_names(pool, prefix, ".tbl");
        if table_names.is_empty() {
            return None;
        }

        let mut table = PaletteTable::default();
        for name in table_names {
            if let Some(bytes) = pool.get(name) {
                table.merge_from_bytes(bytes);
            }
        }
        if table.is_empty() {
            return None;
        }

        let palette_names = collect_matching_names(pool, prefix, ".pal");
        if palette_names.is_empty() {
            return None;
        }

        let mut palettes = HashMap::new();
        for name in palette_names {
            let Some(palette_id) = parse_palette_id(name, prefix) else {
                continue;
            };
            let Some(bytes) = pool.get(name) else {
                continue;
            };
            match Palette::from_bytes(bytes) {
                Ok(palette) => {
                    palettes.insert(palette_id, palette);
                }
                Err(error) => {
                    warn!("Failed to parse palette {}: {}", name, error);
                }
            }
        }

        if palettes.is_empty() {
            return None;
        }

        let fallback_palette_id = if palettes.contains_key(&0) {
            0
        } else {
            *palettes.keys().min()?
        };

        Some(Self {
            table,
            palettes,
            fallback_palette_id,
        })
    }

    pub fn palette_for_id(&self, id: u32) -> Option<&Palette> {
        let palette_number = self.table.palette_number_for_id(id);
        self.palettes.get(&palette_number)
    }

    pub fn fallback_palette(&self) -> Option<&Palette> {
        self.palettes.get(&self.fallback_palette_id)
    }

    pub fn palette_count(&self) -> usize {
        self.palettes.len()
    }

    pub fn mapping_count(&self) -> usize {
        self.table.mapping_count()
    }
}

#[cfg(test)]
mod tests {
    use super::{PaletteTable, parse_table_line};

    #[test]
    fn parses_override_and_range_lines() {
        assert!(parse_table_line("12 7").is_some());
        assert!(parse_table_line("2 9 3").is_some());
        assert!(parse_table_line("2 9 -1").is_none());
        assert!(parse_table_line("abc 9 3").is_none());
    }

    #[test]
    fn table_prefers_overrides_for_exact_ids() {
        let mut table = PaletteTable::default();
        table.merge_from_bytes(b"2 4 5\n3 8\n");

        assert_eq!(table.palette_number_for_id(2), 5);
        assert_eq!(table.palette_number_for_id(3), 8);
        assert_eq!(table.palette_number_for_id(4), 5);
        assert_eq!(table.palette_number_for_id(10), 0);
    }
}
