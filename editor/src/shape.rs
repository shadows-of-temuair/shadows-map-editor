use std::collections::HashSet;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ShapeKind {
    Rect,
    SolidRect,
    Square,
    SolidSquare,
    Circle,
    SolidCircle,
    Triangle,
    SolidTriangle,
}

impl ShapeKind {
    pub const ALL: [Self; 8] = [
        Self::Rect,
        Self::SolidRect,
        Self::Square,
        Self::SolidSquare,
        Self::Circle,
        Self::SolidCircle,
        Self::Triangle,
        Self::SolidTriangle,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Rect => "Rect",
            Self::SolidRect => "Solid Rect",
            Self::Square => "Square",
            Self::SolidSquare => "Solid Square",
            Self::Circle => "Circle",
            Self::SolidCircle => "Solid Circle",
            Self::Triangle => "Triangle",
            Self::SolidTriangle => "Solid Triangle",
        }
    }

    pub fn is_solid(self) -> bool {
        matches!(
            self,
            Self::SolidRect | Self::SolidSquare | Self::SolidCircle | Self::SolidTriangle
        )
    }

    fn base(self) -> Self {
        match self {
            Self::Rect | Self::SolidRect => Self::Rect,
            Self::Square | Self::SolidSquare => Self::Square,
            Self::Circle | Self::SolidCircle => Self::Circle,
            Self::Triangle | Self::SolidTriangle => Self::Triangle,
        }
    }
}

#[cfg(test)]
pub fn outline_points(shape: ShapeKind, start: (u16, u16), end: (u16, u16)) -> Vec<(i32, i32)> {
    let start = (start.0 as i32, start.1 as i32);
    let end = (end.0 as i32, end.1 as i32);
    let mut points = Vec::new();

    append_shape_outline(&mut points, shape.base(), start, end);

    dedup_points(points)
}

pub fn paint_points(shape: ShapeKind, start: (u16, u16), end: (u16, u16)) -> Vec<(i32, i32)> {
    let start = (start.0 as i32, start.1 as i32);
    let end = (end.0 as i32, end.1 as i32);
    let mut points = Vec::new();

    if shape.is_solid() {
        append_shape_fill(&mut points, shape.base(), start, end);
        append_shape_outline(&mut points, shape.base(), start, end);
    } else {
        append_shape_outline(&mut points, shape.base(), start, end);
    }

    dedup_points(points)
}

fn append_shape_outline(
    points: &mut Vec<(i32, i32)>,
    shape: ShapeKind,
    start: (i32, i32),
    end: (i32, i32),
) {
    match shape {
        ShapeKind::Rect => append_rect_outline(points, start, end),
        ShapeKind::Square => append_rect_outline(points, start, constrain_square_end(start, end)),
        ShapeKind::Circle => append_circle_outline(points, start, constrain_square_end(start, end)),
        ShapeKind::Triangle => append_triangle_outline(points, start, end),
        ShapeKind::SolidRect
        | ShapeKind::SolidSquare
        | ShapeKind::SolidCircle
        | ShapeKind::SolidTriangle => unreachable!("solid variants are normalized via base()"),
    }
}

fn append_shape_fill(
    points: &mut Vec<(i32, i32)>,
    shape: ShapeKind,
    start: (i32, i32),
    end: (i32, i32),
) {
    match shape {
        ShapeKind::Rect => append_rect_fill(points, start, end),
        ShapeKind::Square => append_rect_fill(points, start, constrain_square_end(start, end)),
        ShapeKind::Circle => append_circle_fill(points, start, constrain_square_end(start, end)),
        ShapeKind::Triangle => append_triangle_fill(points, start, end),
        ShapeKind::SolidRect
        | ShapeKind::SolidSquare
        | ShapeKind::SolidCircle
        | ShapeKind::SolidTriangle => unreachable!("solid variants are normalized via base()"),
    }
}

fn append_rect_outline(points: &mut Vec<(i32, i32)>, start: (i32, i32), end: (i32, i32)) {
    let min_x = start.0.min(end.0);
    let max_x = start.0.max(end.0);
    let min_y = start.1.min(end.1);
    let max_y = start.1.max(end.1);

    for x in min_x..=max_x {
        points.push((x, min_y));
        points.push((x, max_y));
    }
    for y in min_y..=max_y {
        points.push((min_x, y));
        points.push((max_x, y));
    }
}

