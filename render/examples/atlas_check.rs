use std::path::Path;

use archive::AssetPool;
use render::{Palette, TileAtlas};

fn main() {
    let assets = Path::new("assets");
    if !assets.is_dir() {
        eprintln!("Run from workspace root (assets/ directory not found)");
        std::process::exit(1);
    }

    let pool = AssetPool::load(assets).expect("failed to load asset pool");
    println!(
        "AssetPool: {} archives, {} assets",
        pool.archive_count(),
        pool.len()
    );

    // Load palette
    let pal_data = pool.get("legend.pal").expect("legend.pal not found");
    let palette = Palette::from_bytes(pal_data).expect("failed to parse palette");
    println!("\nPalette loaded: 256 colors");
    println!("  index 0 RGBA: {:?}", palette.color(0));
    println!("  index 1 RGBA: {:?}", palette.color(1));
    println!("  index 50 RGBA: {:?}", palette.color(50));

    // Load tile data
    let tile_data = pool.get("TILEA.BMP").expect("TILEA.BMP not found");
    println!("\nTILEA.BMP: {} bytes", tile_data.len());

    // Build atlas
    let atlas = TileAtlas::from_raw(tile_data, &palette, 56, 27).expect("failed to build atlas");
    let (w, h) = atlas.dimensions();
    let mem_mb = (w as f64 * h as f64 * 4.0) / (1024.0 * 1024.0);
    println!(
        "\nAtlas: {}x{} pixels, {} tiles ({} columns), {:.1} MB RGBA",
        w,
        h,
        atlas.tile_count(),
        atlas.columns(),
        mem_mb
    );

    // Validate tile rects
    let checks = [
        (0, (0, 0, 56, 27)),
        (1, (56, 0, 56, 27)),
        (63, (63 * 56, 0, 56, 27)),
        (64, (0, 27, 56, 27)),
    ];
    println!("\nTile rect checks:");
    for (idx, expected) in &checks {
        let rect = atlas.tile_rect(*idx);
        let pass = rect == Some(*expected);
        println!(
            "  tile {:>5}: {:?} {}",
            idx,
            rect,
            if pass { "OK" } else { "MISMATCH" }
        );
    }

    // Check last tile
    let last = atlas.tile_count() - 1;
    println!("  tile {:>5}: {:?}", last, atlas.tile_rect(last));
    println!(
        "  tile {:>5}: {:?} (out of bounds)",
        atlas.tile_count(),
        atlas.tile_rect(atlas.tile_count())
    );

    // Spot-check: count non-transparent pixels in tile 0
    let (tx, ty, tw, th) = atlas.tile_rect(0).unwrap();
    let pixels = atlas.pixels();
    let stride = w * 4;
    let mut opaque = 0u32;
    let mut transparent = 0u32;
    for py in 0..th {
        for px in 0..tw {
            let offset = ((ty + py) * stride + (tx + px) * 4) as usize;
            if pixels[offset + 3] == 0 {
                transparent += 1;
            } else {
                opaque += 1;
            }
        }
    }
    println!(
        "\nTile 0 pixel stats: {} opaque, {} transparent (of {} total)",
        opaque,
        transparent,
        tw * th
    );
}
