// Windows SMTC (GlobalSystemMediaTransportControls).
// Poller berjalan di thread sendiri dan mengirim pesan ke UI lewat channel.

use crate::Msg;
use eframe::egui;
use std::{
    sync::mpsc::Sender,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use windows::{
    core::Interface,
    Media::Control::{
        GlobalSystemMediaTransportControlsSession as Session,
        GlobalSystemMediaTransportControlsSessionManager as Manager,
        GlobalSystemMediaTransportControlsSessionMediaProperties as Props,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
    },
    Storage::Streams::{DataReader, IInputStream, IRandomAccessStream},
    Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED},
};

#[derive(Clone, Default)]
pub struct MediaState {
    pub available: bool,
    pub app: String,
    pub title: String,
    pub artist: String,
    pub playing: bool,
    pub position_ms: i64,
    pub duration_ms: i64,
    pub track_key: String,
}


/// Ubah teks LRC ("[mm:ss.xx] baris") menjadi daftar (ms, teks); baris rusak dibuang.
fn parse_synced(synced: &str) -> Vec<(i64, String)> {
    let mut lines = Vec::new();
    for raw in synced.lines() {
        let Some(end) = raw.find(']') else { continue };
        let Some(start) = raw.find('[') else { continue };
        if start != 0 || end <= start + 1 {
            continue;
        }
        let stamp = &raw[start + 1..end];
        let mut parts = stamp.split(':');
        let mins = parts.next().and_then(|v| v.parse::<i64>().ok());
        let secs = parts.next().and_then(|v| v.parse::<f64>().ok());
        if let (Some(mins), Some(secs)) = (mins, secs) {
            let text = raw[end + 1..].trim().to_string();
            if !text.is_empty() {
                lines.push((mins * 60_000 + (secs * 1000.0) as i64, text));
            }
        }
    }
    lines.sort_by_key(|line| line.0);
    lines
}

/// Ambil lirik bertimestamp dari LRCLIB. Coba pencocokan tepat dulu, lalu pencarian bebas.
pub fn fetch_lyrics(tx: Sender<Msg>, key: String, title: String, artist: String, duration_ms: i64) {
    thread::spawn(move || {
        let client = match reqwest::blocking::Client::builder()
            .user_agent("RafLand/0.1 (lyrics display)")
            .timeout(Duration::from_secs(8))
            .build()
        {
            Ok(c) => c,
            Err(_) => {
                let _ = tx.send(Msg::Lyrics { key, lines: Vec::new() });
                return;
            }
        };
        let duration = (duration_ms.max(0) as f64 / 1000.0).round() as i64;
        let mut lines: Vec<(i64, String)> = Vec::new();

        // 1) pencocokan tepat
        if let Ok(resp) = client
            .get("https://lrclib.net/api/get")
            .query(&[("track_name", title.as_str()), ("artist_name", artist.as_str())])
            .query(&[("duration", duration.to_string())])
            .send()
        {
            if let Ok(json) = resp.json::<serde_json::Value>() {
                let synced = json.get("syncedLyrics").and_then(|v| v.as_str()).unwrap_or("");
                lines = parse_synced(synced);
            }
        }

        // 2) cadangan: pencarian bebas, ambil hasil pertama yang punya lirik tersinkron
        if lines.is_empty() {
            if let Ok(resp) = client
                .get("https://lrclib.net/api/search")
                .query(&[("track_name", title.as_str()), ("artist_name", artist.as_str())])
                .send()
            {
                if let Ok(json) = resp.json::<serde_json::Value>() {
                    if let Some(arr) = json.as_array() {
                        for item in arr {
                            let synced = item.get("syncedLyrics").and_then(|v| v.as_str()).unwrap_or("");
                            let parsed = parse_synced(synced);
                            if !parsed.is_empty() {
                                lines = parsed;
                                break;
                            }
                        }
                    }
                }
            }
        }
        let _ = tx.send(Msg::Lyrics { key, lines });
    });
}

fn com_init() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

fn request_manager() -> Option<Manager> {
    Manager::RequestAsync().ok()?.get().ok()
}

/// Prioritaskan sesi Spotify; kalau tidak ada pakai sesi aktif sistem.
fn pick_session(mgr: &Manager) -> Option<Session> {
    if let Ok(list) = mgr.GetSessions() {
        if let Ok(n) = list.Size() {
            for i in 0..n {
                if let Ok(s) = list.GetAt(i) {
                    if let Ok(id) = s.SourceAppUserModelId() {
                        if id.to_string().to_lowercase().contains("spotify") {
                            return Some(s);
                        }
                    }
                }
            }
        }
    }
    mgr.GetCurrentSession().ok()
}

