// Penyimpanan lokal: pengaturan dan data pengguna (JSON di %APPDATA%\RafLand).
// Riwayat clipboard sengaja TIDAK disimpan ke disk (bisa berisi data sensitif).

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::path::PathBuf;

pub fn dir() -> PathBuf {
    match std::env::var_os("APPDATA") {
        Some(a) => PathBuf::from(a).join("RafLand"),
        None => PathBuf::from("."),
    }
}

fn load_json<T: DeserializeOwned + Default>(name: &str) -> T {
    std::fs::read_to_string(dir().join(name))
        .ok()
        .and_then(|s| serde_json::from_str::<T>(&s).ok())
        .unwrap_or_default()
}

fn save_json<T: Serialize>(name: &str, v: &T) {
    let d = dir();
    let _ = std::fs::create_dir_all(&d);
    if let Ok(s) = serde_json::to_string_pretty(v) {
        let _ = std::fs::write(d.join(name), s);
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub dark: bool,
    pub lyrics: bool,
    pub notif: bool,
    pub mic_button: bool,
    pub meeting_auto: bool,
    pub hotkey: String,
    pub autostart: bool,
    // sholat
    pub region: String, // "ID" atau "MY"
    pub city: String,
    pub adzan: bool,
    pub adzan_pause: bool,
    // istirahat mata
    pub eye_on: bool,
    pub eye_min: u32,
    // pomodoro
    pub pomo_work: u32,
    pub pomo_short: u32,
    pub pomo_long: u32,
    // minum
    pub water_on: bool,
    pub water_goal: u32,
    pub water_cup: u32,
    pub water_start: u32,
    pub water_end: u32,
    pub water_min: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            dark: true,
            lyrics: true,
            notif: true,
            mic_button: true,
            meeting_auto: true,
            hotkey: "Ctrl+Shift+S".into(),
            autostart: false,
            region: "ID".into(),
            city: "Jakarta".into(),
            adzan: true,
            adzan_pause: true,
            eye_on: true,
            eye_min: 20,
            pomo_work: 25,
            pomo_short: 5,
            pomo_long: 15,
            water_on: false, // awalnya mati
            water_goal: 2000,
            water_cup: 250,
            water_start: 8,
            water_end: 21,
            water_min: 60,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        load_json("settings.json")
    }
    pub fn save(&self) {
        save_json("settings.json", self);
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct AgendaItem {
    pub id: u64,
    pub title: String,
    pub ymd: u32,
    pub hm: u32, // menit sejak 00:00
    pub done: bool,
    pub notified: bool,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Data {
    pub water_ymd: u32,
    pub water_ml: u32,
    pub agenda: Vec<AgendaItem>,
    pub shelf: Vec<String>,
    pub next_id: u64,
}

impl Data {
    pub fn load() -> Self {
        load_json("data.json")
    }
    pub fn save(&self) {
        save_json("data.json", self);
    }
}

pub fn days_in_month(y: u32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                29
            } else {
                28
            }
        }
    }
}

/// Hari berikutnya untuk tanggal berformat YYYYMMDD.
pub fn next_day(ymd: u32) -> u32 {
    let (mut y, mut m, mut d) = (ymd / 10000, (ymd / 100) % 100, ymd % 100);
    d += 1;
    if d > days_in_month(y, m) {
        d = 1;
        m += 1;
        if m > 12 {
            m = 1;
            y += 1;
        }
    }
    y * 10000 + m * 100 + d
}

/// Hari dalam seminggu, Senin = 0 ... Minggu = 6.
pub fn weekday(y: u32, m: u32, d: u32) -> u32 {
    let tbl = [0u32, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let yy = if m < 3 { y - 1 } else { y };
    let dow = (yy + yy / 4 - yy / 100 + yy / 400 + tbl[(m - 1) as usize] + d) % 7; // 0 = Minggu
    (dow + 6) % 7
}