fn append_rect_fill(points: &mut Vec<(i32, i32)>, start: (i32, i32), end: (i32, i32)) {
    let min_x = start.0.min(end.0);
    let max_x = start.0.max(end.0);
    let min_y = start.1.min(end.1);
    let max_y = start.1.max(end.1);

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            points.push((x, y));
        }
    }
}

fn append_triangle_outline(points: &mut Vec<(i32, i32)>, start: (i32, i32), end: (i32, i32)) {
    if start == end {
        points.push(start);
        return;
    }

    let (apex, base_a, base_b) = triangle_vertices(start, end);
    append_line(points, apex, base_a);
    append_line(points, apex, base_b);
    append_line(points, base_a, base_b);
}

fn append_triangle_fill(points: &mut Vec<(i32, i32)>, start: (i32, i32), end: (i32, i32)) {
    if start == end {
        points.push(start);
        return;
    }

    let (apex, base_a, base_b) = triangle_vertices(start, end);
    let min_x = apex.0.min(base_a.0).min(base_b.0);
    let max_x = apex.0.max(base_a.0).max(base_b.0);
    let min_y = apex.1.min(base_a.1).min(base_b.1);
    let max_y = apex.1.max(base_a.1).max(base_b.1);

    let a = (apex.0 as f32, apex.1 as f32);
    let b = (base_a.0 as f32, base_a.1 as f32);
    let c = (base_b.0 as f32, base_b.1 as f32);

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if point_in_triangle((x as f32, y as f32), a, b, c) {
                points.push((x, y));
            }
        }
    }
}

fn triangle_vertices(start: (i32, i32), end: (i32, i32)) -> ((i32, i32), (i32, i32), (i32, i32)) {
    // Oriented isosceles triangle:
    // - apex follows the cursor (`end`)
    // - base is centered on the initial click (`start`)
    let dx = (end.0 - start.0) as f32;
    let dy = (end.1 - start.1) as f32;
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let ux = dx / len;
    let uy = dy / len;
    let perp_x = -uy;
    let perp_y = ux;
    let half_base = (len * 0.5).max(1.0);

    let base_a = (
        (start.0 as f32 + perp_x * half_base).round() as i32,
        (start.1 as f32 + perp_y * half_base).round() as i32,
    );
    let base_b = (
        (start.0 as f32 - perp_x * half_base).round() as i32,
        (start.1 as f32 - perp_y * half_base).round() as i32,
    );
    let apex = end;

    (apex, base_a, base_b)
}

fn append_circle_outline(points: &mut Vec<(i32, i32)>, start: (i32, i32), end: (i32, i32)) {
    let min_x = start.0.min(end.0);
    let max_x = start.0.max(end.0);
    let min_y = start.1.min(end.1);
    let max_y = start.1.max(end.1);
    let side = (max_x - min_x).abs().max((max_y - min_y).abs());

    if side == 0 {
        points.push(start);
        return;
    }

    let center_x = (min_x as f32 + max_x as f32) * 0.5;
    let center_y = (min_y as f32 + max_y as f32) * 0.5;
    let radius = side as f32 * 0.5;
    let steps = (radius * 8.0).round().clamp(24.0, 720.0) as i32;

    for i in 0..steps {
        let t = i as f32 / steps as f32 * std::f32::consts::TAU;
        let x = (center_x + radius * t.cos()).round() as i32;
        let y = (center_y + radius * t.sin()).round() as i32;
        points.push((x, y));
    }
}

fn append_circle_fill(points: &mut Vec<(i32, i32)>, start: (i32, i32), end: (i32, i32)) {
    let min_x = start.0.min(end.0);
    let max_x = start.0.max(end.0);
    let min_y = start.1.min(end.1);
    let max_y = start.1.max(end.1);
    let side = (max_x - min_x).abs().max((max_y - min_y).abs());

    if side == 0 {
        points.push(start);
        return;
    }

    let center_x = (min_x as f32 + max_x as f32) * 0.5;
    let center_y = (min_y as f32 + max_y as f32) * 0.5;
    let radius = side as f32 * 0.5;
    let radius_sq = radius * radius + 0.25;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;
            if dx * dx + dy * dy <= radius_sq {
                points.push((x, y));
            }
        }
    }
}

