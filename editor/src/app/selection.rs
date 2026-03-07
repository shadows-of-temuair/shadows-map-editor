use crate::document::{LayerVisibility, PaintLayer, TileSelection};

use super::EditorApp;

fn selection_move_layers_for_visibility(mut layers: LayerVisibility) -> LayerVisibility {
    if layers.left_wall || layers.right_wall {
        layers.ground = false;
    }
    layers
}

fn selection_duplicate_layers_for_visibility(layers: LayerVisibility) -> LayerVisibility {
    if layers.left_wall || layers.right_wall {
        LayerVisibility {
            ground: false,
            left_wall: layers.left_wall,
            right_wall: layers.right_wall,
        }
    } else {
        LayerVisibility {
            ground: layers.ground,
            left_wall: false,
            right_wall: false,
        }
    }
}

fn selection_action_layers_for_visibility(
    mut layers: LayerVisibility,
    shift_held: bool,
) -> LayerVisibility {
    if !shift_held && (layers.left_wall || layers.right_wall) {
        layers.ground = false;
    }
    layers
}

#[derive(Clone)]
pub(super) struct SelectionClipboard {
    pub map: map::Map,
    pub layers: crate::document::LayerVisibility,
}

#[derive(Clone)]
pub(super) enum SelectionDragMode {
    Selecting {
        allow_single_tile: bool,
    },
    Moving {
        original_selection: TileSelection,
        grab_offset: (u16, u16),
        preview_map: map::Map,
    },
}

impl EditorApp {
    pub(super) fn selection_action_layers(&self, shift_held: bool) -> LayerVisibility {
        selection_action_layers_for_visibility(self.layer_visibility, shift_held)
    }

    pub(super) fn selection_move_layers(&self) -> LayerVisibility {
        selection_move_layers_for_visibility(self.layer_visibility)
    }

    pub(super) fn selection_duplicate_layers(&self) -> LayerVisibility {
        selection_duplicate_layers_for_visibility(self.layer_visibility)
    }

    pub(super) fn clear_active_selection_layers(&mut self, action_layers: LayerVisibility) -> bool {
        let Some(selection) = self.documents[self.active_tab].selection() else {
            return false;
        };

        if !action_layers.any() {
            return false;
        }

        let cleared = self.documents[self.active_tab]
            .clear_selection_visible_layers(selection, action_layers);
        if cleared > 0 {
            self.status_message = "Cleared selected layers in selection.".to_string();
            return true;
        }

        false
    }

    pub(super) fn copy_active_selection_to_clipboard(
        &mut self,
        action_layers: LayerVisibility,
    ) -> bool {
        let Some(selection) = self.documents[self.active_tab].selection() else {
            self.status_message = "No selection to copy.".to_string();
            return false;
        };
        if !action_layers.any() {
            self.status_message = "No visible layers available to copy.".to_string();
            return false;
        }

        let copied = self.documents[self.active_tab]
            .selection_map_for_visible_layers(selection, action_layers);
        let (width, height) = (copied.width, copied.height);
        self.selection_clipboard = Some(SelectionClipboard {
            map: copied,
            layers: action_layers,
        });
        self.paste_preview_active = false;
        self.status_message = format!("Copied selection {}x{}.", width, height);
        true
    }

    pub(super) fn cut_active_selection_to_clipboard(
        &mut self,
        action_layers: LayerVisibility,
    ) -> bool {
        let Some(selection) = self.documents[self.active_tab].selection() else {
            self.status_message = "No selection to cut.".to_string();
            return false;
        };
        if !action_layers.any() {
            self.status_message = "No visible layers available to cut.".to_string();
            return false;
        }

        let copied = self.documents[self.active_tab]
            .selection_map_for_visible_layers(selection, action_layers);
        let (width, height) = (copied.width, copied.height);
        self.selection_clipboard = Some(SelectionClipboard {
            map: copied,
            layers: action_layers,
        });
        self.paste_preview_active = false;
        self.documents[self.active_tab].clear_selection_visible_layers(selection, action_layers);
        self.status_message = format!("Cut selection {}x{}.", width, height);
        true
    }

    pub(super) fn paste_selection_clipboard_at(
        &mut self,
        top_left: (u16, u16),
        keep_preview_active: bool,
    ) -> bool {
        let Some(clipboard) = self.selection_clipboard.clone() else {
            return false;
        };
        if !clipboard.layers.any() {
            return false;
        }

        let (width, height) = (clipboard.map.width, clipboard.map.height);
        let doc = &mut self.documents[self.active_tab];
        let changed_tiles = doc.paste_visible_layers(top_left, &clipboard.map, clipboard.layers);
        doc.set_selection(None);
        self.paste_preview_active = keep_preview_active;

        self.status_message = if changed_tiles > 0 {
            if keep_preview_active {
                format!("Pasted selection {}x{}. Multi-paste active.", width, height)
            } else {
                format!("Pasted selection {}x{}.", width, height)
            }
        } else {
            format!("Paste made no changes for selection {}x{}.", width, height)
        };
        true
    }

