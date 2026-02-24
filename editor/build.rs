#[cfg(target_os = "windows")]
fn main() {
    use std::env;
    use std::fs::File;
    use std::path::PathBuf;

    println!("cargo:rerun-if-changed=app-icon.png");

    let source_png = PathBuf::from("app-icon.png");
    let out_dir =
        PathBuf::from(env::var("OUT_DIR").expect("`OUT_DIR` must be set by Cargo build scripts"));
    let generated_ico = out_dir.join("app-icon.ico");

    let image = image::open(&source_png)
        .expect("Failed to open `app-icon.png` for Windows icon embedding")
        .into_rgba8();
    let (width, height) = image.dimensions();

    let icon_image = ico::IconImage::from_rgba_data(width, height, image.into_raw());
    let icon_entry = ico::IconDirEntry::encode(&icon_image)
        .expect("Failed to encode icon image into ICO directory entry");
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
    icon_dir.add_entry(icon_entry);

    let mut ico_file =
        File::create(&generated_ico).expect("Failed to create generated `.ico` file");
    icon_dir
        .write(&mut ico_file)
        .expect("Failed to write generated `.ico` file");

    winresource::WindowsResource::new()
        .set_icon(generated_ico.to_string_lossy().as_ref())
        .compile()
        .expect("Failed to embed Windows application icon resource");
}

#[cfg(not(target_os = "windows"))]
fn main() {
    println!("cargo:rerun-if-changed=app-icon.png");
}
