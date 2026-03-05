use eframe::egui;

const SYMBOLICONS_FONT_NAME: &str = "ss-symbolicons-line";
const FILE_ICON_NEW: &str = "\u{EC01}";
const FILE_ICON_OPEN: &str = "\u{1F4C1}";
const FILE_ICON_SAVE: &str = "\u{1F4BE}";
const FILE_ICON_EXPORT: &str = "\u{1F304}";
const ACTION_ICON_UNDO: &str = "\u{238C}";
const ACTION_ICON_REDO: &str = "\u{F520}";
const TOOL_ICON_SELECT: &str = "\u{270B}";
const TOOL_ICON_BRUSH: &str = "\u{E224}";
const TOOL_ICON_LINE: &str = "\u{E205}";
const TOOL_ICON_ERASER: &str = "\u{1F4A3}";
const TOOL_ICON_FILL: &str = "\u{E225}";
const TOOL_ICON_EYEDROPPER: &str = "\u{E200}";

pub fn install_font(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let font_name = SYMBOLICONS_FONT_NAME.to_owned();
    let mut symbolicons_family = vec![font_name.clone()];
    if let Some(fallbacks) = fonts.families.get(&egui::FontFamily::Proportional) {
        symbolicons_family.extend(fallbacks.iter().cloned());
    }
    fonts.font_data.insert(
        font_name.clone(),
        egui::FontData::from_static(include_bytes!(
            "../../../fonts/webfonts/ss-symbolicons-line.ttf"
        ))
        .into(),
    );
    fonts
        .families
        .insert(symbolicons_font_family(), symbolicons_family);
    ctx.set_fonts(fonts);
}

fn symbolicons_font_family() -> egui::FontFamily {
    egui::FontFamily::Name(SYMBOLICONS_FONT_NAME.into())
}

pub fn symbol_icon_font_id(size: f32) -> egui::FontId {
    egui::FontId::new(size, symbolicons_font_family())
}

fn draw_symbol_icon(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32, glyph: &str) {
    let size = rect.width().min(rect.height()) * 0.60;
    painter.text(
        rect.center() + egui::vec2(0.0, 2.5),
        egui::Align2::CENTER_CENTER,
        glyph,
        symbol_icon_font_id(size),
        color,
    );
}

fn tool_icon_stroke(color: egui::Color32) -> egui::Stroke {
    egui::Stroke::new(1.7, color)
}

/// Draw a plus/cross icon (New file).
pub fn draw_icon_new(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    draw_symbol_icon(painter, rect, color, FILE_ICON_NEW);
}

/// Draw a folder outline (Open file).
pub fn draw_icon_open(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    draw_symbol_icon(painter, rect, color, FILE_ICON_OPEN);
}

/// Draw a floppy disk outline (Save).
pub fn draw_icon_save(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    draw_symbol_icon(painter, rect, color, FILE_ICON_SAVE);
}

/// Draw an arrow-out-of-box icon (Export).
pub fn draw_icon_export(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    draw_symbol_icon(painter, rect, color, FILE_ICON_EXPORT);
}

/// Draw a cursor arrow pointing upper-left (Select tool).
pub fn draw_icon_select(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    draw_symbol_icon(painter, rect, color, TOOL_ICON_SELECT);
}

/// Draw a brush glyph (Brush tool).
pub fn draw_icon_pencil(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    draw_symbol_icon(painter, rect, color, TOOL_ICON_BRUSH);
}

/// Draw an eraser block shape (Eraser tool).
pub fn draw_icon_eraser(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    draw_symbol_icon(painter, rect, color, TOOL_ICON_ERASER);
}

/// Draw a tilted bucket (Fill tool).
pub fn draw_icon_fill(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    draw_symbol_icon(painter, rect, color, TOOL_ICON_FILL);
}

/// Draw an eyedropper/pipette (Eyedropper tool).
pub fn draw_icon_eyedropper(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    draw_symbol_icon(painter, rect, color, TOOL_ICON_EYEDROPPER);
}

/// Draw a rectangle outline (Rectangle tool).
pub fn draw_icon_rectangle(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.34;
    let stroke = egui::Stroke::new(1.5, color);
    let inner = egui::Rect::from_center_size(c, egui::vec2(s * 2.0, s * 1.6));
    painter.rect_stroke(inner, 0.0, stroke, egui::StrokeKind::Inside);
}

/// Draw a composite shapes icon (Shape tool).
pub fn draw_icon_shapes(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let center = rect.center();
    let scale = rect.width().min(rect.height()) * 0.31;
    let stroke = tool_icon_stroke(color);

    let square = egui::Rect::from_center_size(
        egui::pos2(center.x - scale * 0.24, center.y + scale * 0.20),
        egui::vec2(scale * 1.46, scale * 1.46),
    );
    painter.rect_stroke(square, 0.0, stroke, egui::StrokeKind::Inside);

    painter.circle_stroke(
        egui::pos2(center.x + scale * 0.46, center.y - scale * 0.24),
        scale * 0.62,
        stroke,
    );
}

/// Draw a square outline (Square shape tool).
pub fn draw_icon_square(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.32;
    let stroke = egui::Stroke::new(1.5, color);
    let inner = egui::Rect::from_center_size(c, egui::vec2(s * 2.0, s * 2.0));
    painter.rect_stroke(inner, 0.0, stroke, egui::StrokeKind::Inside);
}

/// Draw a circle outline (Circle shape tool).
pub fn draw_icon_circle(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.32;
    painter.circle_stroke(c, r, egui::Stroke::new(1.5, color));
}

/// Draw an upright triangle outline (Triangle shape tool).
pub fn draw_icon_triangle(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.38;
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
    draw_symbol_icon(painter, rect, color, TOOL_ICON_LINE);
}

/// Draw a simple left-pointing arrow (Undo).
pub fn draw_icon_undo(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    draw_symbol_icon(painter, rect, color, ACTION_ICON_UNDO);
}

/// Draw a simple right-pointing arrow (Redo).
pub fn draw_icon_redo(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    draw_symbol_icon(painter, rect, color, ACTION_ICON_REDO);
}
