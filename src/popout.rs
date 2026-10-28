//! The pop-out: once the seaweed catches fire, a bigger copy of the shrimp
//! climbs out of the tray and sits in the corner of the screen, growing as it
//! gets angrier, until you sit up straight (or it stays there all the time, if
//! you ask it to). It never takes focus or shows up in Alt-Tab.

use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, AC_SRC_ALPHA,
    AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS,
};
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::Shell::{
    SHQueryUserNotificationState, QUERY_USER_NOTIFICATION_STATE, QUNS_ACCEPTS_NOTIFICATIONS,
    QUNS_QUIET_TIME,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::settings::wide;
use crate::shrimp::Look;
use crate::sprite::{self, ART_SIZE};
use crate::tray::WM_TRAY;

/// Gap between the pop-out and the edge of the screen, in 96-DPI pixels.
const MARGIN: i32 = 12;

pub struct Popout {
    hwnd: HWND,
    /// What's on screen right now: the look and its size in real pixels.
    shown: Option<(Look, i32)>,
}

impl Popout {
    /// Creates the (hidden) pop-out window. Clicks on it are forwarded to `owner`
    /// exactly as if the tray icon had been clicked.
    pub fn new(owner: HWND, instance: HINSTANCE) -> Option<Popout> {
        unsafe {
            let class_name = wide("ShrimplePopout");
            let class = WNDCLASSW {
                lpfnWndProc: Some(popout_proc),
                hInstance: instance,
                hCursor: LoadCursorW(null_mut(), IDC_HAND),
                lpszClassName: class_name.as_ptr(),
                ..std::mem::zeroed()
            };
            RegisterClassW(&class);

            // Owned by the hidden tray window, which keeps it off the taskbar.
            let title = wide("Shrimple");
            let (x, y) = corner(0);
            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_POPUP,
                x,
                y,
                0,
                0,
                owner,
                null_mut(),
                instance,
                null(),
            );
            (!hwnd.is_null()).then_some(Popout { hwnd, shown: None })
        }
    }

    /// Shows the shrimp on screen looking like `look`, redrawing only if needed.
    pub fn show(&mut self, look: Look) {
        let dpi = unsafe { GetDpiForWindow(self.hwnd) }.max(96) as i32;
        // Each art pixel is drawn as a whole number of real pixels so it stays
        // crisp, and the shrimp grows as it gets angrier.
        let scale = ((3 + look.anger as i32) * dpi / 96).max(1);
        let size = ART_SIZE as i32 * scale;
        if self.shown == Some((look, size)) {
            return;
        }
        let was_hidden = self.shown.is_none();
        self.draw(look, size, dpi);
        self.shown = Some((look, size));
        if was_hidden {
            unsafe {
                ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
                SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
    }

    pub fn hide(&mut self) {
        if self.shown.take().is_some() {
            unsafe { ShowWindow(self.hwnd, SW_HIDE) };
        }
    }

    /// Forget what's on screen so the next `show` redraws and re-places it
    /// (e.g. after the display layout or DPI changes).
    pub fn invalidate(&mut self) {
        if let Some((look, _)) = self.shown {
            self.shown = Some((look, 0));
        }
    }

    /// Paints `look` into the layered window at `size`x`size` pixels, tucked
    /// into the bottom-right corner of the work area, just above the tray.
    fn draw(&self, look: Look, size: i32, dpi: i32) {
        let pixels = sprite::render(look, size as usize);
        let margin = MARGIN * dpi / 96;
        let (right, bottom) = corner(margin);
        let pos = POINT {
            x: right - size,
            y: bottom - size,
        };
        let extent = SIZE { cx: size, cy: size };
        unsafe {
            let mut header: BITMAPINFO = std::mem::zeroed();
            header.bmiHeader = BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: size,
                biHeight: -size, // negative = top-down rows
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..std::mem::zeroed()
            };
            let mut bits: *mut std::ffi::c_void = null_mut();
            let bitmap = CreateDIBSection(
                null_mut(),
                &header,
                DIB_RGB_COLORS,
                &mut bits,
                null_mut(),
                0,
            );
            if bitmap.is_null() || bits.is_null() {
                return;
            }
            let count = (size * size) as usize;
            let dest = std::slice::from_raw_parts_mut(bits as *mut u8, count * 4);
            // Layered windows want premultiplied BGRA.
            for (px, &[r, g, b, a]) in dest.chunks_exact_mut(4).zip(&pixels) {
                let pre = |c: u8| ((c as u16 * a as u16 + 127) / 255) as u8;
                px.copy_from_slice(&[pre(b), pre(g), pre(r), a]);
            }

            let dc = CreateCompatibleDC(null_mut());
            let old = SelectObject(dc, bitmap);
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let origin = POINT { x: 0, y: 0 };
            UpdateLayeredWindow(
                self.hwnd,
                null_mut(),
                &pos,
                &extent,
                dc,
                &origin,
                0,
                &blend,
                ULW_ALPHA,
            );
            SelectObject(dc, old);
            DeleteDC(dc);
            DeleteObject(bitmap);
        }
    }
}

impl Drop for Popout {
    fn drop(&mut self) {
        unsafe { DestroyWindow(self.hwnd) };
    }
}

/// True when popping up would be rude: a full-screen app, game or
/// presentation is running, or the screen is locked.
pub fn screen_is_busy() -> bool {
    let mut state: QUERY_USER_NOTIFICATION_STATE = 0;
    let ok = unsafe { SHQueryUserNotificationState(&mut state) } >= 0;
    ok && state != QUNS_ACCEPTS_NOTIFICATIONS && state != QUNS_QUIET_TIME
}

/// The bottom-right corner of the primary monitor's work area (the screen minus
/// the taskbar), pulled in by `margin` pixels.
fn corner(margin: i32) -> (i32, i32) {
    let mut area = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    unsafe {
        if SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut area as *mut RECT as *mut _, 0) == 0 {
            area.right = GetSystemMetrics(SM_CXSCREEN);
            area.bottom = GetSystemMetrics(SM_CYSCREEN);
        }
    }
    (area.right - margin, area.bottom - margin)
}

unsafe extern "system" fn popout_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        // Clicking the shrimp shouldn't pull focus from whatever you're doing.
        WM_MOUSEACTIVATE => MA_NOACTIVATE as LRESULT,
        WM_LBUTTONUP | WM_RBUTTONUP => {
            // Hand the click to the tray window, which already knows what a
            // left-click (acknowledge) and right-click (menu) on the shrimp mean.
            let owner = GetWindow(hwnd, GW_OWNER);
            PostMessageW(owner, WM_TRAY, 0, msg as LPARAM);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
