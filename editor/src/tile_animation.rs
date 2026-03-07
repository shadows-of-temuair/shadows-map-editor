use std::collections::HashMap;

#[derive(Clone, Debug)]
struct GroundAnimationEntry {
    tile_sequence: Box<[u16]>,
    animation_interval_ms: u32,
}

#[derive(Clone, Debug, Default)]
pub struct GroundAnimationTable {
    entries: Vec<GroundAnimationEntry>,
    lookup: HashMap<u16, (usize, usize)>,
}

impl GroundAnimationTable {
    /// Parses a Chaos tile animation table such as `gndani.tbl` or `stcani.tbl`.
    ///
    /// Tile IDs are kept in the raw asset ID space used by the reference tools.
    pub fn from_tbl_bytes(bytes: &[u8]) -> Result<Self, String> {
        let text = String::from_utf8_lossy(bytes);
        let mut table = Self::default();

        for line in text.lines() {
            let parsed_values: Vec<u16> = line
                .split_whitespace()
                .filter_map(|part| part.parse::<u16>().ok())
                .collect();

            if parsed_values.len() < 2 {
                continue;
            }

            let (tile_ids, interval_token) = parsed_values.split_at(parsed_values.len() - 1);
            if tile_ids.is_empty() {
                continue;
            }

            let animation_interval_ms = u32::from(interval_token[0]).saturating_mul(100).max(1);
            let tile_sequence: Vec<u16> = tile_ids.to_vec();
            let entry_index = table.entries.len();

            for (sequence_index, tile_id) in tile_sequence.iter().copied().enumerate() {
                table.lookup.insert(tile_id, (entry_index, sequence_index));
            }

            table.entries.push(GroundAnimationEntry {
                tile_sequence: tile_sequence.into_boxed_slice(),
                animation_interval_ms,
            });
        }

        if table.entries.is_empty() {
            return Err("gndani.tbl contained no animation entries".to_string());
        }

        Ok(table)
    }

    pub fn from_gndani_bytes(bytes: &[u8]) -> Result<Self, String> {
        Self::from_tbl_bytes(bytes)
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn minimum_interval_ms(&self) -> Option<u32> {
        self.entries
            .iter()
            .map(|entry| entry.animation_interval_ms)
            .min()
    }

    pub fn animation_metadata(&self, tile_id: u16) -> Option<(usize, u32)> {
        let &(entry_index, _) = self.lookup.get(&tile_id)?;
        let entry = &self.entries[entry_index];
        Some((entry.tile_sequence.len(), entry.animation_interval_ms))
    }

    pub fn animation_start_tile_id(&self, tile_id: u16) -> Option<u16> {
        let &(entry_index, _) = self.lookup.get(&tile_id)?;
        self.entries[entry_index].tile_sequence.first().copied()
    }

    pub fn representative_tile_id(&self, tile_id: u16) -> u16 {
        self.animation_start_tile_id(tile_id).unwrap_or(tile_id)
    }

    pub fn tile_sequence(&self, tile_id: u16) -> Option<&[u16]> {
        let &(entry_index, _) = self.lookup.get(&tile_id)?;
        Some(&self.entries[entry_index].tile_sequence)
    }

    pub fn animated_tile_id(&self, tile_id: u16, time_seconds: f64) -> u16 {
        if tile_id == 0 {
            return 0;
        }

        let Some(&(entry_index, current_index)) = self.lookup.get(&tile_id) else {
            return tile_id;
        };
        let entry = &self.entries[entry_index];
        if entry.tile_sequence.len() <= 1 {
            return tile_id;
        }

        let elapsed_ms = (time_seconds.max(0.0) * 1000.0).floor() as u64;
        let frame_offset = (elapsed_ms / u64::from(entry.animation_interval_ms))
            % entry.tile_sequence.len() as u64;
        let sequence_index = (current_index + frame_offset as usize) % entry.tile_sequence.len();

        entry.tile_sequence[sequence_index]
    }
}

#[cfg(test)]
mod tests {
    use super::GroundAnimationTable;

    #[test]
    fn parses_sequences_and_intervals() {
        let table = GroundAnimationTable::from_gndani_bytes(b"0 1 2 5\n10 11 2\n").unwrap();

        assert_eq!(table.entry_count(), 2);
        assert_eq!(table.minimum_interval_ms(), Some(200));
    }

    #[test]
    fn rotates_from_current_tile() {
        let table = GroundAnimationTable::from_gndani_bytes(b"0 1 2 5\n").unwrap();

        assert_eq!(table.animated_tile_id(1, 0.0), 1);
        assert_eq!(table.animated_tile_id(1, 0.5), 2);
        assert_eq!(table.animated_tile_id(1, 1.0), 0);
    }

    #[test]
    fn leaves_non_animated_tiles_unchanged() {
        let table = GroundAnimationTable::from_gndani_bytes(b"0 1 2 5\n").unwrap();

        assert_eq!(table.animated_tile_id(99, 2.0), 99);
        assert_eq!(table.animated_tile_id(0, 2.0), 0);
    }

    #[test]
    fn representative_tile_is_first_sequence_frame() {
        let table = GroundAnimationTable::from_gndani_bytes(b"10 11 12 2\n").unwrap();

        assert_eq!(table.animation_start_tile_id(12), Some(10));
        assert_eq!(table.representative_tile_id(11), 10);
        assert_eq!(table.representative_tile_id(99), 99);
    }
}
