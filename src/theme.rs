use eframe::egui;

#[derive(Clone, Copy)]
pub struct ThemeColors {
    pub bg: egui::Color32,
    pub bg_2: egui::Color32,
    pub bg_3: egui::Color32,
    pub panel: egui::Color32,
    pub panel_2: egui::Color32,
    pub text: egui::Color32,
    pub muted: egui::Color32,
    pub accent: egui::Color32,
    pub accent_2: egui::Color32,
    pub border: egui::Color32,
}

pub fn theme_colors() -> ThemeColors {
    ThemeColors {
        bg: egui::Color32::from_rgb(11, 12, 14),
        bg_2: egui::Color32::from_rgb(18, 20, 24),
        bg_3: egui::Color32::from_rgb(26, 29, 34),
        panel: egui::Color32::from_rgb(20, 23, 27),
        panel_2: egui::Color32::from_rgb(30, 35, 40),
        text: egui::Color32::from_rgb(227, 227, 227),
        muted: egui::Color32::from_rgb(163, 167, 173),
        accent: egui::Color32::from_rgb(224, 138, 53),
        accent_2: egui::Color32::from_rgb(240, 176, 110),
        border: egui::Color32::from_rgb(44, 50, 56),
    }
}

pub fn apply_theme(ctx: &egui::Context) {
    let colors = theme_colors();
    let mut style = (*ctx.style()).clone();

    let control_bg = egui::Color32::from_rgb(10, 11, 13);
    let control_bg_hover = egui::Color32::from_rgb(12, 14, 17);

    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.window_margin = egui::Margin::same(12);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);

    style.visuals = egui::Visuals::dark();
    style.visuals.window_corner_radius = egui::CornerRadius::same(6);
    style.visuals.window_fill = colors.bg_2;
    style.visuals.window_stroke = egui::Stroke::new(1.0, colors.border);
    style.visuals.panel_fill = colors.bg;

    style.visuals.widgets.inactive.bg_fill = control_bg;
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, colors.border);
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, colors.muted);
    style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(4);

    style.visuals.widgets.hovered.bg_fill = control_bg_hover;
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, colors.accent);
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, colors.text);
    style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(4);

    style.visuals.widgets.active.bg_fill = control_bg_hover;
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, colors.accent);
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, colors.text);
    style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(4);

    style.visuals.widgets.open.bg_fill = control_bg_hover;
    style.visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, colors.accent);
    style.visuals.widgets.open.corner_radius = egui::CornerRadius::same(4);

    style.visuals.widgets.noninteractive.bg_fill = colors.bg_2;
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, colors.border);
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, colors.text);

    style.visuals.extreme_bg_color = control_bg;
    style.visuals.text_edit_bg_color = Some(control_bg);
    style.visuals.selection.bg_fill = colors.accent.gamma_multiply(0.4);
    style.visuals.selection.stroke = egui::Stroke::new(1.0, colors.accent);
    style.visuals.faint_bg_color = colors.bg_2;
    style.visuals.hyperlink_color = colors.accent;
    style.visuals.override_text_color = Some(colors.text);

    style.interaction.selectable_labels = false;

    ctx.set_style(style);
}
