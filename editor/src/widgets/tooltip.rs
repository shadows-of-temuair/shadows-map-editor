use eframe::egui;

const TOOLTIP_TEXT_COLOR: egui::Color32 = egui::Color32::WHITE;
const TOOLTIP_HOTKEY_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 168, 72);
const TOOLTIP_FONT_SIZE: f32 = 13.0;

pub fn attach(response: egui::Response, text: impl AsRef<str>) -> egui::Response {
    let (label, hotkey) = split_label_and_hotkey(text.as_ref());
    response.on_hover_ui(move |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(&label)
                    .size(TOOLTIP_FONT_SIZE)
                    .family(egui::FontFamily::Monospace)
                    .color(TOOLTIP_TEXT_COLOR),
            );
            if let Some(hotkey) = &hotkey {
                ui.label(
                    egui::RichText::new(format!("[{hotkey}]"))
                        .size(TOOLTIP_FONT_SIZE)
                        .family(egui::FontFamily::Monospace)
                        .color(TOOLTIP_HOTKEY_COLOR),
                );
            }
        });
    })
}

fn split_label_and_hotkey(text: &str) -> (String, Option<String>) {
    let trimmed = text.trim();

    if let Some((label, hotkey)) = split_trailing_token(trimmed, '[', ']') {
        return (label.to_owned(), Some(hotkey.to_owned()));
    }
    if let Some((label, hotkey)) = split_trailing_token(trimmed, '(', ')') {
        return (label.to_owned(), Some(hotkey.to_owned()));
    }

    (trimmed.to_owned(), None)
}

fn split_trailing_token(text: &str, open: char, close: char) -> Option<(&str, &str)> {
    if !text.ends_with(close) {
        return None;
    }

    let marker = match open {
        '[' => " [",
        '(' => " (",
        _ => return None,
    };
    let start = text.rfind(marker)?;
    let token = &text[start + 2..text.len().saturating_sub(1)];
    if token.is_empty() || token.chars().any(char::is_whitespace) {
        return None;
    }

    let label = text[..start].trim_end();
    if label.is_empty() {
        return None;
    }

    Some((label, token))
}
