// Helper Win32 dan proses latar: baterai, jam, kursor, volume, mic, kecerahan,
// pintasan global, deteksi rapat, riwayat clipboard, suara adzan.

use crate::Msg;
use eframe::egui;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use windows::core::PCWSTR;
use windows::Win32::{
    Foundation::{BOOL, HMODULE, HWND, LPARAM, POINT},
    Media::Audio::{
        eCapture, eConsole, eRender, EDataFlow, Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator,
        MMDeviceEnumerator,
    },
    System::{
        Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED},
        Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS},
        Registry::{RegDeleteKeyValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ},
        SystemInformation::GetLocalTime,
    },
    UI::{
        Shell::ShellExecuteW,
        Input::KeyboardAndMouse::{
            keybd_event, RegisterHotKey, UnregisterHotKey, KEYEVENTF_KEYUP, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN,
        },
        WindowsAndMessaging::{
            EnumWindows, GetCursorPos, GetSystemMetrics, GetWindowTextW, IsWindowVisible, MessageBeep, PeekMessageW, MB_ICONEXCLAMATION,
            MSG, PM_REMOVE, SM_CXSCREEN, SW_SHOWNORMAL, WM_HOTKEY,
        },
    },
};


// ---------------- waktu, baterai, kursor ----------------

#[derive(Clone, Copy)]
pub struct Dt {
    pub y: u32,
    pub mo: u32,
    pub d: u32,
    pub h: u32,
    pub mi: u32,
}

impl Dt {
    pub fn ymd(&self) -> u32 {
        self.y * 10000 + self.mo * 100 + self.d
    }
    pub fn minutes(&self) -> u32 {
        self.h * 60 + self.mi
    }
}

pub fn local_dt() -> Dt {
    let t = unsafe { GetLocalTime() };
    Dt {
        y: t.wYear as u32,
        mo: t.wMonth as u32,
        d: t.wDay as u32,
        h: t.wHour as u32,
        mi: t.wMinute as u32,
    }
}

#[derive(Clone, Copy)]
pub struct Battery {
    pub percent: u8,
    pub charging: bool,
    pub has_battery: bool,
}

pub fn battery() -> Battery {
    let mut s = SYSTEM_POWER_STATUS::default();
    unsafe {
        let _ = GetSystemPowerStatus(&mut s);
    }
    Battery {
        percent: if s.BatteryLifePercent <= 100 { s.BatteryLifePercent } else { 0 },
        charging: s.ACLineStatus == 1 || (s.BatteryFlag & 8) != 0,
        has_battery: s.BatteryFlag != 128 && s.BatteryLifePercent <= 100,
    }
}

/// Posisi kursor dalam piksel fisik layar.
pub fn cursor_pos() -> Option<(f32, f32)> {
    let mut p = POINT::default();
    unsafe { GetCursorPos(&mut p).ok()? };
    Some((p.x as f32, p.y as f32))
}

/// Lebar monitor utama dalam piksel fisik.
pub fn screen_width_px() -> f32 {
    unsafe { GetSystemMetrics(SM_CXSCREEN) as f32 }
}

// ---------------- menjalankan proses / membuka sesuatu ----------------

/// Buka URI / file / folder lewat shell Windows (ShellExecute).
pub fn open_uri(target: &str) {
    let w = wide(target);
    unsafe {
        let _ = ShellExecuteW(HWND::default(), PCWSTR(wide("open").as_ptr()), PCWSTR(w.as_ptr()), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL);
    }
}

/// Buka overlay tangkap layar bawaan Windows (Snipping Tool).
pub fn screenshot() {
    open_uri("ms-screenclip:");
}

/// Rekam layar lewat Xbox Game Bar: Win + Alt + R.
pub fn record_shortcut() {
    unsafe {
        keybd_event(0x5B, 0, Default::default(), 0); // Win
        keybd_event(0x12, 0, Default::default(), 0); // Alt
        keybd_event(0x52, 0, Default::default(), 0); // R
        keybd_event(0x52, 0, KEYEVENTF_KEYUP, 0);
        keybd_event(0x12, 0, KEYEVENTF_KEYUP, 0);
        keybd_event(0x5B, 0, KEYEVENTF_KEYUP, 0);
    }
}

/// Tangkap layar penuh: Win + PrintScreen (disimpan Windows ke Gambar\\Screenshots).
pub fn screenshot_full() {
    unsafe {
        keybd_event(0x5B, 0, Default::default(), 0);
        keybd_event(0x2C, 0, Default::default(), 0);
        keybd_event(0x2C, 0, KEYEVENTF_KEYUP, 0);
        keybd_event(0x5B, 0, KEYEVENTF_KEYUP, 0);
    }
}

