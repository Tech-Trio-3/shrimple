//! Embeds `assets/shrimple.ico` as the EXE's icon (Explorer, taskbar, Alt-Tab).

fn main() {
    println!("cargo:rerun-if-changed=assets/shrimple.ico");
    #[cfg(windows)]
    winresource::WindowsResource::new()
        .set_icon("assets/shrimple.ico")
        .compile()
        .expect("embed icon");
}
