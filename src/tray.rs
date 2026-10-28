//! The Win32 side: a hidden window that owns the tray icon, ticks a timer to
//! keep the shrimp's look up to date, and handles clicks and the menu.

use std::cell::RefCell;
use std::ptr::{null, null_mut};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{
    GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, POINT, WPARAM,
};
use windows_sys::Win32::Graphics::Gdi::{
    CreateBitmap, CreateDIBSection, DeleteObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::RemoteDesktop::{
    WTSRegisterSessionNotification, WTSUnRegisterSessionNotification, NOTIFY_FOR_THIS_SESSION,
};
use windows_sys::Win32::System::SystemInformation::GetTickCount;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::popout::{self, Popout};
use crate::settings::{self, wide};
use crate::shrimp::{Look, Shrimp};
use crate::sprite;

pub const WM_TRAY: u32 = WM_APP + 1;
const TICK_TIMER: usize = 1;
const TICK_MS: u32 = 500;
const ICON_ID: u32 = 1;

/// No keyboard/mouse input for this long means you've stepped away; the
/// countdown restarts so the shrimp isn't furious when you get back.
const AWAY_AFTER: Duration = Duration::from_secs(5 * 60);

const INTERVAL_CHOICES: [u32; 8] = [5, 10, 15, 20, 30, 45, 60, 90];

const CMD_ACK: usize = 100;
const CMD_PAUSE: usize = 101;
const CMD_AUTOSTART: usize = 102;
const CMD_QUIT: usize = 103;
const CMD_POP_OUT: usize = 104;
const CMD_ALWAYS_SHOWN: usize = 105;
const CMD_INTERVAL_BASE: usize = 200;

// Session-change codes for WM_WTSSESSION_CHANGE and power events for WM_POWERBROADCAST.
const WTS_SESSION_UNLOCK: usize = 0x8;
const PBT_APMRESUMESUSPEND: usize = 0x7;
const PBT_APMRESUMEAUTOMATIC: usize = 0x12;

struct App {
    hwnd: HWND,
    shrimp: Shrimp,
    taskbar_created: u32,
    icon: HICON,
    icon_size: i32,
    shown_look: Option<Look>,
    shown_tip: String,
    /// The big on-screen shrimp that appears when you ignore the tray one.
    popout: Option<Popout>,
    pops_out: bool,
    /// Keep the big shrimp on screen even while it's calm.
    always_shown: bool,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

/// Runs `f` on the app state. Returns `None` if there is no app yet, or if the
/// state is already in use further up the stack (Win32 calls can re-enter the
/// window procedure); such messages just get default handling.
fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|app| app.try_borrow_mut().ok()?.as_mut().map(f))
}

