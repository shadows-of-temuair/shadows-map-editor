use eframe::egui;

use crate::theme::theme_colors;
use crate::widgets::icons;

const PREFAB_CREATE_ICON: &str = "\u{E5D8}";

pub enum PrefabCreateDialogAction {
    None,
    Create { name: String, include_ground: bool },
    Cancel,
}

pub struct PrefabCreateDialog {
    open: bool,
    name: String,
    include_ground: bool,
    error_message: Option<String>,
    should_focus_name: bool,
}

impl Default for PrefabCreateDialog {
    fn default() -> Self {
        Self {
            open: false,
            name: String::new(),
            include_ground: false,
            error_message: None,
            should_focus_name: false,
        }
    }
}

impl PrefabCreateDialog {
    pub fn open(&mut self) {
        self.open = true;
        self.name.clear();
        self.include_ground = false;
        self.error_message = None;
        self.should_focus_name = true;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.error_message = None;
        self.should_focus_name = false;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn restore_after_error(&mut self, name: String, include_ground: bool, error: String) {
        self.open = true;
        self.name = name;
        self.include_ground = include_ground;
        self.error_message = Some(error);
        self.should_focus_name = true;
    }

    pub fn show(&mut self, ctx: &egui::Context) -> PrefabCreateDialogAction {
        if !self.open {
            return PrefabCreateDialogAction::None;
        }

        let viewport = ctx.viewport_rect();
        let screen = ctx.content_rect();
        if !viewport.is_finite() || !screen.is_finite() {
            return PrefabCreateDialogAction::None;
        }

        let colors = theme_colors();
        let mut open = self.open;
        let mut action = PrefabCreateDialogAction::None;

        egui::Area::new(egui::Id::new("prefab_create_backdrop"))
            .order(egui::Order::Middle)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let response = ui.allocate_response(screen.size(), egui::Sense::click());
                ui.painter().rect_filled(
                    screen,
                    0.0,
                    egui::Color32::from_rgba_unmultiplied(0, 0, 0, 140),
                );
                if response.clicked() {
                    action = PrefabCreateDialogAction::Cancel;
                }
            });

        egui::Window::new("")
            .order(egui::Order::Foreground)
            .id(egui::Id::new("prefab_create_dialog"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .fixed_size(egui::vec2(420.0, 0.0))
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .frame(
                egui::Frame::NONE
                    .fill(colors.bg_2)
                    .stroke(egui::Stroke::new(1.0, colors.border))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::same(24)),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

                draw_modal_title(ui, &colors, PREFAB_CREATE_ICON, "Create Prefab");
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(
                        "Give this prefab a name. It will be added to your prefab library.",
                    )
                    .size(13.0)
                    .color(colors.text),
                );

                let name_id = ui.make_persistent_id("prefab_create_name");
                let name_response = ui.add(
                    egui::TextEdit::singleline(&mut self.name)
                        .id(name_id)
                        .hint_text("Prefab name"),
                );
                if self.should_focus_name {
                    name_response.request_focus();
                    self.should_focus_name = false;
                }

                ui.checkbox(&mut self.include_ground, "Include ground");

                if let Some(error_message) = self.error_message.as_ref() {
                    ui.label(
                        egui::RichText::new(error_message)
                            .size(12.0)
                            .color(colors.accent),
                    );
                }

                let submit = ui.input(|i| i.key_pressed(egui::Key::Enter));
                let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
                if escape {
                    action = PrefabCreateDialogAction::Cancel;
                }

                ui.add_space(6.0);
                let (sep_rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 1.0),
                    egui::Sense::hover(),
                );
                ui.painter().rect_filled(sep_rect, 0.0, colors.border);

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);

                        let cancel_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("Cancel").size(14.0).color(colors.text),
                            )
                            .fill(colors.bg_3)
                            .stroke(egui::Stroke::new(1.0, colors.border))
                            .corner_radius(4.0)
                            .min_size(egui::vec2(80.0, 32.0)),
                        );
                        if cancel_btn.clicked() {
                            action = PrefabCreateDialogAction::Cancel;
                        }

                        let create_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("Create")
                                    .size(14.0)
                                    .color(egui::Color32::from_rgb(10, 11, 13)),
                            )
                            .fill(colors.accent)
                            .stroke(egui::Stroke::NONE)
                            .corner_radius(4.0)
                            .min_size(egui::vec2(88.0, 32.0)),
                        );
                        if create_btn.clicked() || submit {
                            action = PrefabCreateDialogAction::Create {
                                name: self.name.clone(),
                                include_ground: self.include_ground,
                            };
                        }
                    });
                });
            });

        if matches!(action, PrefabCreateDialogAction::Cancel) {
            open = false;
        }
        self.open = open;
        action
    }
}

fn draw_modal_title(
    ui: &mut egui::Ui,
    colors: &crate::theme::ThemeColors,
    icon: &str,
    title: &str,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        ui.label(
            egui::RichText::new(icon)
                .font(icons::symbol_icon_font_id(18.0))
                .color(colors.text),
        );
        ui.label(
            egui::RichText::new(title)
                .size(18.0)
                .strong()
                .color(colors.text),
        );
    });
}
