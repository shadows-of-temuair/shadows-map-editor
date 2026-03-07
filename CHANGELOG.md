# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.6.0] - 2026-03-07

### Added
- Ground animation (`gndani.tbl`) and wall animation (`stcani.tbl`) playback in the main viewport, including animated prefab/paste/move ghost previews and animated eyedropper wall feedback.
- Animated palette browsing for ground and wall tiles, with sequence-aware preview sizing so uneven animated wall frames stay stable in the inspector.
- `Trim Canvas` for prefabs, which shrinks the prefab to its occupied bounds without losing painted data.

### Changed
- Animated tile and wall sequences now appear only once in the palette, using the sequence's starting frame as the canonical browser entry.
- Prefab preview rendering in the inspector now animates and fits against the largest wall frame in each animation sequence to avoid jumping/clipping.
- Map size changes, prefab `Resize Canvas...`, and prefab `Trim Canvas` are now undoable.
- Prefab size actions now show `Trim Canvas` above `Resize Canvas...`, separated in the status-bar size menu.
- Leaving the `Select` tool now clears the active selection.
- Prefab inspector polish: the preview edit button now uses a hoverable pencil icon control, and the prefab search box now has a padded inline search icon and clearer empty-state copy.

### Fixed
- Ground animation lookup now matches the Chaos asset ID space instead of skipping valid animated tiles.
- Wall animation preview sizing no longer jitters in the palette or prefab preview when frames have different heights.
- Fast selection drag start is more responsive because the drag origin is latched from the initial mouse-down tile.

## [0.5.0] - 2026-03-07

### Added
- Rectangular tile selection with edge auto-pan, selection dimensions in the status bar, and keyboard delete/backspace support for clearing the selected active layer.
- Viewport selection actions for cut, copy, paste preview placement, create prefab from selection, duplicate-drag, and move previews with undo/redo support.
- Clipboard shortcuts (`Cmd+X`, `Cmd+C`, `Cmd+V`) plus multi-paste support while holding `Shift` during placement.
- Overwrite warnings for prefab, paste, move, and duplicate previews, including red tile overlays and wall warning outlines on clobbered destination walls.
- Richer selection visualization with pulsing tile highlights under walls, merged white wall silhouette outlines, and improved eyedropper wall targeting feedback.
- Modal and inspector presentation polish, including iconized dialog titles, iconized viewport context-menu actions with shortcut labels, and section icons for `Prefabs` and `Tile Palette`.

### Changed
- The editor now defaults to the `Select` tool on startup.
- Selection actions now follow visibility-aware layer rules so hidden layers are not cut, copied, deleted, or erased, and wall-visible workflows prefer wall-only manipulation by default.
- Paste now behaves like prefab placement with a ghost preview and no post-place selection, while shift-drag on a selection duplicates instead of changing layer inclusion.
- Creating prefabs from a selection now prompts for a name, optionally excludes ground by default, and trims the saved prefab to its actual occupied bounds.
- Quick eyedropper use from `Select` now switches into brush mode after picking, and wall-side sampling follows the current left/right wall target instead of `Shift`.
- Inspector resizing and prefab preview interactions now avoid leaking input into the viewport.

### Fixed
- Resizing the inspector no longer triggers viewport scrolling or selection edge-pan when dragging near the map boundary.
- Clicking on the map outside the current selection clears it reliably again.
- Viewport context-menu clicks no longer fight the map beneath them, and context-menu sizing/alignment is now stable.
- Paste preview anchoring is now consistent and no longer collapses unexpectedly to a `1x1` placement footprint.
- Tab-map toggle hotkey handling is no longer lost to transient egui keyboard focus state.

## [0.4.0] - 2026-03-07

### Added
- Full prefab authoring workflow with `.ron` prefab documents stored under `prefabs/`, prefab tabs, and map stamping with undo/redo support.
- Prefab browser management tools including search, inline rename, duplicate, delete confirmation, and rendered prefab previews in the inspector.
- Startup asset setup flow for missing Dark Ages `.dat` archives, with guided import into local `assets/` and inline status-bar progress during copy/load.
- Solid shape variants for rectangles, squares, circles, and triangles.

