use std::fmt;

use crate::Palette;

const ATLAS_COLUMNS: u32 = 64;

/// A rasterized RGBA tile atlas built from 8-bit indexed tile data and a palette.
///
/// Tiles are packed left-to-right, top-to-bottom into a single RGBA image
/// suitable for GPU texture upload.
pub struct TileAtlas {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    tile_width: u32,
    tile_height: u32,
    columns: u32,
    tile_count: u32,
}

impl TileAtlas {
    /// Builds an RGBA atlas from raw 8-bit indexed tile data.
    ///
    /// Each tile is `tile_w × tile_h` pixels stored row-major in `data`.
    /// The palette maps each byte to an RGBA color.
    pub fn from_raw(
        data: &[u8],
        palette: &Palette,
        tile_w: u32,
        tile_h: u32,
    ) -> Result<Self, AtlasError> {
        Self::from_raw_with_tile_palette(data, tile_w, tile_h, |_| palette)
    }

    /// Builds an RGBA atlas from raw 8-bit indexed tile data, selecting the
    /// palette per tile index.
    pub fn from_raw_with_tile_palette<'a, F>(
        data: &[u8],
        tile_w: u32,
        tile_h: u32,
        mut palette_for_tile: F,
    ) -> Result<Self, AtlasError>
    where
        F: FnMut(u32) -> &'a Palette,
    {
        let pixels_per_tile = (tile_w * tile_h) as usize;
        if pixels_per_tile == 0 {
            return Err(AtlasError::InvalidTileSize);
        }
        if data.len() % pixels_per_tile != 0 {
            return Err(AtlasError::DataNotAligned {
                data_len: data.len(),
                tile_size: pixels_per_tile,
            });
        }

        let tile_count = (data.len() / pixels_per_tile) as u32;
        if tile_count == 0 {
            return Err(AtlasError::NoTiles);
        }
        let columns = ATLAS_COLUMNS.min(tile_count);
        let rows = tile_count.div_ceil(columns);
        let width = columns * tile_w;
        let height = rows * tile_h;
        let mut pixels = vec![0u8; (width * height * 4) as usize];

        for tile_idx in 0..tile_count {
            let col = tile_idx % columns;
            let row = tile_idx / columns;
            let tile_data_offset = tile_idx as usize * pixels_per_tile;

            let atlas_x = col * tile_w;
            let atlas_y = row * tile_h;
            let palette = palette_for_tile(tile_idx);

            for py in 0..tile_h {
                for px in 0..tile_w {
                    let src = tile_data_offset + (py * tile_w + px) as usize;
                    let color_index = data[src];
                    let rgba = palette.color(color_index);

                    let dst_x = atlas_x + px;
                    let dst_y = atlas_y + py;
                    let dst = ((dst_y * width + dst_x) * 4) as usize;

                    pixels[dst] = rgba[0];
                    pixels[dst + 1] = rgba[1];
                    pixels[dst + 2] = rgba[2];
                    pixels[dst + 3] = rgba[3];
                }
            }
        }

        Ok(Self {
            pixels,
            width,
            height,
            tile_width: tile_w,
            tile_height: tile_h,
            columns,
            tile_count,
        })
    }

    /// Returns the pixel rect `(x, y, w, h)` of a tile in the atlas.
    pub fn tile_rect(&self, index: u32) -> Option<(u32, u32, u32, u32)> {
        if index >= self.tile_count {
            return None;
        }
        let col = index % self.columns;
        let row = index / self.columns;
        Some((
            col * self.tile_width,
            row * self.tile_height,
            self.tile_width,
            self.tile_height,
        ))
    }

    /// Returns the UV rect `(u_min, v_min, u_max, v_max)` for a tile, normalized to `[0, 1]`.
    pub fn tile_uv(&self, index: u32) -> Option<(f32, f32, f32, f32)> {
        let (x, y, w, h) = self.tile_rect(index)?;
        let inv_w = 1.0 / self.width as f32;
        let inv_h = 1.0 / self.height as f32;
        Some((
            x as f32 * inv_w,
            y as f32 * inv_h,
            (x + w) as f32 * inv_w,
            (y + h) as f32 * inv_h,
        ))
    }

    /// Raw RGBA pixel data for the entire atlas.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Atlas dimensions in pixels `(width, height)`.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Number of tiles in the atlas.
    pub fn tile_count(&self) -> u32 {
        self.tile_count
    }

    /// Tiles per row in the atlas.
    pub fn columns(&self) -> u32 {
        self.columns
    }

    /// Tile dimensions `(width, height)`.
    pub fn tile_size(&self) -> (u32, u32) {
        (self.tile_width, self.tile_height)
    }
}

#[derive(Debug)]
pub enum AtlasError {
    InvalidTileSize,
    NoTiles,
    DataNotAligned { data_len: usize, tile_size: usize },
}

impl fmt::Display for AtlasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AtlasError::InvalidTileSize => write!(f, "tile dimensions must be non-zero"),
            AtlasError::NoTiles => write!(f, "no tiles provided for atlas"),
            AtlasError::DataNotAligned {
                data_len,
                tile_size,
            } => write!(
                f,
                "data length {data_len} is not a multiple of tile size {tile_size}"
            ),
        }
    }
}

impl std::error::Error for AtlasError {}
