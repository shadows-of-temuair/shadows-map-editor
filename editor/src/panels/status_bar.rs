use eframe::egui;

use crate::document::DocumentKind;
use crate::panels::MapSizeDialog;
use crate::panels::MapSizeDialogMode;
use crate::panels::Tool;
use crate::theme::{ThemeColors, theme_colors};
use crate::widgets::tooltip;

const STATUS_BAR_HEIGHT: f32 = 28.0;
const STATUS_PROGRESS_BAR_SIZE: egui::Vec2 = egui::vec2(112.0, 10.0);
const STATUS_MESSAGE_FADE_SECONDS: f64 = 0.18;
const STATUS_POSITION_WIDTH: f32 = 92.0;
const STATUS_SELECTION_WIDTH: f32 = 148.0;
const MAX_DIMENSION_CANDIDATES: usize = 12;

#[derive(Clone, Copy, PartialEq)]
pub enum StatusBarAction {
    None,
    ZoomIn,
    ZoomOut,
    SetDimensions(u16, u16),
}

#[derive(Default)]
pub struct StatusBarPanel {
    size_dialog: MapSizeDialog,
    settled_status_text: String,
    status_message_animation: Option<StatusMessageAnimation>,
}

struct StatusMessageAnimation {
    from_text: String,
    to_text: String,
    started_at: f64,
}

impl StatusBarPanel {
    fn plain_status_text(status_message: &str) -> String {
        status_message.trim_end_matches('.').to_string()
    }

    pub fn is_size_dialog_open(&self) -> bool {
        self.size_dialog.is_open()
    }

    fn finish_completed_status_animation(&mut self, now: f64) {
        let finished_text = self
            .status_message_animation
            .as_ref()
            .and_then(|animation| {
                ((now - animation.started_at) >= STATUS_MESSAGE_FADE_SECONDS)
                    .then(|| animation.to_text.clone())
            });
        if let Some(finished_text) = finished_text {
            self.settled_status_text = finished_text;
            self.status_message_animation = None;
        }
    }

    fn visible_status_text(&self, now: f64) -> String {
        self.status_message_animation
            .as_ref()
            .map(|animation| {
                let progress =
                    ((now - animation.started_at) / STATUS_MESSAGE_FADE_SECONDS).clamp(0.0, 1.0);
                if progress < 0.5 {
                    animation.from_text.clone()
                } else {
                    animation.to_text.clone()
                }
            })
            .unwrap_or_else(|| self.settled_status_text.clone())
    }

    fn animated_status_frame(&self, now: f64) -> Option<(String, f32)> {
        let animation = self.status_message_animation.as_ref()?;
        let progress =
            ((now - animation.started_at) / STATUS_MESSAGE_FADE_SECONDS).clamp(0.0, 1.0) as f32;
        if progress < 0.5 {
            Some((animation.from_text.clone(), 1.0 - (progress / 0.5)))
        } else {
            Some((animation.to_text.clone(), (progress - 0.5) / 0.5))
        }
    }

