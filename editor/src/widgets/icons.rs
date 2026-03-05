use eframe::egui;

/// Draw a plus/cross icon (New file).
pub fn draw_icon_new(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.22;
    let stroke = egui::Stroke::new(1.5, color);
    painter.line_segment([egui::pos2(c.x - s, c.y), egui::pos2(c.x + s, c.y)], stroke);
    painter.line_segment([egui::pos2(c.x, c.y - s), egui::pos2(c.x, c.y + s)], stroke);
}

/// Draw a folder outline (Open file).
pub fn draw_icon_open(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.30;
    let stroke = egui::Stroke::new(1.5, color);

    // Taller folder with tab on top-left
    let back_bl = egui::pos2(c.x - s, c.y + s * 0.85);
    let back_tl = egui::pos2(c.x - s, c.y - s * 0.35);
    let tab_top = egui::pos2(c.x - s, c.y - s * 0.85);
    let tab_right = egui::pos2(c.x - s * 0.2, c.y - s * 0.85);
    let tab_notch = egui::pos2(c.x + s * 0.05, c.y - s * 0.35);
    let back_tr = egui::pos2(c.x + s, c.y - s * 0.35);
    let back_br = egui::pos2(c.x + s, c.y + s * 0.85);

    painter.line_segment([back_bl, back_tl], stroke);
    painter.line_segment([back_tl, tab_top], stroke);
    painter.line_segment([tab_top, tab_right], stroke);
    painter.line_segment([tab_right, tab_notch], stroke);
    painter.line_segment([tab_notch, back_tr], stroke);
    painter.line_segment([back_tr, back_br], stroke);
    painter.line_segment([back_br, back_bl], stroke);
}

/// Draw a floppy disk outline (Save).
pub fn draw_icon_save(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.28;
    let stroke = egui::Stroke::new(1.5, color);

    // Outer disk body
    let tl = egui::pos2(c.x - s, c.y - s);
    let tr = egui::pos2(c.x + s, c.y - s);
    let br = egui::pos2(c.x + s, c.y + s);
    let bl = egui::pos2(c.x - s, c.y + s);
    painter.line_segment([tl, tr], stroke);
    painter.line_segment([tr, br], stroke);
    painter.line_segment([br, bl], stroke);
    painter.line_segment([bl, tl], stroke);

    // Metal shutter (top inset rectangle)
    let st = s * 0.4;
    painter.line_segment(
        [
            egui::pos2(c.x - st, c.y - s),
            egui::pos2(c.x - st, c.y - s * 0.3),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(c.x - st, c.y - s * 0.3),
            egui::pos2(c.x + st, c.y - s * 0.3),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(c.x + st, c.y - s * 0.3),
            egui::pos2(c.x + st, c.y - s),
        ],
        stroke,
    );

    // Label area (bottom inset rectangle)
    let lw = s * 0.65;
    let lt = c.y + s * 0.15;
    let lb = c.y + s;
    painter.line_segment([egui::pos2(c.x - lw, lt), egui::pos2(c.x + lw, lt)], stroke);
    painter.line_segment([egui::pos2(c.x - lw, lt), egui::pos2(c.x - lw, lb)], stroke);
    painter.line_segment([egui::pos2(c.x + lw, lt), egui::pos2(c.x + lw, lb)], stroke);
}

/// Draw an arrow-out-of-box icon (Export).
pub fn draw_icon_export(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.26;
    let stroke = egui::Stroke::new(1.5, color);

    // Box (three sides — left, bottom, right)
    let box_top = c.y - s * 0.1;
    let box_bot = c.y + s;
    let box_l = c.x - s;
    let box_r = c.x + s;
    painter.line_segment(
        [egui::pos2(box_l, box_top), egui::pos2(box_l, box_bot)],
        stroke,
    );
    painter.line_segment(
        [egui::pos2(box_l, box_bot), egui::pos2(box_r, box_bot)],
        stroke,
    );
    painter.line_segment(
        [egui::pos2(box_r, box_bot), egui::pos2(box_r, box_top)],
        stroke,
    );

    // Upward arrow shaft
    let arrow_top = c.y - s;
    let arrow_bot = c.y + s * 0.4;
    painter.line_segment(
        [egui::pos2(c.x, arrow_top), egui::pos2(c.x, arrow_bot)],
        stroke,
    );

    // Arrowhead
    let ah = s * 0.35;
    painter.line_segment(
        [
            egui::pos2(c.x, arrow_top),
            egui::pos2(c.x - ah, arrow_top + ah),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(c.x, arrow_top),
            egui::pos2(c.x + ah, arrow_top + ah),
        ],
        stroke,
    );
}