### Changed
- Asset loading now starts asynchronously after launch, reports progress in the status bar, and builds tile and wall atlas work concurrently.
- Prefab placement, ghost previews, and hover highlights now use occupied-bounds centering instead of raw canvas origin.
- Prefab resizing is now exposed as `Resize Canvas...`, recenters existing content in the new canvas, and warns before painted tiles would be clipped.
- Prefab editing now opens directly into brush mode with wall painting active, matching the most common prefab-editing workflow.
- The prefab inspector now uses a searchable list-and-preview layout with a draggable splitter, full-width rendered preview pane, and clearer destructive action styling.
- Save now writes back to an existing document path without reopening the save dialog.
- Status bar message changes now briefly crossfade for better visual awareness.

### Fixed
- Discarding dirty documents on app close no longer loops back into the unsaved-changes dialog.
- Typing in prefab search or inline rename fields no longer triggers global tool hotkeys.
- Inline prefab rename now correctly auto-selects text, commits on `Enter`, cancels on `Escape`, resolves on focus loss, and rejects duplicate names case-insensitively.
- Missing-asset startup no longer logs a spurious `assets` load warning or panics after the macOS folder picker returns.
- Prefab preview sizing and inspector layout now respect the status bar area and avoid clipping the rendered preview footer.

## [0.3.0] - 2026-03-05

### Added
- Ground and wall paint layers throughout the editor, including left/right wall targeting for painting, line, shape, fill, and eyedropper workflows.
- Dedicated wall palette browsing in the inspector, with side-aware wall previews, reveal-to-selection behavior, and explicit wall target controls.
- Eyedropper target highlighting in the viewport so the sampled ground/left-wall/right-wall target is visible before clicking.
- Keyboard shortcuts for paint-layer toggling (`T`) and wall-side toggling (`Q`).
- Shared hotkey-aware tooltips across the toolbar, viewport controls, status bar, and named tabs.
- Unsaved-changes confirmation before closing dirty tabs or exiting the app, with `Save` and `Discard` choices.

### Changed
- Tile palette controls were reorganized into cleaner inline `Ground`, `Wall`, and `Side` toggles with stronger selection states.
- Drawing-tool toolbar icons now render from an embedded Symbolicons font, with refreshed standard glyphs for select, brush, line, eraser, fill, eyedropper, file actions, and undo/redo.
- The primary paint tool is now labeled `Brush`, keeps the `B` shortcut, and the shape tool shortcut moved to `U`.
- Map resizing now preserves the map's linear tile buffer when dimensions change, making reshape, truncate, and regrow behavior predictable.
- Map metadata matching now only extracts IDs from canonical `LOD...` filenames to avoid false-positive map-name hints.
- Map tabs now show either the looked-up display name or the file name fallback, instead of combining both.
- Viewport mouse panning now uses a hand/grab cursor consistently while dragging.
- Toolbar and shape glyph sizing/alignment were tuned for cleaner visual centering and larger shape symbols.

### Fixed
- Undo/redo batching now handles wall painting edits instead of treating only ground strokes as first-class history operations.
- Wall previews and eyedropper sampling now honor the active paint layer instead of relying on cursor-side inference.
- Palette reveal scrolling and inspector selection feedback are more reliable after resizing and when switching between ground and wall palettes.
- The embedded icon font now includes proportional fallbacks, avoiding startup warnings when egui looks for replacement glyphs.
- Status bar messages no longer render with trailing periods.

## [0.2.0] - 2026-03-05

### Added
- Full in-editor map editing, including pencil painting, erase, fill, line drawing, and a shape tool.
- Shape tool dropdown with `Rect`, `Square`, `Circle`, and `Triangle`.
- Undo and redo support for edit operations.
- New map creation and custom map sizing dialogs.
- `maps.ron` metadata support for friendly map names and map-size hints when counts match.
- Status bar improvements, including always-visible active file name and clearer map/zoom layout.
- Tab collision overlay on the main map view, with a `Tab` toggle and button next to `Grid`.
- Asset loading from game `.dat` archives for tiles, palettes, and wall sprites.
- Wall/object rendering support and better scene layering for map visuals.
- Custom app icon/window frame polish and cross-platform build setup.

### Changed
- Undo and redo icons now use simple back/forward arrows.
- New Map and Custom Size dialogs were simplified and made easier to read.