fn read_thumbnail(props: &Props) -> Option<egui::ColorImage> {
    let thumb = props.Thumbnail().ok()?;
    let stream = thumb.OpenReadAsync().ok()?.get().ok()?;
    let ras: IRandomAccessStream = stream.cast().ok()?;
    let size = ras.Size().ok()? as u32;
    if size == 0 {
        return None;
    }
    let input: IInputStream = ras.cast().ok()?;
    let reader = DataReader::CreateDataReader(&input).ok()?;
    reader.LoadAsync(size).ok()?.get().ok()?;
    let mut buf = vec![0u8; size as usize];
    reader.ReadBytes(&mut buf).ok()?;

    let img = image::load_from_memory(&buf).ok()?.thumbnail(160, 160).to_rgba8();
    let dims = [img.width() as usize, img.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(dims, img.as_raw()))
}

fn now_windows_ticks() -> i64 {
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    (ns / 100) as i64 + 116_444_736_000_000_000
}

fn build_state(session: &Session, props: &Props) -> windows::core::Result<MediaState> {
    let app = session.SourceAppUserModelId().map(|s| s.to_string()).unwrap_or_default();
    let title = props.Title()?.to_string();
    let artist = props.Artist()?.to_string();
    let playing = session.GetPlaybackInfo()?.PlaybackStatus()? == Status::Playing;

    let tl = session.GetTimelineProperties()?;
    let duration_ms = (tl.EndTime()?.Duration - tl.StartTime()?.Duration) / 10_000;
    let mut position_ms = tl.Position()?.Duration / 10_000;
    if playing {
        // Position = snapshot pada LastUpdatedTime -> ekstrapolasi sampai sekarang.
        let delta = (now_windows_ticks() - tl.LastUpdatedTime()?.UniversalTime) / 10_000;
        position_ms += delta.max(0);
    }
    if duration_ms > 0 {
        position_ms = position_ms.clamp(0, duration_ms);
    }
    Ok(MediaState {
        available: true,
        track_key: format!("{app}|{title}|{artist}"),
        app,
        title,
        artist,
        playing,
        position_ms,
        duration_ms: duration_ms.max(0),
    })
}

pub fn spawn_poller(tx: Sender<Msg>, ctx: egui::Context) {
    thread::spawn(move || {
        com_init();
        let mut mgr: Option<Manager> = None;
        let mut last_key = String::new();
        let mut cover_done = false;
        let mut cover_tries = 0u8;

        loop {
            thread::sleep(Duration::from_millis(500));
            if mgr.is_none() {
                mgr = request_manager();
            }
            let session = mgr.as_ref().and_then(pick_session);
            let snapshot = session.as_ref().and_then(|s| {
                let props = s.TryGetMediaPropertiesAsync().ok()?.get().ok()?;
                let st = build_state(s, &props).ok()?;
                Some((st, props))
            });

            match snapshot {
                Some((state, props)) => {
                    if state.track_key != last_key {
                        last_key = state.track_key.clone();
                        cover_done = false;
                        cover_tries = 0;
                    }
                    let key = state.track_key.clone();
                    if tx.send(Msg::State(state)).is_err() {
                        return;
                    }
                    // Spotify kadang telat menyediakan cover -> coba ulang beberapa kali.
                    if !cover_done && cover_tries < 8 {
                        cover_tries += 1;
                        if let Some(image) = read_thumbnail(&props) {
                            cover_done = true;
                            let _ = tx.send(Msg::Cover { key, image });
                        }
                    }
                }
                None => {
                    last_key.clear();
                    if tx.send(Msg::State(MediaState::default())).is_err() {
                        return;
                    }
                }
            }
            ctx.request_repaint();
        }
    });
}

/// action: "toggle" | "pause" | "play" | "next" | "prev" | "seek" (value = ms)
pub fn control(action: &'static str, value: Option<i64>) {
    thread::spawn(move || {
        com_init();
        let Some(mgr) = request_manager() else { return };
        let Some(s) = pick_session(&mgr) else { return };
        match action {
            "toggle" => {
                let _ = s.TryTogglePlayPauseAsync().and_then(|o| o.get());
            }
            "pause" => {
                let _ = s.TryPauseAsync().and_then(|o| o.get());
            }
            "play" => {
                let _ = s.TryPlayAsync().and_then(|o| o.get());
            }
            "next" => {
                let _ = s.TrySkipNextAsync().and_then(|o| o.get());
            }
            "prev" => {
                let _ = s.TrySkipPreviousAsync().and_then(|o| o.get());
            }
            "seek" => {
                if let Some(ms) = value {
                    let _ = s.TryChangePlaybackPositionAsync(ms * 10_000).and_then(|o| o.get());
                }
            }
            _ => {}
        }
    });
}