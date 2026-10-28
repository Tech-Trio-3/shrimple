<p align="center">
  <img src="./assets/readme/hero.svg" width="100%" alt="Shrimple: a tiny pixel-art shrimp that lives in your Windows system tray and reminds you to sit up straight. No sounds, no pop-ups, just a shrimp.">
</p>

<p align="center">
  <a href="https://github.com/miles-howell/shrimple/actions/workflows/build.yml"><img src="https://github.com/miles-howell/shrimple/actions/workflows/build.yml/badge.svg" alt="Build status"></a>
</p>

Shrimple puts a 16×16 shrimp in its own little patch of sea in your Windows
system tray. When it's time to check your posture, the shrimp slouches. If you
ignore it, it gets steadily angrier. Sit up, click it, and it calms down.

<p align="center">
  <img src="./assets/readme/moods.svg" width="100%" alt="The shrimp's moods, using the real tray icon frames: calm during the countdown, slouching when a check is due, curled up after 20 seconds, glaring by two and a half minutes, on fire around four minutes, furious by eight and a half. Clicking it makes it sparkle and starts the countdown over.">
</p>

- **Calm:** the shrimp stands up straight while the timer counts down.
- **Posture check due:** it slouches, then curls up into a ball.
- **Ignored:** after a minute it starts turning red, then glaring, and the
  seaweed wilts. A few minutes later the seaweed bursts into flames and the
  water boils orange. By about nine minutes it's bright red, has an anger
  vein, and is hopping in place.
- **Click the shrimp** once you've sat up straight. It sparkles briefly and
  starts the countdown over.

## Using it

**Hover** over the shrimp to see how long until the next check.
**Left-click** it when you've sat up straight. **Right-click** for the menu:

| Menu item | What it does |
| --- | --- |
| I sat up straight | Same as left-clicking the shrimp. |
| Remind me every | 5, 10, 15, 20, 30 or 45 minutes, or 1 or 1½ hours (default 30 minutes). Choosing one restarts the countdown. |
| Pause reminders | The shrimp stays calm until you unpause. |
| Start with Windows | Launch Shrimple when you sign in. |
| Quit | Bye, shrimp. |

### It won't nag an empty chair

The countdown restarts on its own when you've clearly stepped away: after 5
minutes with no keyboard or mouse input, when you unlock your PC, and when it
wakes from sleep.

## Get the shrimp

**Download:** every push builds `shrimple.exe` on
[GitHub Actions](https://github.com/miles-howell/shrimple/actions/workflows/build.yml).
Open the latest run and download the `shrimple` artifact.

**Build it yourself:** install [Rust](https://rustup.rs/), then on Windows:

```sh
cargo build --release
```

Run `target\release\shrimple.exe` and look for the shrimp in the tray. You may
need to drag it out of the overflow area (`^`) so it's always visible.

To cross-compile from Linux, install `mingw-w64`, then:

```sh
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

Settings live in the registry under `HKCU\Software\Shrimple`. "Start with
Windows" uses the standard `HKCU\...\CurrentVersion\Run` entry.

## Hacking on the shrimp

| File | What's in it |
| --- | --- |
| `src/sprite.rs` | The 16×16 pixel art as ASCII, plus the calm and furious palettes. The water, seabed and flames are drawn behind the shrimp. |
| `src/shrimp.rs` | Timing for when it curls up and how fast it gets angry (constants at the top). |
| `src/tray.rs` | The Win32 tray icon, menu and timer. |

`cargo run --example preview` renders every mood, on light and dark taskbars,
to `shrimp-preview.png` on any OS. `cargo test` runs the mood and sprite tests.
