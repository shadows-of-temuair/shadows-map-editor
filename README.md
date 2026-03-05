# Shadows Map Editor

Isometric map viewer and PNG exporter for Shadows of Temuair (Dark Ages) `.map` files.
Built with Rust and [egui](https://github.com/emilk/egui).

**This is a work in progress. Currently only allows viewing and exporting maps.**

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
| `legend.pal` | 256-color RGB palette used for all indexed-color images |
| `TILEA.BMP` | Ground tile sprite sheet (56x27 px tiles) |
| `stcNNNNN.hpf` | Wall/object sprites (Huffman-compressed, 28px wide) |
| `SOTP.DAT` | Tile collision/passability data — **required for tab map rendering and export** |

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

## Usage

Run from the project root (so the `assets/` directory is found):

```sh
cargo run --release
```

### Opening Maps

- **Cmd+O** — Open a `.map` file via file dialog
- **Drag and drop** — Drop `.map` files directly onto the window to open them
- **Cmd+N** — New blank 50x50 map
- **Cmd+W** — Close the active tab

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

### Editing

- **B** — Switch to Pencil tool
- **L** — Switch to Line tool
- **E** — Switch to Eraser tool
- **G** — Switch to Fill tool
- **I** — Switch to Eyedropper tool
- **Left click (Pencil)** — Paint the hovered tile
- **Left drag (Pencil)** — Paint continuously while holding the mouse button
- **Shift+Left click (Pencil)** — Draw a line from the last pencil click to the clicked tile (does nothing if there is no previous pencil click)
- **Left click (Line)** — First click sets the line start, next click paints a line to that point (live preview shown while hovering)
- **Esc** or **Right click (Line)** — Cancel the pending line start point
- **Left click/drag (Eraser)** — Clear ground tiles (writes tile ID `0`)
- **Left click (Fill)** — Flood-fill contiguous ground region with the selected ground tile
- **Left click (Eyedropper)** — Pick the hovered ground tile
- **Shift+Left click (Eyedropper)** — Pick hovered wall tile (left/right wall chosen by cursor side)
- **Alt/Option (hold)** — Temporarily use Eyedropper while held (supports the same Shift behavior)

### Export

- **Cmd+E** — Open the export dialog
- Exports the map as a PNG at configurable scale (25%–400%)
- Optional solid background color (transparent by default)
- Optional tab map export (collision wireframe) as a separate `_tab.png` file with its own scale and background settings

### Map Dimensions

The status bar shows the current map dimensions. Click the dimension label to choose from all valid factor pairs for the tile count — useful for maps where the original width/height is ambiguous.

## Project Structure

```
├── archive/    # .dat archive memory-mapped loader
├── map/        # Map data structures and tile format
├── render/     # Palette, tile atlas, sprite atlas, HPF decoder
└── editor/     # egui application, UI panels, PNG export
```

## Map Format

Each `.map` file is raw binary — 6 bytes per tile (three little-endian `u16` values: ground ID, left wall ID, right wall ID). The editor infers map dimensions from the tile count.
