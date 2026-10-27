//! Hand-drawn 16x16 pixel art for the shrimp, plus a renderer that recolors it
//! by mood and scales it (nearest-neighbor, so it stays crisp) to any icon size.

use crate::shrimp::{Look, Pose, FLAME_ANGER, MAX_ANGER};

pub const ART_SIZE: usize = 16;

// Legend:
//   .  transparent        o  outline           b  body
//   h  highlight          s  shell shading     l  legs
//   a  antennae           e  eye               w  eye shine
const UPRIGHT: [&str; ART_SIZE] = [
    "..a........a....",
    "...a......a.....",
    "....a....a......",
    ".....ooooo......",
    ".oooobbbbbo.....",
    "...owebbbbho....",
    "...oeebbbbho....",
    "...obbbbbbbo....",
    "....osssssso....",
    "...lobbbbbho....",
    "....ossssso.....",
    "...lobbbbho.....",
    "....osssso......",
    ".....obbo.......",
    "....osooso......",
    "...osso.osso....",
];

const SLOUCH: [&str; ART_SIZE] = [
    "................",
    ".a..............",
    "..a...ooooo.....",
    "...aoobbbbboo...",
    "...obbbbbbbbbo..",
    ".oobbbbbbsbbho..",
    "..owebbbossbbho.",
    "..oeebbo..osbho.",
    "...ooooo..osbbo.",
    "....l.l..osbbho.",
    ".........osbbo..",
    "........osbbbo..",
    ".......osbbbo...",
    ".......obbbo....",
    "......osooso....",
    ".....osso.osso..",
];

const CURLED: [&str; ART_SIZE] = [
    "................",
    "................",
    "................",
    "..a.............",
    "...a..ooooo.....",
    "....oobbbbboo...",
    ".oooobbbbbsbbo..",
    "..owebbbboosbho.",
    "..oeebbboo.osbho",
    "...ooooo...osbbo",
    "....l.l...osbbho",
    "..oo....ooosbbo.",
    ".osso.oosbbbbo..",
    ".ossssobbbbboo..",
    "..oooooooooo....",
    "................",
];

// The world behind the shrimp, so it stands out against any taskbar. The top
// right corner stays clear for the vein and sparkle.
//   d  sand               p  pebble
//   g  seaweed            k  seaweed shadow
//   r  flame edge         f  flame           y  flame core
const SEABED: [&str; ART_SIZE] = [
    "................",
    "................",
    "................",
    "................",
    "................",
    "..............g.",
    ".............gk.",
    ".............g..",
    "............kg..",
    ".............gg.",
    "..............gk",
    "g.............g.",
    "gk...........gk.",
    ".g...........g..",
    "kgd.......d.kgdd",
    "ddddpdddddddpddd",
];

const FLAMES: [[&str; ART_SIZE]; 2] = [
    [
        "......r.........",
        "......rr........",
        ".....rfr..r.....",
        "r....rfyr.rr....",
        "rr..rfyfr.rfr...",
        "rfr.rfyfrrffr..r",
        "rfr..........rfr",
        "ryfr..........fr",
        "ryfr............",
        "rfyr............",
        "rfyr............",
        "ryfr............",
        "rffr............",
        "rffr.........r..",
        "prfr........rfrr",
        "pprrrrrrrrrrrrpp",
    ],
    [
        ".......r........",
        "......rr........",
        "......rfr.r.....",
        ".....rfyr.rr....",
        "r....rfyfrrfr...",
        "rr..rfyyfrffr.r.",
        "rfr..........rrr",
        "rffr..........ff",
        "ryfr............",
        "ryfr............",
        "rfyr............",
        "rfyr............",
        "ryfr............",
        "rffr..........r.",
        "prfr.........rfr",
        "pprrrrrrrrrrrrpp",
    ],
];

/// Anger vein, drawn in the top-right corner when the shrimp is really mad.
const VEIN: [&str; 5] = [
    ".x.x.", //
    "xx.xx", //
    ".....", //
    "xx.xx", //
    ".x.x.",
];

/// Sparkle, drawn in the top-right corner right after being acknowledged.
const SPARKLE: [&str; 5] = [
    "..*..", //
    "..*..", //
    "**.**", //
    "..*..", //
    "..*..",
];

type Rgba = [u8; 4];

/// Calm shrimp colors and fully-furious colors, per legend character.
const PALETTE: &[(char, Rgba, Rgba)] = &[
    ('o', [92, 32, 30, 255], [70, 6, 6, 255]),
    ('b', [255, 142, 108, 255], [228, 32, 26, 255]),
    ('h', [255, 204, 172, 255], [255, 112, 92, 255]),
    ('s', [222, 98, 78, 255], [168, 14, 14, 255]),
    ('l', [200, 82, 66, 255], [150, 14, 14, 255]),
    ('a', [150, 62, 52, 255], [120, 18, 18, 255]),
    ('e', [24, 12, 12, 255], [24, 6, 6, 255]),
    ('w', [255, 255, 255, 255], [255, 220, 80, 255]),
    ('x', [235, 24, 24, 255], [235, 24, 24, 255]),
    ('*', [255, 236, 120, 255], [255, 236, 120, 255]),
    ('d', [222, 184, 112, 255], [92, 70, 58, 255]),
    ('p', [168, 138, 92, 255], [56, 40, 36, 255]),
    ('g', [64, 176, 92, 255], [150, 140, 60, 255]),
    ('k', [30, 112, 64, 255], [100, 84, 40, 255]),
    ('r', [214, 52, 20, 255], [214, 52, 20, 255]),
    ('f', [255, 140, 24, 255], [255, 140, 24, 255]),
    ('y', [255, 226, 80, 255], [255, 226, 80, 255]),
];

