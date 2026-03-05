# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.2.0] - 2026-03-05

### Added
- `maps.ron` metadata support for friendly map naming and size hints when tile count matches.
- A new Shape tool with a dropdown that lets you choose `Rect`, `Square`, `Circle`, or `Triangle`.
- The Shape tool now remembers your last selected shape and keeps that icon in the toolbar.
- Shape drawing now works directly in the map view with live preview before placing.
- The current map/file name is now shown in the status bar near zoom so it is always visible.

### Changed
- Undo and Redo icons are now simple back/forward arrows.
- The New Map dialog is cleaner and now focuses only on width and height inputs.
- Added clearer visual separation in the status bar between map name and zoom controls.

### Fixed
- Improved vertical alignment of `Width x Height` labels and fields in both size dialogs.
- The Custom Size dialog now warns when resizing would reduce tile count and truncate data.
- Removed dependency on tab hover for seeing filenames by showing the active filename in the status bar.
- Fixed a crash while typing size values by correcting truncation-warning arithmetic.
- Added guardrails so map width/height cannot be applied as `0` even outside dialog validation.
