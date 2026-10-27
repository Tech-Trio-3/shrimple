//! Renders a contact sheet of every shrimp mood to `shrimp-preview.png`, so the
//! art can be checked on any platform: `cargo run --example preview`.

#[path = "../src/shrimp.rs"]
#[allow(dead_code)]
mod shrimp;
#[path = "../src/sprite.rs"]
#[allow(dead_code)]
mod sprite;

use shrimp::{Look, Pose, MAX_ANGER};

fn main() {
    const CELL: usize = 64;
    const GAP: usize = 8;
    let mut looks = vec![
        Look::CALM,
        Look {
            sparkle: true,
            ..Look::CALM
        },
        Look {
            pose: Pose::Slouch,
            ..Look::CALM
        },
    ];
    looks.extend((0..=MAX_ANGER).map(|anger| Look {
        pose: Pose::Curled,
        anger,
        ..Look::CALM
    }));
    looks.push(Look {
        pose: Pose::Curled,
        anger: MAX_ANGER,
        shake: 1,
        sparkle: false,
    });

    let width = looks.len() * (CELL + GAP) + GAP;
    let height = CELL + 2 * GAP;
    let mut img = vec![0xEEu8; width * height * 4];
    for (i, look) in looks.iter().enumerate() {
        let pixels = sprite::render(*look, CELL);
        for y in 0..CELL {
            for x in 0..CELL {
                let [r, g, b, a] = pixels[y * CELL + x];
                if a == 0 {
                    continue;
                }
                let at = ((GAP + y) * width + GAP + i * (CELL + GAP) + x) * 4;
                img[at..at + 4].copy_from_slice(&[r, g, b, 255]);
            }
        }
    }

    let file = std::fs::File::create("shrimp-preview.png").expect("create shrimp-preview.png");
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&img)
        .unwrap();
    println!("wrote shrimp-preview.png");
}