/// Draw a cursor arrow pointing upper-left (Select tool).
pub fn draw_icon_select(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.32;
    let stroke = egui::Stroke::new(1.5, color);

    // Classic pointer cursor. The right edge is a clean diagonal from
    // tip to right_wing, then the tail branches off below right_wing.
    let tip = egui::pos2(c.x - s * 0.55, c.y - s);
    let left_bot = egui::pos2(c.x - s * 0.55, c.y + s * 0.65);
    let notch = egui::pos2(c.x - s * 0.15, c.y + s * 0.25);
    let tail_bot = egui::pos2(c.x + s * 0.45, c.y + s);
    let tail_top = egui::pos2(c.x + s * 0.1, c.y + s * 0.55);
    let right_wing = egui::pos2(c.x + s * 0.18, c.y - s * 0.15);

    painter.line_segment([tip, left_bot], stroke);
    painter.line_segment([left_bot, notch], stroke);
    painter.line_segment([notch, tail_bot], stroke);
    painter.line_segment([tail_bot, tail_top], stroke);
    painter.line_segment([tail_top, right_wing], stroke);
    painter.line_segment([right_wing, tip], stroke);
}

/// Draw a pencil shape (Pencil tool).
pub fn draw_icon_pencil(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.30;
    let stroke = egui::Stroke::new(1.5, color);

    // Pencil body: diagonal from bottom-left (tip) to top-right (eraser end)
    // Tip at bottom-left
    let tip = egui::pos2(c.x - s * 0.8, c.y + s * 0.8);
    // Body corners (rotated rectangle)
    let w = s * 0.22; // half-width of pencil body
    let bl = egui::pos2(c.x - s * 0.45 - w, c.y + s * 0.45 + w);
    let br = egui::pos2(c.x - s * 0.45 + w, c.y + s * 0.45 - w);
    let tr = egui::pos2(c.x + s * 0.8 + w, c.y - s * 0.8 - w);
    let tl = egui::pos2(c.x + s * 0.8 - w, c.y - s * 0.8 + w);

    // Tip
    painter.line_segment([tip, bl], stroke);
    painter.line_segment([tip, br], stroke);

    // Body
    painter.line_segment([bl, tl], stroke);
    painter.line_segment([br, tr], stroke);

    // Top cap
    painter.line_segment([tl, tr], stroke);

    // Line separating tip from body
    painter.line_segment([bl, br], stroke);
}

/// Draw an eraser block shape (Eraser tool).
pub fn draw_icon_eraser(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.28;
    let stroke = egui::Stroke::new(1.5, color);

    // Parallelogram eraser shape, tilted
    let tl = egui::pos2(c.x - s * 0.3, c.y - s * 0.7);
    let tr = egui::pos2(c.x + s, c.y - s * 0.7);
    let br = egui::pos2(c.x + s * 0.3, c.y + s * 0.7);
    let bl = egui::pos2(c.x - s, c.y + s * 0.7);

    painter.line_segment([tl, tr], stroke);
    painter.line_segment([tr, br], stroke);
    painter.line_segment([br, bl], stroke);
    painter.line_segment([bl, tl], stroke);

    // Dividing line (separates rubber from grip)
    let ml = egui::pos2(c.x - s * 0.65, c.y + s * 0.0);
    let mr = egui::pos2(c.x + s * 0.65, c.y + s * 0.0);
    painter.line_segment([ml, mr], stroke);
}

/// Draw a tilted bucket (Fill tool).
pub fn draw_icon_fill(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.28;
    let stroke = egui::Stroke::new(1.5, color);

    // Simple tilted rectangle — a bucket tipping to pour
    let tl = egui::pos2(c.x - s * 0.9, c.y - s * 0.2);
    let tr = egui::pos2(c.x + s * 0.3, c.y - s * 0.8);
    let br = egui::pos2(c.x + s * 0.7, c.y + s * 0.2);
    let bl = egui::pos2(c.x - s * 0.5, c.y + s * 0.8);

    painter.line_segment([tl, tr], stroke);
    painter.line_segment([tr, br], stroke);
    painter.line_segment([br, bl], stroke);
    painter.line_segment([bl, tl], stroke);

    // Pour drop
    let drop = egui::pos2(c.x + s * 0.8, c.y + s * 0.65);
    painter.circle_filled(drop, s * 0.15, color);
}