pub fn run() {
    unsafe {
        // Only one shrimp at a time.
        let mutex_name = wide("Local\\ShrimplePostureShrimp");
        CreateMutexW(null(), 0, mutex_name.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return;
        }

        // Ask for real pixel sizes so the icon is drawn crisply on high-DPI screens.
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

        let instance = GetModuleHandleW(null());
        let class_name = wide("ShrimpleTrayWindow");
        let class = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: instance,
            lpszClassName: class_name.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassW(&class);

        // A real (but never shown) top-level window, rather than a message-only
        // one, so we receive broadcasts like "TaskbarCreated" and power events.
        let title = wide("Shrimple");
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            instance,
            null(),
        );
        if hwnd.is_null() {
            return;
        }

        let interval = Duration::from_secs(settings::interval_minutes() as u64 * 60);
        let mut shrimp = Shrimp::new(interval, Instant::now());
        shrimp.reset_quietly(Instant::now());
        let app = App {
            hwnd,
            shrimp,
            taskbar_created: RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()),
            icon: null_mut(),
            icon_size: 0,
            shown_look: None,
            shown_tip: String::new(),
            popout: Popout::new(hwnd, instance),
            pops_out: settings::pops_out(),
            always_shown: settings::always_shown(),
        };
        APP.with(|slot| *slot.borrow_mut() = Some(app));

        with_app(|app| app.add_icon());
        WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION);
        SetTimer(hwnd, TICK_TIMER, TICK_MS, None);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        WTSUnRegisterSessionNotification(hwnd);
        with_app(|app| app.remove_icon());
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_TRAY && matches!(lparam as u32, WM_RBUTTONUP | WM_CONTEXTMENU) {
        // The menu runs its own message loop, so show it without holding the app state.
        if let Some(menu) = with_app(|app| app.build_menu()) {
            let cmd = track_menu(hwnd, menu);
            with_app(|app| app.run_command(cmd));
        }
        return 0;
    }
    let handled = with_app(|app| app.handle(msg, wparam, lparam)).flatten();
    match handled {
        Some(result) => result,
        None => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

impl App {
    fn handle(&mut self, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
        match msg {
            WM_TIMER if wparam == TICK_TIMER => self.tick(),
            WM_TRAY if lparam as u32 == WM_LBUTTONUP => {
                self.shrimp.acknowledge(Instant::now());
                self.refresh();
            }
            WM_TRAY => {}
            WM_WTSSESSION_CHANGE if wparam == WTS_SESSION_UNLOCK => self.welcome_back(),
            WM_POWERBROADCAST
                if wparam == PBT_APMRESUMESUSPEND || wparam == PBT_APMRESUMEAUTOMATIC =>
            {
                self.welcome_back()
            }
            WM_DISPLAYCHANGE | WM_SETTINGCHANGE | WM_DPICHANGED => {
                // The tray icon size may have changed; redraw at the new size.
                self.shown_look = None;
                if let Some(popout) = &mut self.popout {
                    popout.invalidate();
                }
                self.refresh();
            }
            WM_DESTROY => unsafe { PostQuitMessage(0) },
            m if m == self.taskbar_created && m != 0 => {
                // Explorer restarted, taking our icon with it; put it back.
                self.shown_look = None;
                self.shown_tip.clear();
                self.add_icon();
            }
            _ => return None,
        }
        Some(0)
    }

    fn tick(&mut self) {
        if idle_time() >= AWAY_AFTER && !self.shrimp.paused() {
            // You're away from the keyboard: hopefully stretching. Keep the
            // countdown fresh until you're back.
            self.shrimp.reset_quietly(Instant::now());
        }
        self.refresh();
    }

    fn welcome_back(&mut self) {
        if !self.shrimp.paused() {
            self.shrimp.reset_quietly(Instant::now());
        }
        self.refresh();
    }

    fn base_data(&self) -> NOTIFYICONDATAW {
        let mut data: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
        data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        data.hWnd = self.hwnd;
        data.uID = ICON_ID;
        data
    }

    fn add_icon(&mut self) {
        let mut data = self.base_data();
        data.uFlags = NIF_MESSAGE;
        data.uCallbackMessage = WM_TRAY;
        unsafe { Shell_NotifyIconW(NIM_ADD, &data) };
        self.refresh();
    }

    fn remove_icon(&mut self) {
        let data = self.base_data();
        unsafe {
            Shell_NotifyIconW(NIM_DELETE, &data);
            if !self.icon.is_null() {
                DestroyIcon(self.icon);
                self.icon = null_mut();
            }
        }
    }

    /// Pushes the shrimp's current look and tooltip to the tray, if either changed.
    fn refresh(&mut self) {
        let now = Instant::now();
        let look = self.shrimp.look(now);
        let tip = self.shrimp.tooltip(now);
        let size = unsafe { GetSystemMetrics(SM_CXSMICON) }.max(16);

        let mut data = self.base_data();
        let mut old_icon = null_mut();
        if self.shown_look != Some(look) || self.icon_size != size {
            let icon = make_icon(look, size as usize);
            if !icon.is_null() {
                old_icon = std::mem::replace(&mut self.icon, icon);
                self.icon_size = size;
                self.shown_look = Some(look);
                data.uFlags |= NIF_ICON;
                data.hIcon = icon;
            }
        }
        if self.shown_tip != tip {
            let encoded: Vec<u16> = tip.encode_utf16().take(data.szTip.len() - 1).collect();
            data.szTip[..encoded.len()].copy_from_slice(&encoded);
            data.uFlags |= NIF_TIP;
            self.shown_tip = tip;
        }
        if data.uFlags != 0 {
            data.hIcon = self.icon;
            data.uFlags |= NIF_ICON;
            unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) };
        }
        if !old_icon.is_null() {
            unsafe { DestroyIcon(old_icon) };
        }

        if let Some(popout) = &mut self.popout {
            let wanted = self.always_shown || (self.pops_out && self.shrimp.wants_attention(now));
            if wanted && !popout::screen_is_busy() {
                popout.show(look);
            } else {
                popout.hide();
            }
        }
    }

    fn build_menu(&self) -> HMENU {
        let now = Instant::now();
        let status = if self.shrimp.paused() {
            "Paused".to_string()
        } else if let Some(left) = self.shrimp.remaining(now) {
            let mins = left.as_secs().div_ceil(60);
            format!("Next posture check in {mins} min")
        } else {
            "Posture check time!".to_string()
        };
        let current = self.shrimp.interval().as_secs() / 60;

        unsafe {
            let intervals = CreatePopupMenu();
            for (i, &mins) in INTERVAL_CHOICES.iter().enumerate() {
                let checked = if mins as u64 == current {
                    MF_CHECKED
                } else {
                    0
                };
                let label = if mins < 60 || mins % 60 != 0 {
                    format!("{mins} minutes")
                } else if mins == 60 {
                    "1 hour".to_string()
                } else {
                    format!("{} hours", mins / 60)
                };
                append(
                    intervals,
                    MF_STRING | checked,
                    CMD_INTERVAL_BASE + i,
                    &label,
                );
            }

            let menu = CreatePopupMenu();
            append(menu, MF_STRING | MF_GRAYED, 0, &status);
            AppendMenuW(menu, MF_SEPARATOR, 0, null());
            append(menu, MF_STRING, CMD_ACK, "I sat up straight");
            SetMenuDefaultItem(menu, CMD_ACK as u32, 0);
            append(menu, MF_POPUP, intervals as usize, "Remind me every");
            let paused = if self.shrimp.paused() { MF_CHECKED } else { 0 };
            append(menu, MF_STRING | paused, CMD_PAUSE, "Pause reminders");
            let pops_out = if self.pops_out { MF_CHECKED } else { 0 };
            append(
                menu,
                MF_STRING | pops_out,
                CMD_POP_OUT,
                "Pop out when ignored",
            );
            let always = if self.always_shown { MF_CHECKED } else { 0 };
            append(
                menu,
                MF_STRING | always,
                CMD_ALWAYS_SHOWN,
                "Always show on screen",
            );
            let autostart = if settings::starts_with_windows() {
                MF_CHECKED
            } else {
                0
            };
            append(
                menu,
                MF_STRING | autostart,
                CMD_AUTOSTART,
                "Start with Windows",
            );
            AppendMenuW(menu, MF_SEPARATOR, 0, null());
            append(menu, MF_STRING, CMD_QUIT, "Quit");
            menu
        }
    }

    fn run_command(&mut self, cmd: usize) {
        let now = Instant::now();
        match cmd {
            CMD_ACK => self.shrimp.acknowledge(now),
            CMD_PAUSE => self.shrimp.set_paused(!self.shrimp.paused(), now),
            CMD_POP_OUT => {
                self.pops_out = !self.pops_out;
                settings::set_pops_out(self.pops_out);
            }
            CMD_ALWAYS_SHOWN => {
                self.always_shown = !self.always_shown;
                settings::set_always_shown(self.always_shown);
            }
            CMD_AUTOSTART => settings::set_starts_with_windows(!settings::starts_with_windows()),
            CMD_QUIT => unsafe {
                // Close via the message queue so WM_DESTROY isn't handled re-entrantly.
                PostMessageW(self.hwnd, WM_CLOSE, 0, 0);
                return;
            },
            c if (CMD_INTERVAL_BASE..CMD_INTERVAL_BASE + INTERVAL_CHOICES.len()).contains(&c) => {
                let mins = INTERVAL_CHOICES[c - CMD_INTERVAL_BASE];
                settings::set_interval_minutes(mins);
                self.shrimp
                    .set_interval(Duration::from_secs(mins as u64 * 60), now);
            }
            _ => {}
        }
        self.refresh();
    }
}

