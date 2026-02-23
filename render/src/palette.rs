use std::fmt;

const PALETTE_SIZE: usize = 768;
const COLOR_COUNT: usize = 256;

/// A 256-color RGB palette parsed from a 768-byte `.pal` file.
///
/// Index 0 is treated as transparent when converting to RGBA.
pub struct Palette {
    colors: [[u8; 3]; COLOR_COUNT],
}

impl Palette {
    /// Parses a 768-byte palette buffer (256 × RGB).
    pub fn from_bytes(data: &[u8]) -> Result<Self, PaletteError> {
        if data.len() != PALETTE_SIZE {
            return Err(PaletteError::InvalidSize(data.len()));
        }

        let mut colors = [[0u8; 3]; COLOR_COUNT];
        for i in 0..COLOR_COUNT {
            let base = i * 3;
            colors[i] = [data[base], data[base + 1], data[base + 2]];
        }

        Ok(Self { colors })
    }

    /// Returns the RGBA color for a palette index.
    ///
    /// Index 0 is fully transparent `[0, 0, 0, 0]`.
    /// All other indices are fully opaque.
    pub fn color(&self, index: u8) -> [u8; 4] {
        if index == 0 {
            [0, 0, 0, 0]
        } else {
            let [r, g, b] = self.colors[index as usize];
            [r, g, b, 255]
        }
    }

    /// Returns the raw RGB triplet for a palette index.
    pub fn rgb(&self, index: u8) -> [u8; 3] {
        self.colors[index as usize]
    }
}

#[derive(Debug)]
pub enum PaletteError {
    InvalidSize(usize),
}

impl fmt::Display for PaletteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaletteError::InvalidSize(n) => {
                write!(f, "expected {PALETTE_SIZE} bytes for palette, got {n}")
            }
        }
    }
}

impl std::error::Error for PaletteError {}
