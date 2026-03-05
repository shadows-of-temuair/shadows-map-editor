# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
