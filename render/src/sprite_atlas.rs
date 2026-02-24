use std::fmt;

use crate::hpf::HpfSprite;
use crate::Palette;

const ATLAS_COLUMNS: u32 = 64;

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
/// Sprites are packed into columns of fixed width. Since all wall sprites
/// share the same pixel width (28), they are stacked vertically within each
/// column, avoiding the wasted space that would come from padding every sprite
/// to the tallest height.
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
    /// `sprites` is indexed by sprite ID (0-based). `None` entries represent
    /// empty/missing sprite slots and are preserved so that lookups by tile ID
    /// remain aligned (tile `left_wall = N` maps to `sprites[N - 1]`).
    ///
    /// `sprite_width` is the fixed pixel width shared by all sprites (28 for
    /// wall sprites). `palette` converts the 8-bit indexed pixels to RGBA.
    pub fn build(
        sprites: &[Option<HpfSprite>],
        palette: &Palette,
        sprite_width: u32,
    ) -> Result<Self, SpriteAtlasError> {
        // Count how many actual sprites we have.
        let real_count = sprites.iter().filter(|s| s.is_some()).count();
        if real_count == 0 {
            return Err(SpriteAtlasError::NoSprites);
        }

        let columns = ATLAS_COLUMNS.min(real_count as u32);

        // First pass: assign each sprite to a column and compute column heights.
        let mut column_heights = vec![0u32; columns as usize];
        let mut entries: Vec<Option<SpriteEntry>> = Vec::with_capacity(sprites.len());
        let mut col_cursor = 0u32; // round-robin column assignment

        for sprite_opt in sprites {
            match sprite_opt {
                None => {
                    entries.push(None);
                }
                Some(sprite) => {
                    let col = col_cursor % columns;
                    let x = col * sprite_width;
                    let y = column_heights[col as usize];
                    let h = sprite.height as u32;

                    entries.push(Some(SpriteEntry {
                        x,
                        y,
                        width: sprite_width,
                        height: h,
                    }));

                    column_heights[col as usize] += h;
                    col_cursor += 1;
                }
            }
        }

        let atlas_width = columns * sprite_width;
        let atlas_height = *column_heights.iter().max().unwrap_or(&0);

        if atlas_height == 0 {
            return Err(SpriteAtlasError::NoSprites);
        }

        // Second pass: blit sprite pixels into the RGBA atlas buffer.
        let mut pixels = vec![0u8; (atlas_width * atlas_height * 4) as usize];

        for (i, sprite_opt) in sprites.iter().enumerate() {
            let sprite = match sprite_opt {
                Some(s) => s,
                None => continue,
            };
            let entry = match entries[i] {
                Some(e) => e,
                None => continue,
            };

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