fn append_line(points: &mut Vec<(i32, i32)>, start: (i32, i32), end: (i32, i32)) {
    let (mut x0, mut y0) = start;
    let (x1, y1) = end;
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        points.push((x0, y0));
        if x0 == x1 && y0 == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

fn constrain_square_end(start: (i32, i32), end: (i32, i32)) -> (i32, i32) {
    let (x0, y0) = start;
    let dx = end.0 - x0;
    let dy = end.1 - y0;
    let side = dx.abs().max(dy.abs());
    if side == 0 {
        return start;
    }

    let mut sx = dx.signum();
    let mut sy = dy.signum();
    if sx == 0 {
        sx = if sy == 0 { 1 } else { sy };
    }
    if sy == 0 {
        sy = sx;
    }

    (x0 + sx * side, y0 + sy * side)
}

fn point_in_triangle(point: (f32, f32), a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
    let d1 = edge_function(point, a, b);
    let d2 = edge_function(point, b, c);
    let d3 = edge_function(point, c, a);
    let epsilon = 0.001;
    let has_neg = d1 < -epsilon || d2 < -epsilon || d3 < -epsilon;
    let has_pos = d1 > epsilon || d2 > epsilon || d3 > epsilon;
    !(has_neg && has_pos)
}

fn edge_function(point: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    (point.0 - b.0) * (a.1 - b.1) - (a.0 - b.0) * (point.1 - b.1)
}

fn dedup_points(points: Vec<(i32, i32)>) -> Vec<(i32, i32)> {
    let mut out = Vec::with_capacity(points.len());
    let mut seen = HashSet::with_capacity(points.len());
    for point in points {
        if seen.insert(point) {
            out.push(point);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{ShapeKind, outline_points, paint_points};

    #[test]
    fn rect_contains_expected_corners() {
        let points = outline_points(ShapeKind::Rect, (1, 2), (4, 5));
        assert!(points.contains(&(1, 2)));
        assert!(points.contains(&(4, 2)));
        assert!(points.contains(&(1, 5)));
        assert!(points.contains(&(4, 5)));
    }

    #[test]
    fn square_constrains_to_equal_side() {
        let points = outline_points(ShapeKind::Square, (10, 10), (13, 11));
        assert!(points.contains(&(10, 10)));
        assert!(points.contains(&(13, 13)));
        assert!(points.contains(&(10, 13)));
    }

    #[test]
    fn circle_stays_within_constrained_square() {
        let points = outline_points(ShapeKind::Circle, (20, 20), (24, 22));
        assert!(!points.is_empty());
        for (x, y) in points {
            assert!((20..=24).contains(&x));
            assert!((20..=24).contains(&y));
        }
    }

    #[test]
    fn triangle_apex_follows_cursor_horizontal() {
        let points = outline_points(ShapeKind::Triangle, (10, 10), (14, 10));
        assert!(points.contains(&(14, 10)));
        assert!(points.contains(&(10, 8)));
        assert!(points.contains(&(10, 12)));
    }

    #[test]
    fn triangle_apex_follows_cursor_vertical() {
        let points = outline_points(ShapeKind::Triangle, (10, 10), (10, 6));
        assert!(points.contains(&(10, 6)));
        assert!(points.contains(&(8, 10)));
        assert!(points.contains(&(12, 10)));
    }

    #[test]
    fn solid_rect_fills_interior() {
        let points = paint_points(ShapeKind::SolidRect, (1, 2), (4, 5));
        assert!(points.contains(&(2, 3)));
        assert!(points.contains(&(3, 4)));
    }

    #[test]
    fn solid_square_fills_constrained_bounds() {
        let points = paint_points(ShapeKind::SolidSquare, (10, 10), (13, 11));
        assert!(points.contains(&(11, 11)));
        assert!(points.contains(&(12, 12)));
        assert!(points.contains(&(13, 13)));
    }

    #[test]
    fn solid_circle_includes_center_tile() {
        let points = paint_points(ShapeKind::SolidCircle, (20, 20), (24, 22));
        assert!(points.contains(&(22, 22)));
    }

    #[test]
    fn solid_triangle_fills_interior() {
        let points = paint_points(ShapeKind::SolidTriangle, (10, 10), (14, 10));
        assert!(points.contains(&(12, 10)));
        assert!(points.contains(&(14, 10)));
    }
}