    pub(super) fn create_prefab_from_active_selection(&mut self) -> bool {
        self.begin_prefab_create_flow()
    }

    pub(super) fn clear_active_selection(&mut self) -> bool {
        let doc = &mut self.documents[self.active_tab];
        if doc.selection().is_none() {
            return false;
        }

        doc.set_selection(None);
        self.selection_drag_start_tile = None;
        self.selection_drag_mode = None;
        true
    }

    pub(super) fn moved_selection_top_left(
        map: &map::Map,
        original_selection: TileSelection,
        grab_offset: (u16, u16),
        hover_tile: (u16, u16),
    ) -> (u16, u16) {
        let (width, height) = original_selection.dimensions();
        let unclamped_col = hover_tile.0.saturating_sub(grab_offset.0);
        let unclamped_row = hover_tile.1.saturating_sub(grab_offset.1);
        let max_col = map.width.saturating_sub(width);
        let max_row = map.height.saturating_sub(height);
        (unclamped_col.min(max_col), unclamped_row.min(max_row))
    }

    pub(super) fn tile_value_for_layer(tile: &map::Tile, layer: PaintLayer) -> u16 {
        match layer {
            PaintLayer::Ground => tile.ground,
            PaintLayer::LeftWall => tile.left_wall,
            PaintLayer::RightWall => tile.right_wall,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        selection_action_layers_for_visibility, selection_duplicate_layers_for_visibility,
        selection_move_layers_for_visibility,
    };
    use crate::document::LayerVisibility;

    #[test]
    fn selection_action_layers_drop_ground_when_any_wall_is_visible_without_shift() {
        assert_eq!(
            selection_action_layers_for_visibility(
                LayerVisibility {
                    ground: true,
                    left_wall: true,
                    right_wall: false,
                },
                false,
            ),
            LayerVisibility {
                ground: false,
                left_wall: true,
                right_wall: false,
            }
        );

        assert_eq!(
            selection_action_layers_for_visibility(
                LayerVisibility {
                    ground: true,
                    left_wall: false,
                    right_wall: true,
                },
                false,
            ),
            LayerVisibility {
                ground: false,
                left_wall: false,
                right_wall: true,
            }
        );
    }

    #[test]
    fn selection_action_layers_keep_ground_for_ground_only_or_shift() {
        assert_eq!(
            selection_action_layers_for_visibility(
                LayerVisibility {
                    ground: true,
                    left_wall: false,
                    right_wall: false,
                },
                false,
            ),
            LayerVisibility {
                ground: true,
                left_wall: false,
                right_wall: false,
            }
        );

        assert_eq!(
            selection_action_layers_for_visibility(
                LayerVisibility {
                    ground: true,
                    left_wall: true,
                    right_wall: true,
                },
                true,
            ),
            LayerVisibility {
                ground: true,
                left_wall: true,
                right_wall: true,
            }
        );
    }

    #[test]
    fn selection_move_layers_drop_ground_when_any_wall_is_visible() {
        assert_eq!(
            selection_move_layers_for_visibility(LayerVisibility {
                ground: true,
                left_wall: true,
                right_wall: false,
            }),
            LayerVisibility {
                ground: false,
                left_wall: true,
                right_wall: false,
            }
        );

        assert_eq!(
            selection_move_layers_for_visibility(LayerVisibility {
                ground: true,
                left_wall: false,
                right_wall: false,
            }),
            LayerVisibility {
                ground: true,
                left_wall: false,
                right_wall: false,
            }
        );
    }

    #[test]
    fn selection_duplicate_layers_prefer_walls_and_fallback_to_ground() {
        assert_eq!(
            selection_duplicate_layers_for_visibility(LayerVisibility {
                ground: true,
                left_wall: true,
                right_wall: true,
            }),
            LayerVisibility {
                ground: false,
                left_wall: true,
                right_wall: true,
            }
        );

        assert_eq!(
            selection_duplicate_layers_for_visibility(LayerVisibility {
                ground: true,
                left_wall: false,
                right_wall: false,
            }),
            LayerVisibility {
                ground: true,
                left_wall: false,
                right_wall: false,
            }
        );
    }
}
