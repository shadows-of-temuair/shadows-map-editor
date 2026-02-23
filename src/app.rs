use eframe::egui;

use crate::map::Map;
use crate::panels::{
    InspectorPanel, StatusBarPanel, TitleBarPanel, Tool, ToolbarPanel, ViewportPanel,
};
use crate::theme;

pub struct EditorApp {
    map: Map,
    active_tool: Tool,
    inspector: InspectorPanel,
}

impl EditorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply_theme(&cc.egui_ctx);

        Self {
            map: Map::new(50, 50),
            active_tool: Tool::Pencil,
            inspector: InspectorPanel::default(),
        }
    }
}

impl eframe::App for EditorApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.043, 0.047, 0.055, 1.0] // matches bg (#0b0c0e)
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        TitleBarPanel::show(ctx, frame);
        StatusBarPanel::show(ctx, &self.map, self.active_tool);
        ToolbarPanel::show(ctx, &mut self.active_tool);
        self.inspector.show(ctx);
        ViewportPanel::show(ctx, &self.map);
    }
}
