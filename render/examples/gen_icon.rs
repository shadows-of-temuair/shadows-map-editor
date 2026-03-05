/// Generates the map editor app icon: isometric 3x3 grid in orange
/// on a rounded square with charcoal gradient and orange glow.

fn main() {
    let size = 512u32;
    let center = size as f32 / 2.0;
    let mut pixels = vec![0u8; (size * size * 4) as usize];

    let border_color = [70u8, 75, 82];
    let accent = [224.0f32, 138.0, 53.0];

    let margin = 8.0;
    let corner_radius = 64.0;
    let border_width = 8.0;

    fn sdf_rounded_rect(
        px: f32,
        py: f32,
        cx: f32,
        cy: f32,
        half_w: f32,
        half_h: f32,
        r: f32,
    ) -> f32 {
        let dx = (px - cx).abs() - half_w + r;
        let dy = (py - cy).abs() - half_h + r;
        let outside = (dx.max(0.0) * dx.max(0.0) + dy.max(0.0) * dy.max(0.0)).sqrt();
        let inside = dx.max(dy).min(0.0);
        outside + inside - r
    }

    let half_ext = center - margin;
    let inner_half = half_ext - border_width;
    let inner_r = corner_radius - border_width * 0.5;

    // --- Pass 1: Background with charcoal gradient + bevel ---
    for y in 0..size {
        for x in 0..size {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let idx = ((y * size + x) * 4) as usize;

            let d_outer =
                sdf_rounded_rect(px, py, center, center, half_ext, half_ext, corner_radius);
            let d_inner = sdf_rounded_rect(px, py, center, center, inner_half, inner_half, inner_r);

            if d_outer > 1.0 {
                continue;
            }

            let outer_alpha = (-d_outer).clamp(0.0, 1.0);

            if d_inner < -0.5 {
                // Interior — charcoal gradient (lighter at top, darker at bottom)
                let t = (py - margin) / (size as f32 - margin * 2.0); // 0 at top, 1 at bottom
                let top_val = 28.0f32;
                let bot_val = 12.0f32;
                let base = top_val + (bot_val - top_val) * t;

                // Subtle bevel: lighten near top edge, darken near bottom edge
                let dist_from_inner = -d_inner;
                let bevel_zone = 18.0;
                let bevel = if dist_from_inner < bevel_zone {
                    let bevel_t = dist_from_inner / bevel_zone;
                    // top = lighten, bottom = darken based on vertical position relative to center
                    let vert_bias = (center - py) / (inner_half); // positive = above center
                    vert_bias * (1.0 - bevel_t) * 12.0
                } else {
                    0.0
                };

                let v = (base + bevel).clamp(6.0, 45.0) as u8;
                pixels[idx] = v;
                pixels[idx + 1] = (v as f32 * 1.05) as u8;
                pixels[idx + 2] = (v as f32 * 1.12) as u8;
                pixels[idx + 3] = (outer_alpha * 255.0) as u8;
            } else {
                // Border
                let border_alpha = (0.5 - d_inner).clamp(0.0, 1.0);
                // Bevel on border: lighter top edge, darker bottom
                let vert_t = (py - margin) / (size as f32 - margin * 2.0);
                let bevel_mult = 1.0 + (0.5 - vert_t) * 0.5; // >1 at top, <1 at bottom
                let r = ((border_color[0] as f32 * bevel_mult) * border_alpha
                    + 14.0 * (1.0 - border_alpha))
                    .min(255.0) as u8;
                let g = ((border_color[1] as f32 * bevel_mult) * border_alpha
                    + 15.0 * (1.0 - border_alpha))
                    .min(255.0) as u8;
                let b = ((border_color[2] as f32 * bevel_mult) * border_alpha
                    + 17.0 * (1.0 - border_alpha))
                    .min(255.0) as u8;
                pixels[idx] = r;
                pixels[idx + 1] = g;
                pixels[idx + 2] = b;
                pixels[idx + 3] = (outer_alpha * 255.0) as u8;
            }
        }
    }

    // --- Collect grid geometry ---
    let grid_h = (inner_half - 20.0) * 0.52;
    let grid_w = grid_h * 2.0;

    let top = (center, center - grid_h);
    let right = (center + grid_w, center);
    let bottom = (center, center + grid_h);
    let left = (center - grid_w, center);

    fn lerp(a: (f32, f32), b: (f32, f32), t: f32) -> (f32, f32) {
        (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
    }

    let stroke_w = 10.0;
    let thin_stroke = 5.5;

    let mut lines: Vec<((f32, f32), (f32, f32), f32)> = Vec::new();
    lines.push((top, right, stroke_w));
    lines.push((right, bottom, stroke_w));
    lines.push((bottom, left, stroke_w));
    lines.push((left, top, stroke_w));
    for i in 1..3 {
        let t = i as f32 / 3.0;
        lines.push((lerp(left, top, t), lerp(bottom, right, t), thin_stroke));
        lines.push((lerp(top, right, t), lerp(left, bottom, t), thin_stroke));
    }

    // Precompute a distance field for the grid lines (for the glow pass)
    // Using a coarse grid is fine for the glow radius we need
    let glow_radius = 40.0f32;

    // Helper: min distance from a point to any grid line segment
    let dist_to_grid = |px: f32, py: f32| -> f32 {
        let mut min_d = f32::MAX;
        for (p0, p1, _) in &lines {
            let dx = p1.0 - p0.0;
            let dy = p1.1 - p0.1;
            let len_sq = dx * dx + dy * dy;
            if len_sq < 0.001 {
                continue;
            }
            let vx = px - p0.0;
            let vy = py - p0.1;
            let t = ((vx * dx + vy * dy) / len_sq).clamp(0.0, 1.0);
            let cx = p0.0 + t * dx;
            let cy = p0.1 + t * dy;
            let d = ((px - cx) * (px - cx) + (py - cy) * (py - cy)).sqrt();
            if d < min_d {
                min_d = d;
            }
        }
        min_d
    };

    // --- Pass 2: Orange glow behind the grid ---
    for y in 0..size {
        for x in 0..size {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let idx = ((y * size + x) * 4) as usize;

            // Only apply glow inside the shape
            if pixels[idx + 3] == 0 {
                continue;
            }

            let d_inner = sdf_rounded_rect(px, py, center, center, inner_half, inner_half, inner_r);
            if d_inner > -0.5 {
                continue;
            }

            let grid_dist = dist_to_grid(px, py);
            if grid_dist >= glow_radius {
                continue;
            }

            // Glow falloff — stronger near the lines, fading outward
            let glow_t = 1.0 - (grid_dist / glow_radius);
            let glow_intensity = glow_t * glow_t * 0.22; // quadratic falloff, max ~22% blend

            // Blend glow color onto existing pixel
            let er = pixels[idx] as f32;
            let eg = pixels[idx + 1] as f32;
            let eb = pixels[idx + 2] as f32;
            pixels[idx] = (er + (accent[0] - er) * glow_intensity) as u8;
            pixels[idx + 1] = (eg + (accent[1] - eg) * glow_intensity) as u8;
            pixels[idx + 2] = (eb + (accent[2] - eb) * glow_intensity) as u8;
        }
    }

    // --- Pass 3: Draw the grid lines themselves ---
    for (p0, p1, width) in &lines {
        let half_w = width / 2.0;

        let min_x = (p0.0.min(p1.0) - half_w - 2.0).max(0.0) as u32;
        let max_x = ((p0.0.max(p1.0) + half_w + 2.0) as u32 + 1).min(size);
        let min_y = (p0.1.min(p1.1) - half_w - 2.0).max(0.0) as u32;
        let max_y = ((p0.1.max(p1.1) + half_w + 2.0) as u32 + 1).min(size);

        let dx = p1.0 - p0.0;
        let dy = p1.1 - p0.1;
        let len_sq = dx * dx + dy * dy;

        if len_sq < 0.001 {
            continue;
        }

        for y in min_y..max_y {
            for x in min_x..max_x {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;

                let d = sdf_rounded_rect(px, py, center, center, inner_half, inner_half, inner_r);
                if d > 0.5 {
                    continue;
                }

                let vx = px - p0.0;
                let vy = py - p0.1;
                let t = ((vx * dx + vy * dy) / len_sq).clamp(0.0, 1.0);
                let cx = p0.0 + t * dx;
                let cy = p0.1 + t * dy;
                let dist = ((px - cx) * (px - cx) + (py - cy) * (py - cy)).sqrt();

                if dist < half_w + 1.0 {
                    let line_alpha = ((half_w + 0.8 - dist) / 1.2).clamp(0.0, 1.0);
                    let clip_alpha = (-d).clamp(0.0, 2.0).min(1.0);
                    let alpha = line_alpha * clip_alpha;

                    if alpha > 0.01 {
                        let idx = ((y * size + x) * 4) as usize;
                        let existing_a = pixels[idx + 3] as f32 / 255.0;
                        let src_a = alpha;
                        let out_a = src_a + existing_a * (1.0 - src_a);

                        if out_a > 0.001 {
                            let inv = 1.0 / out_a;
                            for c in 0..3 {
                                let src = accent[c] / 255.0;
                                let dst = pixels[idx + c] as f32 / 255.0;
                                pixels[idx + c] = ((src * src_a + dst * existing_a * (1.0 - src_a))
                                    * inv
                                    * 255.0)
                                    as u8;
                            }
                            pixels[idx + 3] = (out_a * 255.0) as u8;
                        }
                    }
                }
            }
        }
    }

    // Encode as PNG
    let mut png_data = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_data, size, size);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("png header");
        writer.write_image_data(&pixels).expect("png data");
    }

    let out_path = "editor/app-icon.png";
    std::fs::write(out_path, &png_data).expect("write png");
    println!("Wrote {out_path} ({} bytes)", png_data.len());
}
