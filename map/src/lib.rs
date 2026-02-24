use std::io;
use std::path::Path;

pub const TILE_WIDTH: f32 = 56.0;
pub const TILE_HEIGHT: f32 = 27.0;
pub const TILE_BYTES: usize = 6;

#[derive(Clone, Copy, Default)]
pub struct Tile {
    pub ground: u16,
    pub left_wall: u16,
    pub right_wall: u16,
}

pub struct Map {
    pub width: u16,
    pub height: u16,
    pub tiles: Vec<Tile>,
}

impl Map {
    pub fn new(width: u16, height: u16) -> Self {
        let count = width as usize * height as usize;
        Self {
            width,
            height,
            tiles: vec![Tile::default(); count],
        }
    }

    /// Parse a map from raw binary tile data. Dimensions are guessed
    /// to be the most square factorization with width >= height.
    pub fn from_bytes(data: &[u8]) -> io::Result<Self> {
        if data.len() % TILE_BYTES != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "map data length {} is not a multiple of {}",
                    data.len(),
                    TILE_BYTES
                ),
            ));
        }

        let tile_count = data.len() / TILE_BYTES;
        let (width, height) = guess_dimensions(tile_count);

        let mut tiles = Vec::with_capacity(tile_count);
        for i in 0..tile_count {
            let base = i * TILE_BYTES;
            let ground = u16::from_le_bytes([data[base], data[base + 1]]);
            let left_wall = u16::from_le_bytes([data[base + 2], data[base + 3]]);
            let right_wall = u16::from_le_bytes([data[base + 4], data[base + 5]]);
            tiles.push(Tile {
                ground,
                left_wall,
                right_wall,
            });
        }

        Ok(Self {
            width,
            height,
            tiles,
        })
    }

    /// Serialize the map back to raw binary tile data.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(self.tiles.len() * TILE_BYTES);
        for tile in &self.tiles {
            data.extend_from_slice(&tile.ground.to_le_bytes());
            data.extend_from_slice(&tile.left_wall.to_le_bytes());
            data.extend_from_slice(&tile.right_wall.to_le_bytes());
        }
        data
    }

    /// Load a map from a `.map` file.
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let data = std::fs::read(path)?;
        Self::from_bytes(&data)
    }

    /// Save the map to a `.map` file.
    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        std::fs::write(path, self.to_bytes())
    }

    /// Returns the tile at (col, row), or None if out of bounds.
    pub fn get(&self, col: u16, row: u16) -> Option<&Tile> {
        if col < self.width && row < self.height {
            Some(&self.tiles[row as usize * self.width as usize + col as usize])
        } else {
            None
        }
    }
}

/// Returns all valid (width, height) factor pairs for `n` tiles,
/// where width >= height. Sorted most-square first (ascending width).
pub fn all_dimensions(n: usize) -> Vec<(u16, u16)> {
    if n == 0 {
        return vec![(0, 0)];
    }
    let mut pairs = Vec::new();
    let mut d = 1usize;
    while d * d <= n {
        if n % d == 0 {
            let w = (n / d) as u16;
            let h = d as u16;
            pairs.push((w, h)); // w >= h since d <= sqrt(n)
        }
        d += 1;
    }
    pairs.reverse(); // most-square first
    pairs
}

/// Find the most square factorization of `n` where width >= height.
/// If no perfect factors exist, returns (n, 1).
pub fn guess_dimensions(n: usize) -> (u16, u16) {
    if n == 0 {
        return (0, 0);
    }

    let sqrt = (n as f64).sqrt() as usize;

    // Try from sqrt upward to find the smallest width >= sqrt
    for w in sqrt..=n {
        if n % w == 0 {
            let h = n / w;
            return (w as u16, h as u16);
        }
    }

    (n as u16, 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guess_dimensions_perfect_square() {
        assert_eq!(guess_dimensions(2500), (50, 50));
        assert_eq!(guess_dimensions(10000), (100, 100));
        assert_eq!(guess_dimensions(400), (20, 20));
    }

    #[test]
    fn test_guess_dimensions_rectangle() {
        // 2250 = 50*45, which is closer to square than 75*30 etc.
        assert_eq!(guess_dimensions(2250), (50, 45));
        // 75 = 15*5
        assert_eq!(guess_dimensions(75), (15, 5));
    }

    #[test]
    fn test_all_dimensions() {
        // 2500 = 50*50, many factors
        let dims = all_dimensions(2500);
        assert_eq!(dims[0], (50, 50)); // most square first
        assert!(dims.contains(&(100, 25)));
        assert!(dims.contains(&(2500, 1)));
        // all pairs valid
        for &(w, h) in &dims {
            assert_eq!(w as usize * h as usize, 2500);
            assert!(w >= h);
        }
    }

    #[test]
    fn test_all_dimensions_prime() {
        // 7 is prime → only (7, 1)
        assert_eq!(all_dimensions(7), vec![(7, 1)]);
    }

    #[test]
    fn test_roundtrip() {
        let mut map = Map::new(10, 8);
        map.tiles[0].ground = 42;
        map.tiles[79].right_wall = 99;
        let data = map.to_bytes();
        assert_eq!(data.len(), 10 * 8 * 6);
        let loaded = Map::from_bytes(&data).unwrap();
        assert_eq!(loaded.tiles[0].ground, 42);
        assert_eq!(loaded.tiles[79].right_wall, 99);
    }
}
