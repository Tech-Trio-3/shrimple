//! Shrimple: a pixel-art shrimp that lives in the Windows system tray and
//! eyes you suspiciously (then gets increasingly red and cranky) when it's time
//! to check your posture. Click it to tell it you've sat up straight.
//! 10/28/20026

#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg_attr(not(windows), allow(dead_code))]
mod shrimp;
#[cfg_attr(not(windows), allow(dead_code))]
mod sprite;

#[cfg(windows)]
mod settings;
#[cfg(windows)]
mod tray;

fn main() {
    #[cfg(windows)]
    tray::run();

    #[cfg(not(windows))]
    eprintln!(
        "Shrimple is a Windows system tray app. \
         On other platforms, try `cargo run --example preview` to render the sprites."
    );
}
