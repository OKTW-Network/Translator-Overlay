//! Render `app-icon-small.svg` (16–24 px) and `app-icon.svg` (30 px and up) into a
//! multi-size `app.ico` covering every Win32 size Windows 11 requests.
//!
//! Usage: `cargo run --release -- <icon-dir> [png-preview-dir]`

use std::{env, fs, path::PathBuf};

use resvg::{tiny_skia, usvg};

const SMALL: [u32; 3] = [16, 20, 24];
const LARGE: [u32; 11] = [30, 32, 36, 40, 48, 60, 64, 72, 80, 96, 256];

fn main() {
    let mut args = env::args_os().skip(1).map(PathBuf::from);
    let dir = args.next().expect("usage: icon-gen <icon-dir> [png-preview-dir]");
    let preview = args.next();

    let load = |name: &str| {
        let data = fs::read(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        usvg::Tree::from_data(&data, &usvg::Options::default()).unwrap_or_else(|e| panic!("{name}: {e}"))
    };
    let small = load("app-icon-small.svg");
    let large = load("app-icon.svg");

    let mut icon = ico::IconDir::new(ico::ResourceType::Icon);
    for (tree, size) in SMALL.map(|s| (&small, s)).into_iter().chain(LARGE.map(|s| (&large, s))) {
        let mut pixmap = tiny_skia::Pixmap::new(size, size).unwrap();
        let scale = size as f32 / tree.size().width();
        resvg::render(tree, tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
        let png = pixmap.encode_png().unwrap();
        if let Some(out) = &preview {
            fs::create_dir_all(out).unwrap();
            fs::write(out.join(format!("app-{size}.png")), &png).unwrap();
        }
        let image = ico::IconImage::read_png(png.as_slice()).unwrap();
        icon.add_entry(ico::IconDirEntry::encode(&image).unwrap());
    }
    icon.write(fs::File::create(dir.join("app.ico")).unwrap()).unwrap();
}
