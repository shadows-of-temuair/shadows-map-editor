# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
