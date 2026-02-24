use std::io;
use std::path::Path;

/// Returns true if a wall tile ID should be rendered.
fn is_rendered_wall(id: u16) -> bool {
    if id == 0 {
        return false;
    }
    (id > 10012) || ((id % 10000) > 12)
}

/// Find the tallest wall sprite actually referenced by the map.
fn max_wall_height_in_map(map: &map::Map, wall_atlas: &render::SpriteAtlas) -> u32 {
    let mut max_h = 0u32;
    for tile in &map.tiles {
        if is_rendered_wall(tile.left_wall) {
            max_h = max_h.max(wall_atlas.sprite_height(tile.left_wall as u32));
        }
        if is_rendered_wall(tile.right_wall) {
            max_h = max_h.max(wall_atlas.sprite_height(tile.right_wall as u32));
        }
    }
    max_h
}

/// Computes output image dimensions for a given map, zoom factor, and wall atlas.
///
/// The height accounts for the tallest wall sprite actually used in the map,
/// so there is no wasted headroom.
///
/// Returns `(width_px, height_px)`.
pub fn compute_output_dimensions(
    map: &map::Map,
    zoom: f32,
    wall_atlas: Option<&render::SpriteAtlas>,
) -> (u32, u32) {
    let half_w = (map::TILE_WIDTH / 2.0) * zoom;
    let half_h = (map::TILE_HEIGHT / 2.0) * zoom;
    let img_w = ((map.width as f32 + map.height as f32) * half_w).ceil() as u32;
    let base_h = ((map.width as f32 + map.height as f32) * half_h).ceil() as u32;
    // Headroom = tallest wall sprite minus the ground tile height (walls extend
    // upward from the bottom of a tile, ground tile height is already included).
    let max_wall_px = wall_atlas
        .map(|wa| max_wall_height_in_map(map, wa))
        .unwrap_or(0);
    let tile_h = map::TILE_HEIGHT * zoom;
    let wall_headroom = ((max_wall_px as f32 * zoom) - tile_h).max(0.0).ceil() as u32;
    let img_h = base_h + wall_headroom;
    (img_w.max(1), img_h.max(1))
}

/// Renders a map to a PNG file at the given zoom factor.
///
/// `zoom` is a multiplier (1.0 = native resolution, 2.0 = 2x, etc.).
pub fn export_map_png(
    path: &Path,
    map: &map::Map,
    tile_atlas: &render::TileAtlas,
    wall_atlas: &render::SpriteAtlas,
    zoom: f32,
) -> io::Result<()> {
    let (out_w, out_h) = compute_output_dimensions(map, zoom, Some(wall_atlas));
    let mut buffer = vec![0u8; out_w as usize * out_h as usize * 4];

    let half_w = (map::TILE_WIDTH / 2.0) * zoom;
    let half_h = (map::TILE_HEIGHT / 2.0) * zoom;

    // Headroom above the topmost tile for tall walls
    let max_wall_px = max_wall_height_in_map(map, wall_atlas);
    let tile_h = map::TILE_HEIGHT * zoom;
    let wall_headroom = ((max_wall_px as f32 * zoom) - tile_h).max(0.0).ceil();

    // Origin: leftmost point of the isometric diamond
    let origin_x = map.height as f32 * half_w;
    let origin_y = wall_headroom;

    let tile_pixels = tile_atlas.pixels();
    let (tile_atlas_w, _) = tile_atlas.dimensions();
    let wall_pixels = wall_atlas.pixels();
    let (wall_atlas_w, _) = wall_atlas.dimensions();

    let max_depth = map.width as u16 + map.height as u16;

    for depth in 0..max_depth {
        let row_min = depth.saturating_sub(map.width - 1);
        let row_max = depth.min(map.height - 1);

        for row in row_min..=row_max {
            let col = depth - row;
            let tile = &map.tiles[row as usize * map.width as usize + col as usize];

            let cx = origin_x + (col as f32 - row as f32) * half_w;
            let cy = origin_y + (col as f32 + row as f32) * half_h;

            // Ground tile
            if tile.ground != 0 {
                let atlas_index = (tile.ground - 1) as u32;
                if let Some((sx, sy, sw, sh)) = tile_atlas.tile_rect(atlas_index) {
                    let dst_x = (cx - half_w).round() as i32;
                    let dst_y = cy.round() as i32;
                    let dst_w = (half_w * 2.0).round() as u32;
                    let dst_h = (half_h * 2.0).round() as u32;
                    blit_scaled(
                        &mut buffer,
                        out_w,
                        out_h,
                        dst_x,
                        dst_y,
                        dst_w,
                        dst_h,
                        tile_pixels,
                        tile_atlas_w,
                        sx,
                        sy,
                        sw,
                        sh,
                    );
                }
            }

            let bottom_y = cy + 2.0 * half_h;

            // Left wall
            if is_rendered_wall(tile.left_wall) {
                let idx = tile.left_wall as u32;
                if let Some((sx, sy, sw, sh)) = wall_atlas.sprite_rect(idx) {
                    let screen_h = (sh as f32 * zoom).round() as u32;
                    let dst_x = (cx - half_w).round() as i32;
                    let dst_y = (bottom_y - screen_h as f32).round() as i32;
                    let dst_w = half_w.round() as u32;
                    blit_scaled(
                        &mut buffer,
                        out_w,
                        out_h,
                        dst_x,
                        dst_y,
                        dst_w,
                        screen_h,
                        wall_pixels,
                        wall_atlas_w,
                        sx,
                        sy,
                        sw,
                        sh,
                    );
                }
            }

            // Right wall
            if is_rendered_wall(tile.right_wall) {
                let idx = tile.right_wall as u32;
                if let Some((sx, sy, sw, sh)) = wall_atlas.sprite_rect(idx) {
                    let screen_h = (sh as f32 * zoom).round() as u32;
                    let dst_x = cx.round() as i32;
                    let dst_y = (bottom_y - screen_h as f32).round() as i32;
                    let dst_w = half_w.round() as u32;
                    blit_scaled(
                        &mut buffer,
                        out_w,
                        out_h,
                        dst_x,
                        dst_y,
                        dst_w,
                        screen_h,
                        wall_pixels,
                        wall_atlas_w,
                        sx,
                        sy,
                        sw,
                        sh,
                    );
                }
            }
        }
    }

    write_png(path, &buffer, out_w, out_h)
}