    fn status_label_state(
        &mut self,
        ctx: &egui::Context,
        status_message: &str,
        accent: egui::Color32,
    ) -> (String, egui::Color32) {
        let incoming_text = Self::plain_status_text(status_message);
        let now = ctx.input(|i| i.time);
        self.finish_completed_status_animation(now);

        if self.settled_status_text.is_empty() && self.status_message_animation.is_none() {
            self.settled_status_text = incoming_text.clone();
        }

        let current_target = self
            .status_message_animation
            .as_ref()
            .map(|animation| animation.to_text.as_str())
            .unwrap_or(self.settled_status_text.as_str())
            .to_string();
        if incoming_text != current_target {
            let from_text = self.visible_status_text(now);
            if from_text != incoming_text {
                self.status_message_animation = Some(StatusMessageAnimation {
                    from_text,
                    to_text: incoming_text.clone(),
                    started_at: now,
                });
            } else {
                self.settled_status_text = incoming_text.clone();
                self.status_message_animation = None;
            }
        }

        if let Some((text, alpha)) = self.animated_status_frame(now) {
            ctx.request_repaint();
            return (text, accent.gamma_multiply(alpha));
        }

        self.settled_status_text = incoming_text.clone();
        (incoming_text, accent)
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        map: &map::Map,
        document_kind: DocumentKind,
        current_file_label: &str,
        active_tool: Tool,
        hover_tile: (u16, u16),
        selection_dimensions: Option<(u16, u16)>,
        zoom: f32,
        status_message: &str,
        status_progress: Option<f32>,
    ) -> StatusBarAction {
        let colors = theme_colors();
        let (status_text, status_color) =
            self.status_label_state(ctx, status_message, colors.accent);
        let mut action = StatusBarAction::None;

        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(STATUS_BAR_HEIGHT)
            .frame(
                egui::Frame::NONE
                    .fill(colors.bg_2)
                    .inner_margin(egui::Margin {
                        left: 12,
                        right: 12,
                        top: 0,
                        bottom: 0,
                    }),
            )
            .show(ctx, |ui| {
                // Top border
                let rect = ui.max_rect();
                ui.painter().line_segment(
                    [
                        egui::pos2(rect.left() - 12.0, rect.top()),
                        egui::pos2(rect.right() + 12.0, rect.top()),
                    ],
                    egui::Stroke::new(1.0, colors.border),
                );

                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(16.0, 0.0);

                    // Tool indicator (left)
                    ui.label(
                        egui::RichText::new(active_tool.tooltip())
                            .size(13.0)
                            .color(colors.muted),
                    );

                    Self::separator(ui, &colors);

                    // Status (left, accent)
                    ui.label(
                        egui::RichText::new(status_text.as_str())
                            .size(13.0)
                            .color(status_color),
                    );

                    // Right-aligned section
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);

                        // Position — fixed width to prevent layout jitter
                        let pos_text = format!("Pos: {:>3}, {:>3}", hover_tile.0, hover_tile.1);
                        ui.add_sized(
                            egui::vec2(STATUS_POSITION_WIDTH, 18.0),
                            egui::Label::new(
                                egui::RichText::new(pos_text).size(13.0).color(colors.text),
                            ),
                        );

                        Self::separator(ui, &colors);

                        let selection_text = selection_dimensions
                            .map(|(width, height)| format!("Selection: {} x {}", width, height))
                            .unwrap_or_else(|| String::from("Selection: --"));
                        let selection_color = if selection_dimensions.is_some() {
                            colors.text
                        } else {
                            colors.muted
                        };
                        ui.add_sized(
                            egui::vec2(STATUS_SELECTION_WIDTH, 18.0),
                            egui::Label::new(
                                egui::RichText::new(selection_text)
                                    .size(13.0)
                                    .color(selection_color),
                            ),
                        );

                        Self::separator(ui, &colors);

                        // Dimensions dropdown — "caret | NxN | Size" (RTL order)
                        // Caret triangle (allocated in RTL flow, appears rightmost)
                        let (caret_rect, caret_response) =
                            ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::click());
                        {
                            let cx = caret_rect.center().x;
                            let cy = caret_rect.center().y;
                            let s = 4.5;
                            ui.painter().add(egui::Shape::convex_polygon(
                                vec![
                                    egui::pos2(cx - s, cy - s * 0.5),
                                    egui::pos2(cx + s, cy - s * 0.5),
                                    egui::pos2(cx, cy + s * 0.5),
                                ],
                                colors.muted,
                                egui::Stroke::NONE,
                            ));
                        }
                        let dim_text = format!("{}x{}", map.width, map.height);
                        let dim_response = ui.add(
                            egui::Button::new(
                                egui::RichText::new(&dim_text).size(13.0).color(colors.text),
                            )
                            .fill(egui::Color32::TRANSPARENT)
                            .stroke(egui::Stroke::NONE)
                            .corner_radius(0.0),
                        );
                        let popup_id = egui::Popup::default_response_id(&dim_response);
                        if caret_response.clicked() {
                            egui::Popup::toggle_id(ui.ctx(), popup_id);
                        }
                        if caret_response.hovered() {
                            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                        }
                        ui.label(egui::RichText::new("Size").size(13.0).color(colors.muted));

                        let popup_width = dim_response.rect.width().max(120.0);
                        egui::Popup::from_toggle_button_response(&dim_response)
                            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                            .width(popup_width)
                            .show(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(0.0, 2.0);
                                match document_kind {
                                    DocumentKind::Map => {
                                        let tile_count = map.tiles.len();
                                        let dims: Vec<_> = map::all_dimensions(tile_count)
                                            .into_iter()
                                            .take(MAX_DIMENSION_CANDIDATES)
                                            .collect();
                                        egui::ScrollArea::vertical().max_height(200.0).show(
                                            ui,
                                            |ui| {
                                                for (w, h) in dims {
                                                    let is_current =
                                                        w == map.width && h == map.height;
                                                    let label = format!("{}x{}", w, h);
                                                    let text_color = if is_current {
                                                        colors.accent
                                                    } else {
                                                        colors.text
                                                    };
                                                    let btn = ui.add(
                                                        egui::Button::new(
                                                            egui::RichText::new(&label)
                                                                .size(13.0)
                                                                .color(text_color),
                                                        )
                                                        .fill(egui::Color32::TRANSPARENT)
                                                        .stroke(egui::Stroke::NONE)
                                                        .min_size(egui::vec2(
                                                            ui.available_width(),
                                                            24.0,
                                                        ))
                                                        .corner_radius(4.0),
                                                    );
                                                    if btn.clicked() {
                                                        action =
                                                            StatusBarAction::SetDimensions(w, h);
                                                        egui::Popup::close_id(ui.ctx(), popup_id);
                                                    }
                                                }
                                            },
                                        );

                                        ui.add_space(4.0);
                                        ui.separator();
                                    }
                                    DocumentKind::Prefab => {}
                                }

                                let custom_label = match document_kind {
                                    DocumentKind::Map => "Custom Size...",
                                    DocumentKind::Prefab => "Resize Canvas...",
                                };
                                let custom_btn = ui.add(
                                    egui::Button::new(
                                        egui::RichText::new(custom_label)
                                            .size(13.0)
                                            .color(colors.text),
                                    )
                                    .fill(egui::Color32::TRANSPARENT)
                                    .stroke(egui::Stroke::NONE)
                                    .min_size(egui::vec2(ui.available_width(), 24.0))
                                    .corner_radius(4.0),
                                );
                                if custom_btn.clicked() {
                                    self.size_dialog.open(map.width, map.height);
                                    egui::Popup::close_id(ui.ctx(), popup_id);
                                }
                            });