unsafe fn append(menu: HMENU, flags: MENU_ITEM_FLAGS, id: usize, label: &str) {
    AppendMenuW(menu, flags, id, wide(label).as_ptr());
}

/// Shows `menu` at the mouse cursor, waits for a choice, and destroys the menu.
/// Returns the chosen command id, or 0 if the menu was dismissed.
unsafe fn track_menu(hwnd: HWND, menu: HMENU) -> usize {
    let mut cursor = POINT { x: 0, y: 0 };
    GetCursorPos(&mut cursor);
    // Required so the menu closes when you click elsewhere.
    SetForegroundWindow(hwnd);
    let cmd = TrackPopupMenu(
        menu,
        TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN,
        cursor.x,
        cursor.y,
        0,
        hwnd,
        null(),
    );
    PostMessageW(hwnd, WM_NULL, 0, 0);
    DestroyMenu(menu); // also destroys the attached submenu
    cmd.max(0) as usize
}

/// How long since the last keyboard or mouse input anywhere in the session.
fn idle_time() -> Duration {
    let mut info = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    unsafe {
        if GetLastInputInfo(&mut info) == 0 {
            return Duration::ZERO;
        }
        Duration::from_millis(GetTickCount().wrapping_sub(info.dwTime) as u64)
    }
}

/// Builds a 32-bit alpha-blended icon of the shrimp at `size`x`size` pixels.
fn make_icon(look: Look, size: usize) -> HICON {
    let pixels = sprite::render(look, size);
    unsafe {
        let mut header: BITMAPINFO = std::mem::zeroed();
        header.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size as i32,
            biHeight: -(size as i32), // negative = top-down rows
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..std::mem::zeroed()
        };
        let mut bits: *mut std::ffi::c_void = null_mut();
        let color = CreateDIBSection(
            null_mut(),
            &header,
            DIB_RGB_COLORS,
            &mut bits,
            null_mut(),
            0,
        );
        if color.is_null() || bits.is_null() {
            return null_mut();
        }
        let dest = std::slice::from_raw_parts_mut(bits as *mut u8, size * size * 4);
        for (px, &[r, g, b, a]) in dest.chunks_exact_mut(4).zip(&pixels) {
            px.copy_from_slice(&[b, g, r, a]);
        }

        // The AND mask is ignored for 32-bit icons with alpha, but must exist.
        let mask_stride = size.div_ceil(16) * 2;
        let mask_bits = vec![0u8; mask_stride * size];
        let mask = CreateBitmap(
            size as i32,
            size as i32,
            1,
            1,
            mask_bits.as_ptr() as *const _,
        );

        let info = ICONINFO {
            fIcon: 1,
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask,
            hbmColor: color,
        };
        let icon = CreateIconIndirect(&info);
        DeleteObject(color);
        DeleteObject(mask);
        icon
    }
}
