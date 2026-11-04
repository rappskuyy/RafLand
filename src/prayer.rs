// Jadwal sholat (Aladhan API) dan cuaca (Open-Meteo). Tanpa API key.
//
// Malaysia memakai metode 17 (sudut JAKIM), Indonesia memakai metode 20 (Kemenag).
// Hasilnya perhitungan berdasarkan sudut metode tersebut, bukan tabel zona resmi e-solat.

use crate::Msg;
use eframe::egui;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

pub const MY_CITIES: &[&str] = &[
    "Kuala Lumpur", "Putrajaya", "Labuan", "Shah Alam", "Petaling Jaya", "Klang", "Subang Jaya",
    "Kajang", "Ampang", "Rawang", "Seremban", "Nilai", "Port Dickson", "Melaka", "Alor Gajah",
    "Johor Bahru", "Batu Pahat", "Muar", "Kluang", "Segamat", "Kota Tinggi", "Pontian", "Mersing",
    "Kulai", "George Town", "Butterworth", "Bukit Mertajam", "Ipoh", "Taiping", "Teluk Intan",
    "Sitiawan", "Kampar", "Alor Setar", "Sungai Petani", "Kulim", "Langkawi", "Kangar",
    "Kota Bharu", "Kuala Terengganu", "Dungun", "Kemaman", "Kuantan", "Temerloh", "Bentong",
    "Raub", "Jerantut", "Kuala Lipis", "Cameron Highlands", "Kuching", "Sibu", "Miri", "Bintulu",
    "Kota Kinabalu", "Sandakan", "Tawau", "Lahad Datu",
];

pub const ID_CITIES: &[&str] = &[
    "Jakarta", "Bogor", "Depok", "Tangerang", "Bekasi", "Bandung", "Cimahi", "Sukabumi", "Cirebon",
    "Semarang", "Yogyakarta", "Surakarta", "Surabaya", "Malang", "Kediri", "Denpasar", "Mataram",
    "Medan", "Pekanbaru", "Padang", "Palembang", "Jambi", "Bandar Lampung", "Batam", "Banda Aceh",
    "Pontianak", "Banjarmasin", "Balikpapan", "Samarinda", "Makassar", "Manado", "Palu", "Kupang",
    "Ambon", "Jayapura",
];

pub fn cities(region: &str) -> &'static [&'static str] {
    if region == "MY" { MY_CITIES } else { ID_CITIES }
}

pub fn country(region: &str) -> &'static str {
    if region == "MY" { "Malaysia" } else { "Indonesia" }
}

pub fn region_label(region: &str) -> &'static str {
    if region == "MY" { "Malaysia (JAKIM)" } else { "Indonesia (Kemenag)" }
}

/// Urutan: Subuh, Terbit, Dzuhur, Ashar, Maghrib, Isya. Malaysia: Zohor, Asar, Isyak.
pub fn names(region: &str) -> [&'static str; 6] {
    if region == "MY" {
        ["Subuh", "Syuruk", "Zohor", "Asar", "Maghrib", "Isyak"]
    } else {
        ["Subuh", "Terbit", "Dzuhur", "Ashar", "Maghrib", "Isya"]
    }
}

/// "HH:MM" -> menit sejak tengah malam.
pub fn parse_hm(s: &str) -> Option<u32> {
    let mut it = s.split(':');
    let h = it.next()?.trim().parse::<u32>().ok()?;
    let m = it.next()?.trim().parse::<u32>().ok()?;
    if h < 24 && m < 60 { Some(h * 60 + m) } else { None }
}

fn client() -> Option<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .user_agent("RafLand/0.1")
        .timeout(Duration::from_secs(10))
        .build()
        .ok()
}

pub fn fetch_times(tx: Sender<Msg>, ctx: egui::Context, key: String, region: String, city: String) {
    thread::spawn(move || {
        let res = (|| -> Option<[String; 6]> {
            let c = client()?;
            let method = if region == "MY" { "17" } else { "20" };
            let json: serde_json::Value = c
                .get("https://api.aladhan.com/v1/timingsByCity")
                .query(&[("city", city.as_str()), ("country", country(&region)), ("method", method)])
                .send()
                .ok()?
                .json()
                .ok()?;
            let t = json.get("data")?.get("timings")?;
            let mut out: [String; 6] = Default::default();
            for (i, k) in ["Fajr", "Sunrise", "Dhuhr", "Asr", "Maghrib", "Isha"].iter().enumerate() {
                let s = t.get(*k)?.as_str()?;
                out[i] = s.chars().take(5).collect();
            }
            Some(out)
        })();
        let _ = tx.send(Msg::Prayer { key, times: res });
        ctx.request_repaint();
    });
}

fn weather_text(code: i64) -> &'static str {
    match code {
        0 => "Cerah",
        1 | 2 => "Cerah berawan",
        3 => "Berawan",
        45 | 48 => "Berkabut",
        51..=57 => "Gerimis",
        61..=67 => "Hujan",
        71..=77 => "Salju",
        80..=82 => "Hujan lebat",
        95..=99 => "Badai petir",
        _ => "Tidak diketahui",
    }
}

pub fn fetch_weather(tx: Sender<Msg>, ctx: egui::Context, key: String, region: String, city: String) {
    thread::spawn(move || {
        let res = (|| -> Option<(f32, String)> {
            let c = client()?;
            let cc = if region == "MY" { "MY" } else { "ID" };
            let geo: serde_json::Value = c
                .get("https://geocoding-api.open-meteo.com/v1/search")
                .query(&[("name", city.as_str()), ("count", "1"), ("country_code", cc), ("language", "en")])
                .send()
                .ok()?
                .json()
                .ok()?;
            let first = geo.get("results")?.get(0)?;
            let lat = first.get("latitude")?.as_f64()?;
            let lon = first.get("longitude")?.as_f64()?;
            let w: serde_json::Value = c
                .get("https://api.open-meteo.com/v1/forecast")
                .query(&[
                    ("latitude", lat.to_string()),
                    ("longitude", lon.to_string()),
                    ("current", "temperature_2m,weather_code".to_string()),
                    ("timezone", "auto".to_string()),
                ])
                .send()
                .ok()?
                .json()
                .ok()?;
            let cur = w.get("current")?;
            let temp = cur.get("temperature_2m")?.as_f64()? as f32;
            let code = cur.get("weather_code")?.as_i64()?;
            Some((temp, weather_text(code).to_string()))
        })();
        let _ = tx.send(Msg::Weather { key, info: res });
        ctx.request_repaint();
    });
}
