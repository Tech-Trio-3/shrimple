//! Writes the calm shrimp to `assets/shrimple.ico`, the icon embedded in the EXE by
//! `build.rs`: `cargo run --example make_icon`.

#[path = "../src/shrimp.rs"]
#[allow(dead_code)]
mod shrimp;
#[path = "../src/sprite.rs"]
#[allow(dead_code)]
mod sprite;

use shrimp::Look;

const SIZES: [usize; 4] = [16, 32, 48, 256];

fn png(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size * size * 4);
    for [r, g, b, a] in sprite::render(Look::CALM, size) {
        data.extend_from_slice(&[r, g, b, a]);
    }
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, size as u32, size as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .and_then(|mut writer| writer.write_image_data(&data))
        .expect("encode png");
    out
}

fn main() {
    let images: Vec<Vec<u8>> = SIZES.iter().map(|&size| png(size)).collect();
    let mut ico = Vec::new();
    ico.extend_from_slice(&[0, 0, 1, 0]);
    ico.extend_from_slice(&(SIZES.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * SIZES.len();
    for (&size, image) in SIZES.iter().zip(&images) {
        // A stored size of 0 means 256.
        let dim = if size >= 256 { 0 } else { size as u8 };
        ico.extend_from_slice(&[dim, dim, 0, 0, 1, 0, 32, 0]);
        ico.extend_from_slice(&(image.len() as u32).to_le_bytes());
        ico.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += image.len();
    }
    for image in &images {
        ico.extend_from_slice(image);
    }
    std::fs::write("assets/shrimple.ico", ico).expect("write assets/shrimple.ico");
}