/// Tangkap jendela aktif ke clipboard: Alt + PrintScreen.
pub fn screenshot_window() {
    unsafe {
        keybd_event(0x12, 0, Default::default(), 0);
        keybd_event(0x2C, 0, Default::default(), 0);
        keybd_event(0x2C, 0, KEYEVENTF_KEYUP, 0);
        keybd_event(0x12, 0, KEYEVENTF_KEYUP, 0);
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Mulai bersama Windows (HKCU\...\Run) lewat API registry.
pub fn set_autostart(on: bool) {
    let sub = wide(r"Software\Microsoft\Windows\CurrentVersion\Run");
    let name = wide("RafLand");
    unsafe {
        if on {
            if let Ok(exe) = std::env::current_exe() {
                let val = wide(&format!("\"{}\"", exe.display()));
                let _ = RegSetKeyValueW(
                    HKEY_CURRENT_USER,
                    PCWSTR(sub.as_ptr()),
                    PCWSTR(name.as_ptr()),
                    REG_SZ.0,
                    Some(val.as_ptr() as *const std::ffi::c_void),
                    (val.len() * 2) as u32,
                );
            }
        } else {
            let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, PCWSTR(sub.as_ptr()), PCWSTR(name.as_ptr()));
        }
    }
}

/// Mainkan adzan: pakai %APPDATA%\RafLand\adzan.wav bila ada, kalau tidak bunyi sistem.
pub fn play_adzan() {
    let wav = crate::store::dir().join("adzan.wav");
    unsafe {
        if wav.exists() {
            let w = wide(&wav.display().to_string());
            let _ = PlaySoundW(PCWSTR(w.as_ptr()), HMODULE::default(), SND_FILENAME | SND_ASYNC);
        } else {
            let _ = MessageBeep(MB_ICONEXCLAMATION);
        }
    }
}

// ---------------- volume & mic (Core Audio) ----------------

fn endpoint(flow: EDataFlow) -> Option<IAudioEndpointVolume> {
    thread_local! {
        static INIT: std::cell::Cell<bool> = std::cell::Cell::new(false);
    }
    unsafe {
        INIT.with(|i| {
            if !i.get() {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                i.set(true);
            }
        });
        let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
        let dev = en.GetDefaultAudioEndpoint(flow, eConsole).ok()?;
        dev.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None).ok()
    }
}

pub fn set_volume(percent: u8) {
    if let Some(v) = endpoint(eRender) {
        unsafe {
            let _ = v.SetMasterVolumeLevelScalar(percent.min(100) as f32 / 100.0, std::ptr::null());
        }
    }
}

pub fn get_volume() -> Option<u8> {
    let v = endpoint(eRender)?;
    unsafe { v.GetMasterVolumeLevelScalar().ok().map(|f| (f * 100.0).round() as u8) }
}

/// Status bisu mikrofon bawaan (perangkat rekam default).
pub fn mic_muted() -> Option<bool> {
    let v = endpoint(eCapture)?;
    unsafe { v.GetMute().ok().map(|b| b.as_bool()) }
}

pub fn set_mic_muted(muted: bool) {
    if let Some(v) = endpoint(eCapture) {
        unsafe {
            let _ = v.SetMute(BOOL::from(muted), std::ptr::null());
        }
    }
}

// ---------------- deteksi rapat ----------------

unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let list = &mut *(lparam.0 as *mut Vec<String>);
    if IsWindowVisible(hwnd).as_bool() {
        let mut buf = [0u16; 256];
        let n = GetWindowTextW(hwnd, &mut buf);
        if n > 0 {
            list.push(String::from_utf16_lossy(&buf[..n as usize]));
        }
    }
    BOOL(1)
}

fn window_titles() -> Vec<String> {
    let mut list: Vec<String> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(enum_cb), LPARAM(&mut list as *mut Vec<String> as isize));
    }
    list
}

/// Heuristik berdasarkan judul jendela. Telepon WhatsApp tidak dihitung rapat.
/// Rapat di browser hanya terdeteksi saat tab rapatnya yang aktif.
fn in_meeting(ignore: &[String]) -> bool {
    window_titles().iter().any(|t| {
        let l = t.to_lowercase();
        if l.contains("whatsapp") || ignore.iter().any(|k| l.contains(k.as_str())) {
            return false;
        }
        l.contains("zoom meeting")
            || l.starts_with("meet - ")
            || l.contains("meet.google.com")
            || (l.contains("microsoft teams") && (l.contains("meeting") || l.contains("call")))
    })
}