fn color(c: char, anger: f32) -> Rgba {
    let Some(&(_, calm, mad)) = PALETTE.iter().find(|(k, _, _)| *k == c) else {
        return [0; 4];
    };
    let mut out = [0; 4];
    for i in 0..4 {
        out[i] = (calm[i] as f32 + (mad[i] as f32 - calm[i] as f32) * anger).round() as u8;
    }
    out
}

fn grid_of(art: &[&str; ART_SIZE]) -> [[char; ART_SIZE]; ART_SIZE] {
    let mut grid = [['.'; ART_SIZE]; ART_SIZE];
    for (y, row) in art.iter().enumerate() {
        for (x, c) in row.chars().enumerate() {
            grid[y][x] = c;
        }
    }
    grid
}

/// Builds the 16x16 grid of legend characters for a given look.
fn compose(look: Look) -> [[char; ART_SIZE]; ART_SIZE] {
    let art = match look.pose {
        Pose::Upright => &UPRIGHT,
        Pose::Slouch => &SLOUCH,
        Pose::Curled => &CURLED,
    };
    let mut shrimp = grid_of(art);

    if look.anger >= 2 {
        // Angry glare: the top half of the eye gets a heavy lid.
        if let Some((wx, wy)) = find(&shrimp, 'w') {
            shrimp[wy][wx] = 'o';
            shrimp[wy][wx + 1] = 'o';
            shrimp[wy + 1][wx] = 'w';
        }
    }

    // Only the shrimp hops; the scenery stays put.
    let shift = (look.shake.unsigned_abs() as usize).min(ART_SIZE);
    for row in shrimp.iter_mut().filter(|_| shift > 0) {
        if look.shake > 0 {
            row.rotate_right(shift);
            row[..shift].fill('.');
        } else {
            row.rotate_left(shift);
            row[ART_SIZE - shift..].fill('.');
        }
    }

    let mut grid = if look.anger >= FLAME_ANGER {
        grid_of(&FLAMES[look.flicker as usize])
    } else {
        grid_of(&SEABED)
    };
    for (row, shrimp_row) in grid.iter_mut().zip(&shrimp) {
        for (c, &s) in row.iter_mut().zip(shrimp_row) {
            if s != '.' {
                *c = s;
            }
        }
    }

    let corner = if look.sparkle {
        Some(&SPARKLE)
    } else if look.anger >= MAX_ANGER - 2 {
        Some(&VEIN)
    } else {
        None
    };
    if let Some(stamp) = corner {
        for (y, row) in stamp.iter().enumerate() {
            for (x, c) in row.chars().enumerate() {
                if c != '.' {
                    grid[y][ART_SIZE - 5 + x] = c;
                }
            }
        }
    }
    grid
}

fn find(grid: &[[char; ART_SIZE]; ART_SIZE], c: char) -> Option<(usize, usize)> {
    (0..ART_SIZE)
        .flat_map(|y| (0..ART_SIZE).map(move |x| (x, y)))
        .find(|&(x, y)| grid[y][x] == c)
}

/// Renders the shrimp as top-down, straight-alpha RGBA pixels at `size`x`size`.
pub fn render(look: Look, size: usize) -> Vec<Rgba> {
    let grid = compose(look);
    let anger = look.anger as f32 / MAX_ANGER as f32;
    let mut pixels = Vec::with_capacity(size * size);
    for y in 0..size {
        let ay = y * ART_SIZE / size;
        for x in 0..size {
            let ax = x * ART_SIZE / size;
            pixels.push(color(grid[ay][ax], anger));
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn art_is_well_formed() {
        for art in [&UPRIGHT, &SLOUCH, &CURLED, &SEABED, &FLAMES[0], &FLAMES[1]] {
            for row in art.iter() {
                assert_eq!(row.chars().count(), ART_SIZE, "{row:?}");
                for c in row.chars() {
                    assert!(c == '.' || PALETTE.iter().any(|p| p.0 == c), "{c:?}");
                }
            }
        }
        for art in [&UPRIGHT, &SLOUCH, &CURLED] {
            assert!(art.concat().contains('w'), "every pose needs an eye");
        }
    }

    #[test]
    fn renders_every_look_at_common_sizes() {
        for pose in [Pose::Upright, Pose::Slouch, Pose::Curled] {
            for anger in 0..=MAX_ANGER {
                for (shake, flicker) in [(0, false), (1, true)] {
                    let look = Look {
                        pose,
                        anger,
                        shake,
                        sparkle: false,
                        flicker,
                    };
                    for size in [16, 20, 24, 32, 48] {
                        assert_eq!(render(look, size).len(), size * size);
                    }
                }
            }
        }
    }

    #[test]
    fn gets_redder_when_angry() {
        let calm = color('b', 0.0);
        let mad = color('b', 1.0);
        assert!(mad[1] < calm[1] && mad[2] < calm[2]);
    }
}
