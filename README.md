# Shadows Map Editor

Isometric map viewer and PNG exporter for Shadows of Temuair (Dark Ages) `.map` files.
Built with Rust and [egui](https://github.com/emilk/egui).

Supports both:
- Legacy single-palette rendering (`legend.pal`)
- Newer palette-table rendering (`mpt*` / `stc*` `.tbl` + `.pal`)

**This is a work in progress with map viewing, editing, and export support.**

![Screenshot](screenshot.png)

## Requirements

- **Rust stable** (2024 Edition) — install via [rustup](https://rustup.rs/)
- macOS, Windows, or Linux with GPU support for egui/wgpu

## Building

```sh
cargo build --release
```

The binary is output to `target/release/map-editor`.

## Assets

The editor loads game assets from `.dat` archive files in an `assets/` directory at the working directory root. These archives are not included in the repository — you must supply them from your own copy of the game.

**Required files** (inside the `.dat` archives):

| File | Purpose |
|------|---------|
| `TILEA.BMP` | Ground tile sprite sheet (56x27 px tiles) |
| `stcNNNNN.hpf` | Wall/object sprites (Huffman-compressed, 28px wide) |
| `SOTP.DAT` | Tile collision/passability data — **required for tab map rendering and export** |

**Optional animation tables:**

| File | Purpose |
|------|---------|
| `gndani.tbl` | Ground animation sequences and frame timing |
| `stcani.tbl` | Wall/object animation sequences and frame timing |

**Palette assets (one of these modes, auto-detected):**

| Mode | Files |
|------|-------|
| Legacy | `legend.pal` |
| Palette-table (newer clients) | Ground: `mpt*.tbl` + `mpt*.pal`<br>Walls: `stc*.tbl` + `stc*.pal` |

When palette-table assets are present, the editor uses them. If they are missing, it falls back to legacy `legend.pal` rendering. Mixed asset sets are also supported (for example, palette-table ground with legacy walls).

Without `SOTP.DAT`, the tab map preview in the inspector panel and tab map PNG export will be unavailable. Without the tile/wall assets, maps will open but tiles will not render.

Place your `.dat` archive files in the `assets/` directory:

```
shadows-map-editor/
  assets/
    ARCHIVE1.DAT
    ARCHIVE2.DAT
    ...
```

Archives are loaded alphabetically. If multiple archives contain the same filename, the last one (alphabetically) wins.

## Prefabs

Prefabs are reusable multi-tile map pieces for composing repeated world objects quickly: trees, tables, grave plots, wall segments, building chunks, and other stampable arrangements of ground and wall tiles.

Prefab definitions live in `prefabs/` at the project root, alongside `assets/`. Each prefab is stored as a `.ron` file and can be edited in its own tab (`prefab: ...`) using the same tile/wall tools as normal maps.

Typical prefab workflow:
- Build a reusable object once in a prefab tab
- Save it into `prefabs/`
- Select it from the prefab browser
- Stamp it into one or many maps with undo/redo support

Creating prefabs:
- Activate the `Prefab` tool (`P`) to switch the inspector from the tile palette to the prefab browser
- Click `New` to create a new prefab tab
- Paint the prefab using the normal ground/wall editing tools
- Save it as a `.ron` file in `prefabs/`

Managing prefabs:
- `Import` copies an existing `.ron` prefab into the local `prefabs/` registry
- The prefab list supports live search by partial filename match
- List rows show the prefab file stem and occupied dimensions, not the full canvas size
- Double-click a prefab row, or use `Edit` from the preview header, to open that prefab in a tab
- Right-click a prefab row for rename, duplicate, or `Delete Prefab`, which asks for confirmation before removing the local `.ron`
- The prefab size menu offers `Trim Canvas` and `Resize Canvas...`, both with undo/redo support

Placing prefabs:
- The bottom inspector pane shows a rendered preview of the selected prefab using the loaded ground and wall assets, including animation when animation tables are available
- Placement uses the center of the prefab's occupied area, so empty canvas padding does not shift the stamp
- Stamping participates in normal undo/redo history

Empty prefab cells are transparent when placed:
- `ground = 0` does not overwrite map ground
- `left_wall = 0` does not overwrite the destination left wall
- `right_wall = 0` does not overwrite the destination right wall

## Usage

Run from the project root (so the `assets/` directory is found):

```sh
cargo run --release
```

### Opening Documents

- **Cmd+O** — Open a `.map` file via file dialog
- **Drag and drop** — Drop `.map` or `.ron` files directly onto the window to open them
- **Cmd+N** — Open the New Map size dialog and create a new map
- **Cmd+W** — Close the active tab

If `maps.ron` exists in the project root, the editor uses it as map metadata:
- matches the map number from the filename (e.g. `LOD185.MAP` → `185`)
- applies the listed map name as the tab title
- applies the listed dimensions when `width * height` matches the file tile count

### Viewport

- **Scroll wheel** — Zoom in/out
- **Cmd+Plus / Cmd+Minus** — Zoom in/out (snaps to 25% increments)
- **Cmd+0** — Reset zoom to 100%
- **Middle mouse drag** — Pan the viewport

### Layer Visibility

- **Cmd+1** — Toggle ground tiles
- **Cmd+2** — Toggle left walls
- **Cmd+3** — Toggle right walls
- **Cmd+4** — Toggle grid overlay
- **Tab** — Toggle tab collision overlay on the main viewport (requires `SOTP.DAT`)
- The `Tab` overlay toggle affects only the main map viewport (not the inspector tab map or tab map PNG export).
- Floating viewport controls include `Grid` and `Tab` toggles in the top-right.
- The inspector bottom panel shows the map's `Tab Map` normally, and switches to a resizable `Preview` pane while the Prefab tool is active.

### Editing

- **V** — Switch to Select tool
- **B** — Switch to Brush tool
- **L** — Switch to Line tool
- **P** — Switch to Prefab tool
- **E** — Switch to Eraser tool
- **G** — Switch to Fill tool
- **I** — Switch to Eyedropper tool
- **U** — Switch to Shape tool
- **T** — Toggle tile paint layer between Ground and Wall (preserves the current wall side)
- **Q** — Toggle wall paint target side (Left/Right) while in Wall mode
- **Left drag (Select)** — Create a rectangular selection, with automatic edge-pan near viewport bounds
- **Click outside selection (Select)** — Clear the current selection
- **Esc (Select/Paste preview)** — Clear the selection and cancel paste preview if active
- **Delete / Backspace (Select)** — Clear the selected area on the active visible selection layers, or the hovered tile when no selection exists
- **Cmd+C / Cmd+X** — Copy or cut the current selection using the current visible-layer rules, or operate on the hovered tile when no selection exists
- **Cmd+V** — Start paste preview from the selection clipboard
- **Right click (viewport)** — Open the selection/clipboard context menu; `Paste` is available in any tool when the clipboard has content
- **Paste preview** — Shows a translucent animated ghost and orange footprint; left click places it, `Shift+left click` keeps the preview active for repeated placement
- **Left drag inside selection** — Move the selection with a live preview and full undo/redo support
- **Shift+Left drag inside selection** — Duplicate the selection to a new location and keep the new copy selected for chained duplication
- **Left drag on an occupied tile with no selection** — Starts a convenience `1x1` move or duplicate drag without leaving a sticky single-tile selection behind
- **Selection layer rules** — Hidden layers are never cut, copied, deleted, moved, duplicated, or erased; when any wall layer is visible, selection actions default to walls unless only ground is visible
- **Create Prefab... (selection menu)** — Prompts for a prefab name, optionally includes ground, and saves a trimmed prefab using only the occupied bounds; when no selection exists it can operate on the hovered tile
- **Switching away from Select** — Clears the current selection
- **Left click (Brush)** — Paint the hovered tile
- **Left drag (Brush)** — Paint continuously while holding the mouse button
- **Shift+Left click (Brush)** — Draw a line from the last brush click to the clicked tile (does nothing if there is no previous brush click)
- **Left click (Line)** — First click sets the line start, next click paints a line to that point (live preview shown while hovering)
- **Left click (Shape)** — First click sets shape start, next click draws outline to that point (live preview shown while hovering)
- **Shape dropdown (toolbar)** — Choose `Rect`, `Square`, `Circle`, or `Triangle` (last selected shape icon is shown on the button)
- **Esc** or **Right click (Line/Shape)** — Cancel the pending start point
- **Left click/drag (Eraser)** — Clear ground tiles (writes tile ID `0`)
- **Left click (Fill)** — Flood-fill contiguous ground region with the selected ground tile
- **Prefab tool** — Uses the selected prefab from the inspector's `Prefab Library`
- **Left click (Prefab)** — Place the selected prefab centered on the hovered tile using the occupied prefab bounds, with live translucent animated preview
- **Left click (Eyedropper)** — Pick the hovered value for the active palette mode (ground in Ground mode, left wall in Wall mode)
- **Wall eyedropper target** — Uses the current wall target side (toggle with `Q`)
- **Eyedropper hover highlight** — Shows exactly which ground/wall target will be sampled before clicking
- **Alt/Option (hold)** — Temporarily use Eyedropper while held; quick-picking from Select switches to Brush afterward
- **Prefab tool inspector** — Replaces the tile palette with the prefab browser
- **Search prefabs** — Filters the prefab list by partial filename match as you type
- **Prefab list rows** — Show the prefab file stem and the occupied dimensions, not the full canvas size
- **New** — Create a new prefab tab
- **Import** — Pick a `.ron` prefab file and copy it into the local `prefabs/` registry
- **Right click prefab row** — Open prefab actions for rename, duplicate, and delete
- **Preview** — The inspector bottom panel shows a rendered animated ground+wall preview of the selected prefab, scaled to fit

### Export

- **Cmd+E** — Open the export dialog
- Exports the map as a PNG at configurable scale (25%–400%)
- Optional solid background color (transparent by default)
- Optional tab map export (collision wireframe) as a separate `_tab.png` file with its own scale and background settings

### Map Dimensions

The status bar shows the current map dimensions. Click the dimension label to choose from all valid factor pairs for the tile count — useful for maps where the original width/height is ambiguous.
The size menu also includes **Custom Size...** to enter explicit `width x height` values (each from `1` to `65535`).
Map size changes are undoable.
When reducing total tile count in **Custom Size...**, the dialog shows a warning that data will be truncated.
For prefabs, the same size menu offers **Trim Canvas** and **Resize Canvas...**, and both are undoable.

### Status Bar

The active file name is shown in the status bar next to zoom controls so it is always visible without hovering tabs.

## Project Structure

```
├── archive/    # .dat archive memory-mapped loader
├── map/        # Map data structures and tile format
├── prefabs/    # Prefab `.ron` files used by the editor prefab tool
├── render/     # Palette, tile atlas, sprite atlas, HPF decoder
└── editor/     # egui application, UI panels, PNG export
```

## Map Format

Each `.map` file is raw binary — 6 bytes per tile (three little-endian `u16` values: ground ID, left wall ID, right wall ID). The editor infers map dimensions from the tile count.

## Prefab Format

Each `.ron` prefab stores `width`, `height`, and a flat `tiles` array. Tile fields are optional in RON and only non-zero layers are placed onto destination maps.