/// Draw an eyedropper/pipette (Eyedropper tool).
pub fn draw_icon_eyedropper(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.30;
    let stroke = egui::Stroke::new(1.5, color);

    // Tip at bottom-left
    let tip = egui::pos2(c.x - s * 0.8, c.y + s * 0.8);

    // Body (diagonal tube from tip toward upper-right)
    let w = s * 0.18;
    let b1 = egui::pos2(c.x - s * 0.35 - w, c.y + s * 0.35 + w);
    let b2 = egui::pos2(c.x - s * 0.35 + w, c.y + s * 0.35 - w);
    let t1 = egui::pos2(c.x + s * 0.4 - w, c.y - s * 0.4 + w);
    let t2 = egui::pos2(c.x + s * 0.4 + w, c.y - s * 0.4 - w);

    // Tip lines
    painter.line_segment([tip, b1], stroke);
    painter.line_segment([tip, b2], stroke);

    // Body sides
    painter.line_segment([b1, t1], stroke);
    painter.line_segment([b2, t2], stroke);

    // Bulb at top (circle)
    let bulb_center = egui::pos2(c.x + s * 0.55, c.y - s * 0.55);
    painter.circle_stroke(bulb_center, s * 0.3, stroke);
}

/// Draw a rectangle outline (Rectangle tool).
pub fn draw_icon_rectangle(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.30;
    let stroke = egui::Stroke::new(1.5, color);
    let inner = egui::Rect::from_center_size(c, egui::vec2(s * 2.0, s * 1.6));
    painter.rect_stroke(inner, 0.0, stroke, egui::StrokeKind::Inside);
}

/// Draw a square outline (Square shape tool).
pub fn draw_icon_square(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.28;
    let stroke = egui::Stroke::new(1.5, color);
    let inner = egui::Rect::from_center_size(c, egui::vec2(s * 2.0, s * 2.0));
    painter.rect_stroke(inner, 0.0, stroke, egui::StrokeKind::Inside);
}

/// Draw a circle outline (Circle shape tool).
pub fn draw_icon_circle(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.28;
    painter.circle_stroke(c, r, egui::Stroke::new(1.5, color));
}

/// Draw an upright triangle outline (Triangle shape tool).
pub fn draw_icon_triangle(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.34;
    let stroke = egui::Stroke::new(1.5, color);
    let top = egui::pos2(c.x, c.y - s * 0.85);
    let left = egui::pos2(c.x - s, c.y + s * 0.75);
    let right = egui::pos2(c.x + s, c.y + s * 0.75);
    painter.line_segment([top, left], stroke);
    painter.line_segment([left, right], stroke);
    painter.line_segment([right, top], stroke);
}

/// Draw a simple diagonal segment with endpoints (Line tool).
pub fn draw_icon_line(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.30;
    let stroke = egui::Stroke::new(1.5, color);

    let start = egui::pos2(c.x - s, c.y + s * 0.8);
    let end = egui::pos2(c.x + s, c.y - s * 0.8);
    painter.line_segment([start, end], stroke);
    painter.circle_filled(start, 1.6, color);
    painter.circle_filled(end, 1.6, color);
}

/// Draw a simple left-pointing arrow (Undo).
pub fn draw_icon_undo(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.28;
    let stroke = egui::Stroke::new(1.5, color);

    let tip = egui::pos2(c.x - s, c.y);
    let tail = egui::pos2(c.x + s, c.y);
    let head = s * 0.55;
    painter.line_segment([tail, tip], stroke);
    painter.line_segment([egui::pos2(tip.x + head, tip.y - head), tip], stroke);
    painter.line_segment([egui::pos2(tip.x + head, tip.y + head), tip], stroke);
}

/// Draw a simple right-pointing arrow (Redo).
pub fn draw_icon_redo(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.28;
    let stroke = egui::Stroke::new(1.5, color);

    let tip = egui::pos2(c.x + s, c.y);
    let tail = egui::pos2(c.x - s, c.y);
    let head = s * 0.55;
    painter.line_segment([tail, tip], stroke);
    painter.line_segment([egui::pos2(tip.x - head, tip.y - head), tip], stroke);
    painter.line_segment([egui::pos2(tip.x - head, tip.y + head), tip], stroke);
}
