use std::fmt;

use crate::Palette;
use crate::hpf::HpfSprite;

/// Maximum atlas dimension in pixels. Chosen to fit within common GPU limits.
const MAX_ATLAS_SIDE: u32 = 8192;

/// Metadata for a single sprite within the atlas.
#[derive(Clone, Copy, Debug)]
pub struct SpriteEntry {
    /// X offset of the sprite in the atlas (pixels).
    pub x: u32,
    /// Y offset of the sprite in the atlas (pixels).
    pub y: u32,
    /// Width of the sprite (pixels).
    pub width: u32,
    /// Height of the sprite (pixels).
    pub height: u32,
}

/// A rasterized RGBA sprite atlas for variable-height sprites.
///
/// Sprites are packed into columns of fixed width using a greedy
/// shortest-column strategy: each sprite is placed in whichever column
/// currently has the least total height. This minimises the overall atlas
/// height and keeps it within GPU texture limits.
///
/// Each sprite's position and size is tracked in an `entries` table so that
/// UV coordinates can be computed for any sprite by its ID.
pub struct SpriteAtlas {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    entries: Vec<Option<SpriteEntry>>,
}

#[derive(Debug)]
pub enum SpriteAtlasError {
    NoSprites,
}

impl fmt::Display for SpriteAtlasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpriteAtlasError::NoSprites => write!(f, "no sprites provided for atlas"),
        }
    }
}

impl std::error::Error for SpriteAtlasError {}

impl SpriteAtlas {
    /// Build a sprite atlas from decoded HPF sprites.
    ///
    /// `sprites` is indexed by sprite ID. `None` entries represent
    /// empty/missing sprite slots and are preserved so that lookups by tile ID
    /// remain aligned.
    ///
    /// `sprite_width` is the fixed pixel width shared by all sprites (28 for
    /// wall sprites). `palette` converts the 8-bit indexed pixels to RGBA.
    pub fn build(
        sprites: &[Option<HpfSprite>],
        palette: &Palette,
        sprite_width: u32,
    ) -> Result<Self, SpriteAtlasError> {
        Self::build_with_sprite_palette(sprites, sprite_width, |_| palette)
    }

    /// Build a sprite atlas from decoded HPF sprites, selecting the palette
    /// per sprite ID.
    pub fn build_with_sprite_palette<'a, F>(
        sprites: &[Option<HpfSprite>],
        sprite_width: u32,
        mut palette_for_sprite: F,
    ) -> Result<Self, SpriteAtlasError>
    where
        F: FnMut(u32) -> &'a Palette,
    {
        // Count how many actual sprites we have.
        let real_count = sprites.iter().filter(|s| s.is_some()).count();
        if real_count == 0 {
            return Err(SpriteAtlasError::NoSprites);
        }

        // Compute the total pixel height across all sprites so we can choose
        // a column count that keeps the atlas within MAX_ATLAS_SIDE.
        let total_height: u64 = sprites
            .iter()
            .filter_map(|s| s.as_ref())
            .map(|s| s.height as u64)
            .sum();

        // Minimum columns needed: total_height / max_height, with 10% headroom
        // for imperfect bin packing.
        let min_columns = ((total_height * 11 / 10 + MAX_ATLAS_SIDE as u64 - 1)
            / MAX_ATLAS_SIDE as u64)
            .max(1) as u32;
        // Also cap width at MAX_ATLAS_SIDE
        let max_columns = MAX_ATLAS_SIDE / sprite_width;
        let columns = min_columns.max(1).min(max_columns).min(real_count as u32);

        // First pass: assign each sprite to the shortest column (greedy).
        let mut column_heights = vec![0u32; columns as usize];
        let mut entries: Vec<Option<SpriteEntry>> = Vec::with_capacity(sprites.len());

        for sprite_opt in sprites {
            match sprite_opt {
                None => {
                    entries.push(None);
                }
                Some(sprite) => {
                    // Find the column with the least height.
                    let col = column_heights
                        .iter()
                        .enumerate()
                        .min_by_key(|&(_, &h)| h)
                        .map(|(i, _)| i)
                        .unwrap_or(0);

                    let x = col as u32 * sprite_width;
                    let y = column_heights[col];
                    let h = sprite.height as u32;

                    entries.push(Some(SpriteEntry {
                        x,
                        y,
                        width: sprite_width,
                        height: h,
                    }));

                    column_heights[col] += h;
                }
            }
        }

        let atlas_width = columns * sprite_width;
        let atlas_height = *column_heights.iter().max().unwrap_or(&0);

        if atlas_height == 0 {
            return Err(SpriteAtlasError::NoSprites);
        }

        // Second pass: blit sprite pixels into the RGBA atlas buffer.
        let mut pixels = vec![0u8; atlas_width as usize * atlas_height as usize * 4];

        for (i, sprite_opt) in sprites.iter().enumerate() {
            let sprite = match sprite_opt {
                Some(s) => s,
                None => continue,
            };
            let entry = match entries[i] {
                Some(e) => e,
                None => continue,
            };
            let palette = palette_for_sprite(i as u32);

            let sw = sprite.width as u32;
            let sh = sprite.height as u32;

            for py in 0..sh {
                for px in 0..sw.min(sprite_width) {
                    let src_idx = (py * sw + px) as usize;
                    if src_idx >= sprite.pixels.len() {
                        break;
                    }
                    let color_index = sprite.pixels[src_idx];
                    let rgba = palette.color(color_index);

                    let dst_x = entry.x + px;
                    let dst_y = entry.y + py;
                    let dst = ((dst_y * atlas_width + dst_x) * 4) as usize;

                    pixels[dst] = rgba[0];
                    pixels[dst + 1] = rgba[1];
                    pixels[dst + 2] = rgba[2];
                    pixels[dst + 3] = rgba[3];
                }
            }
        }

        Ok(Self {
            pixels,
            width: atlas_width,
            height: atlas_height,
            entries,
        })
    }

    /// Returns the UV rect `(u_min, v_min, u_max, v_max)` for a sprite,
    /// normalized to `[0, 1]`.
    ///
    /// Returns `None` if the index is out of range or the slot is empty.
    pub fn sprite_uv(&self, index: u32) -> Option<(f32, f32, f32, f32)> {
        let entry = self.entries.get(index as usize)?.as_ref()?;
        if entry.height == 0 {
            return None;
        }
        let inv_w = 1.0 / self.width as f32;
        let inv_h = 1.0 / self.height as f32;
        Some((
            entry.x as f32 * inv_w,
            entry.y as f32 * inv_h,
            (entry.x + entry.width) as f32 * inv_w,
            (entry.y + entry.height) as f32 * inv_h,
        ))
    }

    /// Returns the pixel rect `(x, y, width, height)` for a sprite in the atlas,
    /// or `None` if the index is out of range or the slot is empty.
    pub fn sprite_rect(&self, index: u32) -> Option<(u32, u32, u32, u32)> {
        let entry = self.entries.get(index as usize)?.as_ref()?;
        if entry.height == 0 {
            return None;
        }
        Some((entry.x, entry.y, entry.width, entry.height))
    }

    /// Returns the original pixel height of a sprite, or 0 if the slot is
    /// empty or out of range.
    pub fn sprite_height(&self, index: u32) -> u32 {
        self.entries
            .get(index as usize)
            .and_then(|e| e.as_ref())
            .map_or(0, |e| e.height)
    }

    /// Raw RGBA pixel data for the entire atlas.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Atlas dimensions in pixels `(width, height)`.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Number of sprite entries (including empty slots).
    pub fn sprite_count(&self) -> u32 {
        self.entries.len() as u32
    }
}
