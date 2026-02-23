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
}
