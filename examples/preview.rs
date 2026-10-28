//! Renders a contact sheet of every shrimp mood, on a light and a dark taskbar,
//! to `shrimp-preview.png`, so the art can be checked on any platform:
//! `cargo run --example preview`.

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
        flicker: true,
    });

    // Windows 11's light and dark taskbar colors.
    const TASKBARS: [[u8; 4]; 2] = [[0xEE, 0xEE, 0xEE, 255], [0x20, 0x20, 0x20, 255]];
    let width = looks.len() * (CELL + GAP) + GAP;
    let row_height = CELL + 2 * GAP;
    let height = TASKBARS.len() * row_height;
    let mut img = vec![0u8; width * height * 4];
    for (t, taskbar) in TASKBARS.iter().enumerate() {
        for px in
            img[t * row_height * width * 4..(t + 1) * row_height * width * 4].chunks_exact_mut(4)
        {
            px.copy_from_slice(taskbar);
        }
        for (i, look) in looks.iter().enumerate() {
            let pixels = sprite::render(*look, CELL);
            for y in 0..CELL {
                for x in 0..CELL {
                    let [r, g, b, a] = pixels[y * CELL + x];
                    let at = ((t * row_height + GAP + y) * width + GAP + i * (CELL + GAP) + x) * 4;
                    for (dst, src) in img[at..at + 3].iter_mut().zip([r, g, b]) {
                        let blended =
                            (src as u32 * a as u32 + *dst as u32 * (255 - a as u32)) / 255;
                        *dst = blended as u8;
                    }
                }
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