pub fn spawn_meeting_watcher(tx: Sender<Msg>, ctx: egui::Context, ignore: Arc<Mutex<Vec<String>>>) {
    thread::spawn(move || {
        let mut last = false;
        loop {
            thread::sleep(Duration::from_secs(3));
            let list = ignore.lock().map(|g| g.clone()).unwrap_or_default();
            let now = in_meeting(&list);
            if now != last {
                last = now;
                if tx.send(Msg::Meeting(now)).is_err() {
                    return;
                }
                ctx.request_repaint();
            }
        }
    });
}

// ---------------- pintasan global ----------------

pub struct Combo {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub win: bool,
    pub vk: i32,
}

/// "Ctrl+Shift+S" -> Combo. Mendukung A-Z, Num0-Num9, F1-F12.
pub fn parse_combo(s: &str) -> Option<Combo> {
    let mut c = Combo { ctrl: false, shift: false, alt: false, win: false, vk: 0 };
    for part in s.split('+') {
        let p = part.trim();
        match p.to_lowercase().as_str() {
            "ctrl" | "control" => c.ctrl = true,
            "shift" => c.shift = true,
            "alt" => c.alt = true,
            "win" => c.win = true,
            _ => {
                if p.len() == 1 && p.chars().all(|ch| ch.is_ascii_alphabetic()) {
                    c.vk = p.to_ascii_uppercase().as_bytes()[0] as i32;
                } else if let Some(n) = p.strip_prefix("Num").and_then(|x| x.parse::<i32>().ok()) {
                    if (0..=9).contains(&n) {
                        c.vk = 0x30 + n;
                    }
                } else if let Some(n) = p.strip_prefix('F').and_then(|x| x.parse::<i32>().ok()) {
                    if (1..=12).contains(&n) {
                        c.vk = 0x70 + (n - 1);
                    }
                }
            }
        }
    }
    if c.vk == 0 || !(c.ctrl || c.shift || c.alt || c.win) {
        None
    } else {
        Some(c)
    }
}

pub fn spawn_hotkey(tx: Sender<Msg>, ctx: egui::Context, combo: Arc<Mutex<String>>) {
    thread::spawn(move || {
        let mut registered = String::from("\u{0}");
        let mut active = false;
        loop {
            let text = combo.lock().map(|g| g.clone()).unwrap_or_default();
            if text != registered {
                unsafe {
                    if active {
                        let _ = UnregisterHotKey(HWND::default(), 1);
                        active = false;
                    }
                    if let Some(c) = parse_combo(&text) {
                        let mut m = MOD_NOREPEAT;
                        if c.ctrl {
                            m |= MOD_CONTROL;
                        }
                        if c.shift {
                            m |= MOD_SHIFT;
                        }
                        if c.alt {
                            m |= MOD_ALT;
                        }
                        if c.win {
                            m |= MOD_WIN;
                        }
                        active = RegisterHotKey(HWND::default(), 1, m, c.vk as u32).is_ok();
                    }
                }
                registered = text;
            }
            unsafe {
                let mut msg = MSG::default();
                while PeekMessageW(&mut msg, HWND::default(), WM_HOTKEY, WM_HOTKEY, PM_REMOVE).as_bool() {
                    if msg.message == WM_HOTKEY {
                        if tx.send(Msg::Hotkey).is_err() {
                            return;
                        }
                        ctx.request_repaint();
                    }
                }
            }
            thread::sleep(Duration::from_millis(60));
        }
    });
}

// ---------------- riwayat clipboard (teks, hanya di memori) ----------------

pub fn spawn_clipboard(tx: Sender<Msg>, ctx: egui::Context) {
    thread::spawn(move || {
        let mut cb = match arboard::Clipboard::new() {
            Ok(c) => c,
            Err(_) => return,
        };
        let mut last = String::new();
        loop {
            thread::sleep(Duration::from_millis(800));
            if let Ok(t) = cb.get_text() {
                if t != last && !t.trim().is_empty() {
                    last = t.clone();
                    let short: String = t.chars().take(500).collect();
                    if tx.send(Msg::Clip(short)).is_err() {
                        return;
                    }
                    ctx.request_repaint();
                }
            }
        }
    });
}

pub fn set_clipboard(text: &str) {
    if let Ok(mut cb) = arboard::Clipboard::new() {
        let _ = cb.set_text(text.to_string());
    }
}