                        Self::separator(ui, &colors);

                        // Zoom controls: [+] "Zoom N%" [-] (RTL order: + is rightmost)
                        if Self::zoom_button(ui, "+", &colors, "Zoom in (Cmd+Plus)") {
                            action = StatusBarAction::ZoomIn;
                        }

                        let zoom_text = format!("Zoom {}%", (zoom * 100.0).round() as i32);
                        ui.label(egui::RichText::new(zoom_text).size(13.0).color(colors.text));

                        if Self::zoom_button(ui, "-", &colors, "Zoom out (Cmd+Minus)") {
                            action = StatusBarAction::ZoomOut;
                        }

                        Self::separator(ui, &colors);

                        ui.label(
                            egui::RichText::new(current_file_label)
                                .size(13.0)
                                .color(colors.muted),
                        );

                        if let Some(progress) = status_progress {
                            Self::separator(ui, &colors);
                            Self::progress_bar(ui, progress, &colors);
                        }

                        Self::separator(ui, &colors);
                    });
                });
            });

        let (dialog_title, confirm_label, dialog_mode) = match document_kind {
            DocumentKind::Map => ("Custom Size", "Apply", MapSizeDialogMode::Standard),
            DocumentKind::Prefab => ("Resize Canvas", "Resize", MapSizeDialogMode::PrefabCanvas),
        };
        if let Some((width, height)) = self.size_dialog.show(
            ctx,
            "status_custom_size_dialog",
            dialog_title,
            confirm_label,
            Some(map),
            dialog_mode,
        ) {
            action = StatusBarAction::SetDimensions(width, height);
        }

        action
    }

    fn zoom_button(ui: &mut egui::Ui, label: &str, colors: &ThemeColors, tooltip: &str) -> bool {
        let size = egui::vec2(20.0, 20.0);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

        let bg = if response.hovered() {
            colors.panel_2
        } else {
            egui::Color32::TRANSPARENT
        };
        let border = if response.hovered() {
            egui::Stroke::new(1.0, colors.border)
        } else {
            egui::Stroke::NONE
        };

        ui.painter()
            .rect(rect, 4.0, bg, border, egui::StrokeKind::Inside);

        let text_color = if response.hovered() {
            colors.text
        } else {
            colors.muted
        };
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(14.0),
            text_color,
        );

        let _ = tooltip::attach(response.clone(), tooltip);
        response.clicked()
    }

    fn progress_bar(ui: &mut egui::Ui, progress: f32, colors: &ThemeColors) {
        let progress = progress.clamp(0.0, 1.0);
        let (rect, response) =
            ui.allocate_exact_size(STATUS_PROGRESS_BAR_SIZE, egui::Sense::hover());

        ui.painter().rect(
            rect,
            4.0,
            colors.bg_3,
            egui::Stroke::new(1.0, colors.border),
            egui::StrokeKind::Inside,
        );

        if progress > 0.0 {
            let fill_rect = egui::Rect::from_min_max(
                rect.min,
                egui::pos2(rect.min.x + rect.width() * progress, rect.max.y),
            );
            ui.painter().rect_filled(fill_rect, 4.0, colors.accent);
        }

        let _ = tooltip::attach(
            response,
            format!("Asset progress {}%", (progress * 100.0).round() as i32),
        );
    }

    fn separator(ui: &mut egui::Ui, colors: &ThemeColors) {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, 14.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 0.0, colors.border);
    }
}
