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
/// `bg_color` is an optional RGBA background fill; `None` means transparent.
pub fn export_map_png(
    path: &Path,
    map: &map::Map,
    tile_atlas: &render::TileAtlas,
    wall_atlas: &render::SpriteAtlas,
    zoom: f32,
    bg_color: Option<[u8; 4]>,
) -> io::Result<()> {
    let (out_w, out_h) = compute_output_dimensions(map, zoom, Some(wall_atlas));
    let mut buffer = vec![0u8; out_w as usize * out_h as usize * 4];

    // Fill background if specified
    if let Some(bg) = bg_color {
        for pixel in buffer.chunks_exact_mut(4) {
            pixel.copy_from_slice(&bg);
        }
    }

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

/// Derive the tab map output path from the main export path.
/// e.g. "foo.png" → "foo_tab.png"
pub fn tab_map_path(main_path: &Path) -> std::path::PathBuf {
    let stem = main_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("export");
    let ext = main_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("png");
    main_path.with_file_name(format!("{}_tab.{}", stem, ext))
}

// --- Tab Map export ---

/// Base tile dimensions for the tab map at 100% scale.
const TAB_MAP_TILE_W: f32 = 32.0;
const TAB_MAP_TILE_H: f32 = 16.0;

/// Compute output dimensions for a tab map PNG export.
pub fn compute_tab_map_dimensions(map: &map::Map, zoom: f32) -> (u32, u32) {
    let hw = (TAB_MAP_TILE_W / 2.0) * zoom;
    let hh = (TAB_MAP_TILE_H / 2.0) * zoom;
    // +1 ensures the outermost boundary pixels (bottom/right vertices) aren't clipped
    let w = ((map.width as f32 + map.height as f32) * hw).ceil() as u32 + 1;
    let h = ((map.width as f32 + map.height as f32) * hh).ceil() as u32 + 1;
    (w.max(1), h.max(1))
}

/// Build a flat bool grid indicating whether each tile is solid (impassable).
fn build_solid_grid(map: &map::Map, sotp: &[u8]) -> Vec<bool> {
    map.tiles
        .iter()
        .map(|tile| {
            let left_solid = tile.left_wall > 0
                && sotp
                    .get((tile.left_wall - 1) as usize)
                    .map(|&b| b & 0xF == 0xF)
                    .unwrap_or(false);
            let right_solid = tile.right_wall > 0
                && sotp
                    .get((tile.right_wall - 1) as usize)
                    .map(|&b| b & 0xF == 0xF)
                    .unwrap_or(false);
            left_solid || right_solid
        })
        .collect()
}

/// Renders a tab map (collision wireframe) to a PNG file.
///
/// Solid tiles are filled with a diagonal hatch pattern and boundary wireframe edges.
/// `zoom` is a multiplier on the base 32x16 tile size.
/// `bg_color` is an optional RGBA background fill; `None` means transparent.
pub fn export_tab_map_png(
    path: &Path,
    map: &map::Map,
    sotp: &[u8],
    zoom: f32,
    bg_color: Option<[u8; 4]>,
) -> io::Result<()> {
    let (out_w, out_h) = compute_tab_map_dimensions(map, zoom);
    let mut buffer = vec![0u8; out_w as usize * out_h as usize * 4];

    // Fill background if specified
    if let Some(bg) = bg_color {
        for pixel in buffer.chunks_exact_mut(4) {
            pixel.copy_from_slice(&bg);
        }
    }

    let w = map.width as usize;
    let h = map.height as usize;
    let solid = build_solid_grid(map, sotp);

    let hw = (TAB_MAP_TILE_W / 2.0) * zoom;
    let hh = (TAB_MAP_TILE_H / 2.0) * zoom;

    let origin_x = h as f32 * hw;
    let origin_y = 0.0;

    // Hatch line spacing scales with zoom so it doesn't get too dense or sparse
    let hatch_spacing = (4.0 * zoom).round().max(2.0) as u32;

    // Pre-composite the hatch color onto the background so hatch_diamond can
    // write pixels directly (avoiding alpha accumulation where neighbor
    // diamonds overlap at shared boundary pixels).
    let hatch_color = match bg_color {
        Some(bg) => {
            let sa = 50u32;
            let inv = 255 - sa;
            [
                ((255 * sa + bg[0] as u32 * inv) / 255) as u8,
                ((255 * sa + bg[1] as u32 * inv) / 255) as u8,
                ((255 * sa + bg[2] as u32 * inv) / 255) as u8,
                255,
            ]
        }
        None => [255, 255, 255, 50],
    };

    let is_solid = |col: i32, row: i32| -> bool {
        if col < 0 || col >= w as i32 || row < 0 || row >= h as i32 {
            return false;
        }
        solid[row as usize * w + col as usize]
    };

    // Two-pass rendering: hatch fill first, then edges.
    // This prevents a later tile's hatch from overwriting an earlier tile's edge pixel.

    // Pass 1: hatch fills
    for row in 0..h {
        for col in 0..w {
            if !solid[row * w + col] {
                continue;
            }
            let cx = origin_x + (col as f32 - row as f32) * hw;
            let cy = origin_y + (col as f32 + row as f32) * hh;
            hatch_diamond(
                &mut buffer,
                out_w,
                out_h,
                cx,
                cy,
                hw,
                hh,
                hatch_color,
                hatch_spacing,
            );
        }
    }

    // Pass 2: boundary edges (drawn on top of all hatch)
    let edge_color = [255, 255, 255, 180];
    for row in 0..h {
        for col in 0..w {
            if !solid[row * w + col] {
                continue;
            }
            let cx = origin_x + (col as f32 - row as f32) * hw;
            let cy = origin_y + (col as f32 + row as f32) * hh;
            let c = col as i32;
            let r = row as i32;

            if !is_solid(c, r - 1) {
                draw_line(&mut buffer, out_w, out_h, cx, cy, cx + hw, cy + hh, edge_color);
            }
            if !is_solid(c + 1, r) {
                draw_line(&mut buffer, out_w, out_h, cx + hw, cy + hh, cx, cy + 2.0 * hh, edge_color);
            }
            if !is_solid(c, r + 1) {
                draw_line(&mut buffer, out_w, out_h, cx, cy + 2.0 * hh, cx - hw, cy + hh, edge_color);
            }
            if !is_solid(c - 1, r) {
                draw_line(&mut buffer, out_w, out_h, cx - hw, cy + hh, cx, cy, edge_color);
            }
        }
    }

    write_png(path, &buffer, out_w, out_h)
}

/// Fill an isometric diamond with a diagonal line hatch pattern.
/// Lines run at 45 degrees (top-left to bottom-right) spaced `spacing` pixels apart.
///
/// Pixels are written directly (no alpha compositing) so that overlapping
/// neighbor diamonds don't cause brightness accumulation artifacts.
/// The caller should pre-composite the color onto any background.
#[allow(clippy::too_many_arguments)]
fn hatch_diamond(
    buf: &mut [u8],
    buf_w: u32,
    buf_h: u32,
    cx: f32,
    cy: f32,
    hw: f32,
    hh: f32,
    color: [u8; 4],
    spacing: u32,
) {
    let top_y = cy.round() as i32;
    let bot_y = (cy + 2.0 * hh).round() as i32;

    for py in top_y..=bot_y {
        if py < 0 || py >= buf_h as i32 {
            continue;
        }
        // Compute horizontal extent at this scanline
        let fy = py as f32 + 0.5;
        let t = if fy <= cy + hh {
            (fy - cy) / hh
        } else {
            (cy + 2.0 * hh - fy) / hh
        };
        if t < 0.0 {
            continue;
        }
        let half_span = hw * t.min(1.0);
        let left_x = (cx - half_span).round() as i32;
        let right_x = (cx + half_span).round() as i32;

        for px in left_x..=right_x {
            if px < 0 || px >= buf_w as i32 {
                continue;
            }
            // Diagonal hatch: draw pixel only on lines where (px + py) mod spacing == 0
            if ((px + py) as u32) % spacing != 0 {
                continue;
            }
            let idx = (py as usize * buf_w as usize + px as usize) * 4;
            // Direct write — prevents alpha accumulation from overlapping neighbor diamonds
            buf[idx] = color[0];
            buf[idx + 1] = color[1];
            buf[idx + 2] = color[2];
            buf[idx + 3] = color[3];
        }
    }
}

/// Draw an anti-aliased line using Bresenham's algorithm with alpha blending.
#[allow(clippy::too_many_arguments)]
fn draw_line(
    buf: &mut [u8],
    buf_w: u32,
    buf_h: u32,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    color: [u8; 4],
) {
    // Bresenham's line (integer coords)
    let mut ix0 = x0.round() as i32;
    let mut iy0 = y0.round() as i32;
    let ix1 = x1.round() as i32;
    let iy1 = y1.round() as i32;

    let dx = (ix1 - ix0).abs();
    let dy = -(iy1 - iy0).abs();
    let sx = if ix0 < ix1 { 1 } else { -1 };
    let sy = if iy0 < iy1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if ix0 >= 0 && ix0 < buf_w as i32 && iy0 >= 0 && iy0 < buf_h as i32 {
            let idx = (iy0 as usize * buf_w as usize + ix0 as usize) * 4;
            let sa = color[3] as u32;
            if sa == 255 {
                buf[idx] = color[0];
                buf[idx + 1] = color[1];
                buf[idx + 2] = color[2];
                buf[idx + 3] = 255;
            } else if sa > 0 {
                let inv = 255 - sa;
                buf[idx] = ((color[0] as u32 * sa + buf[idx] as u32 * inv) / 255) as u8;
                buf[idx + 1] = ((color[1] as u32 * sa + buf[idx + 1] as u32 * inv) / 255) as u8;
                buf[idx + 2] = ((color[2] as u32 * sa + buf[idx + 2] as u32 * inv) / 255) as u8;
                buf[idx + 3] = (sa + buf[idx + 3] as u32 * inv / 255) as u8;
            }
        }

        if ix0 == ix1 && iy0 == iy1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            ix0 += sx;
        }
        if e2 <= dx {
            err += dx;
            iy0 += sy;
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