/// Blit a source rect from an RGBA atlas into the destination buffer with scaling.
/// Uses nearest-neighbor sampling and source-over alpha compositing.
#[allow(clippy::too_many_arguments)]
fn blit_scaled(
    dst_buf: &mut [u8],
    dst_buf_w: u32,
    dst_buf_h: u32,
    dst_x: i32,
    dst_y: i32,
    dst_w: u32,
    dst_h: u32,
    src_buf: &[u8],
    src_buf_w: u32,
    src_x: u32,
    src_y: u32,
    src_w: u32,
    src_h: u32,
) {
    if dst_w == 0 || dst_h == 0 || src_w == 0 || src_h == 0 {
        return;
    }

    for dy in 0..dst_h {
        let py = dst_y + dy as i32;
        if py < 0 || py >= dst_buf_h as i32 {
            continue;
        }

        // Nearest-neighbor: map destination row to source row
        let sy_local = ((dy as f32 / dst_h as f32) * src_h as f32) as u32;
        if sy_local >= src_h {
            continue;
        }

        for dx in 0..dst_w {
            let px = dst_x + dx as i32;
            if px < 0 || px >= dst_buf_w as i32 {
                continue;
            }

            // Nearest-neighbor: map destination col to source col
            let sx_local = ((dx as f32 / dst_w as f32) * src_w as f32) as u32;
            if sx_local >= src_w {
                continue;
            }

            let src_idx =
                ((src_y + sy_local) as usize * src_buf_w as usize + (src_x + sx_local) as usize)
                    * 4;
            if src_idx + 3 >= src_buf.len() {
                continue;
            }

            let sa = src_buf[src_idx + 3] as u32;
            if sa == 0 {
                continue;
            }

            let dst_idx = (py as usize * dst_buf_w as usize + px as usize) * 4;

            if sa == 255 {
                dst_buf[dst_idx] = src_buf[src_idx];
                dst_buf[dst_idx + 1] = src_buf[src_idx + 1];
                dst_buf[dst_idx + 2] = src_buf[src_idx + 2];
                dst_buf[dst_idx + 3] = 255;
            } else {
                let inv_sa = 255 - sa;
                dst_buf[dst_idx] =
                    ((src_buf[src_idx] as u32 * sa + dst_buf[dst_idx] as u32 * inv_sa) / 255)
                        as u8;
                dst_buf[dst_idx + 1] = ((src_buf[src_idx + 1] as u32 * sa
                    + dst_buf[dst_idx + 1] as u32 * inv_sa)
                    / 255) as u8;
                dst_buf[dst_idx + 2] = ((src_buf[src_idx + 2] as u32 * sa
                    + dst_buf[dst_idx + 2] as u32 * inv_sa)
                    / 255) as u8;
                dst_buf[dst_idx + 3] =
                    (sa + dst_buf[dst_idx + 3] as u32 * inv_sa / 255) as u8;
            }
        }
    }
}

/// Write an RGBA buffer as a PNG file.
fn write_png(path: &Path, rgba: &[u8], width: u32, height: u32) -> io::Result<()> {
    let file = std::fs::File::create(path)?;
    let w = io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    writer
        .write_image_data(rgba)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    Ok(())
}
