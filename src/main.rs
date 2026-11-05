#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// RafLand - Dynamic Island bergaya kaca untuk Windows.
// Dasar: animasi island, kontrol media/lirik, kecerahan/volume dari proyek azay.

mod anim;
mod draw;
mod media;
mod prayer;
mod settings_ui;
mod share;
mod store;
mod sys;
mod ui;

use anim::Tween;
use eframe::egui::{self, pos2, Rect, ViewportCommand};
use media::MediaState;
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use store::{AgendaItem, Data, Settings};

pub const WIN_W: f32 = 560.0;
pub const WIN_H: f32 = 450.0;

/// Pesan dari thread latar ke UI.
pub enum Msg {
    State(MediaState),
    Cover { key: String, image: egui::ColorImage },
    Lyrics { key: String, lines: Vec<(i64, String)> },
    Brightness(u8),
    Prayer { key: String, times: Option<[String; 6]> },
    Weather { key: String, info: Option<(f32, String)> },
    Meeting(bool),
    Clip(String),
    Hotkey,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    Collapsed,
    Music,
    Alert,
    Expanded,
}

impl Mode {
    /// [lebar, tinggi, offset-atas] pill
    pub fn geom(self) -> [f32; 3] {
        match self {
            Mode::Collapsed => [210.0, 38.0, 8.0],
            Mode::Music => [320.0, 38.0, 8.0],
            Mode::Alert => [380.0, 60.0, 8.0],
            Mode::Expanded => [520.0, 410.0, 8.0],
        }
    }
    pub fn hit_w(self) -> f32 {
        self.geom()[0]
    }
    pub fn anim_secs(self) -> f32 {
        match self {
            Mode::Expanded => 0.45,
            Mode::Alert => 0.35,
            _ => 0.32,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Home,
    Focus,
    Prayer,
    Water,
    Shelf,
    Agenda,
    Eye,
    Capture,
    Settings,
}

#[derive(Clone, Copy, PartialEq)]
pub enum FocusMode {
    Pomodoro,
    Timer,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Phase {
    Work,
    Short,
    Long,
}

#[derive(Clone, Copy, PartialEq)]
pub enum AlertKind {
    Water,
    Eye,
    Prayer,
    Resume,
    Timer,
    Agenda,
}

pub struct Alert {
    pub kind: AlertKind,
    pub text: String,
    pub sub: String,
    pub secs: Option<u64>,
    pub until: Option<Instant>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum SetId {
    Dark,
    Lyrics,
    MicBtn,
    MeetingAuto,
    Notif,
    Hotkey,
    Adzan,
    AdzanPause,
    Region,
    EyeOn,
    EyeInt,
    PomoWork,
    PomoShort,
    PomoLong,
    Autostart,
    OpenWin,
    Quit,
    WaterOn,
    WaterInt,
    WaterGoal,
    WaterStart,
    WaterEnd,
    Weather,
    Clip,
    Robot,
    LyricOff,
    HotkeyOn,
    HotkeyReset,
    ThemeDark,
    ThemeLight,
    CityStep,
}

#[derive(Clone)]
pub enum Action {
    PillClick,
    Pin,
    Tab(Tab),
    Prev,
    Toggle,
    Next,
    Seek(i64),
    BrightnessValue(u8),
    VolumeValue(u8),
    MicToggle,
    Screenshot,
    Record,
    AlertClick,
    AlertClose,
    PomoToggle,
    PomoReset,
    PomoSkip,
    PomoMode(FocusMode),
    TimerAdj(i32),
    Drink(i32),
    Set(SetId, i32),
    AgAdd,
    AgToggle(u64),
    AgDel(u64),
    AgTime(i32),
    AgDay,
    EyeSet(u32),
    EyeTest,
    CalNav(i32),
    CalSel(u32),
    CapFull,
    CapWindow,
    OpenSettings,
    SetCat(settings_ui::Cat),
    SetF(SetId, f32),
    IgnoreAdd,
    IgnoreDel(usize),
    TestAdzan,
    OpenDataDir,
    OpenShots,
    OpenCaps,
    ShelfRemove(usize),
    ShelfOpen(usize),
    ShareToggle,
    ClipCopy(usize),
    ClipClear,
    CityPick(bool),
    CityChoose(String),
}

pub struct Pomo {
    pub mode: FocusMode,
    pub phase: Phase,
    pub remaining_ms: i64,
    pub running: bool,
    pub last: Instant,
    pub cycles: u32,
    pub timer_min: u32,
}

pub struct App {
    pub rx: Receiver<Msg>,
    pub tx: Sender<Msg>,
    pub ctx: egui::Context,
    pub t0: Instant,
    pub cfg: Settings,
    pub data: Data,

    pub media: MediaState,
    pub media_at: Instant,
    pub cover: Option<(String, egui::TextureHandle)>,
    pub lyrics: Vec<(i64, String)>,
    pub lyrics_key: String,
    pub battery: sys::Battery,
    pub battery_at: Instant,
    pub brightness_percent: u8,
    pub volume_percent: u8,
    pub vol_sync_at: Instant,
    pub mic_muted: bool,
    pub mic_at: Instant,
    pub meeting: bool,

    pub mode: Mode,
    pub tween: Tween,
    pub pinned: bool,
    pub hovering: bool,
    pub passthrough: bool,
    pub pending: Option<(Instant, Mode)>,
    pub placed_frames: u8,
    pub tab: Tab,

    pub alert: Option<Alert>,
    pub alerts: VecDeque<Alert>,

    pub pomo: Pomo,
    pub next_eye_at: Instant,
    pub next_water_at: Instant,

    pub prayer_times: Option<[String; 6]>,
    pub prayer_key: String,
    pub prayer_fetch_key: String,
    pub prayer_retry_at: Instant,
    pub weather: Option<(f32, String)>,
    pub weather_key: String,
    pub weather_fetch_key: String,
    pub weather_retry_at: Instant,
    pub city_picking: bool,
    pub city_scroll: usize,
    pub city_filter: String,
    pub adzan_fired: [u32; 6],
    pub resume_at: Option<Instant>,

    pub clips: Vec<String>,
    pub share_files: Arc<Mutex<Vec<PathBuf>>>,
    pub share_on: Arc<AtomicBool>,
    pub share_url: Option<String>,

    pub ag_title: String,
    pub ag_hm: u32,
    pub ag_tomorrow: bool,
    pub cal_off: i32,
    pub cal_sel: u32,

    pub search: String,
    pub set_scroll: usize,
    pub capturing_hotkey: bool,
    pub hotkey_shared: Arc<Mutex<String>>,
    pub focus_search: bool,
    pub settings_open: bool,
    pub set_cat: settings_ui::Cat,
    pub set_scroll_px: f32,
    pub set_content_h: f32,
    pub ignore_input: String,
    pub ignore_shared: Arc<Mutex<Vec<String>>>,
    pub last_slow: Instant,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        draw::setup_fonts(&cc.egui_ctx);
        let (tx, rx) = mpsc::channel();
        let ctx = cc.egui_ctx.clone();
        let cfg = Settings::load();
        let mut data = Data::load();
        let now = Instant::now();

        media::spawn_poller(tx.clone(), ctx.clone());
        let ignore_shared = Arc::new(Mutex::new(cfg.ignore_titles.clone()));
        sys::spawn_meeting_watcher(tx.clone(), ctx.clone(), ignore_shared.clone());
        sys::spawn_clipboard(tx.clone(), ctx.clone());
        let hotkey_shared = Arc::new(Mutex::new(if cfg.hotkey_on { cfg.hotkey.clone() } else { String::new() }));
        sys::spawn_hotkey(tx.clone(), ctx.clone(), hotkey_shared.clone());

        let today = sys::local_dt();
        if data.water_ymd != today.ymd() {
            data.water_ymd = today.ymd();
            data.water_ml = 0;
        }
        data.agenda.retain(|a| a.ymd >= today.ymd());

        let shelf: Vec<PathBuf> = data.shelf.iter().map(PathBuf::from).filter(|p| p.exists()).collect();
        data.shelf = shelf.iter().map(|p| p.display().to_string()).collect();
        let default_hm = ((today.h + 1) % 24) * 60;
        let work_ms = cfg.pomo_work as i64 * 60_000;
        let eye = Duration::from_secs(cfg.eye_min as u64 * 60);
        let water = Duration::from_secs(cfg.water_min as u64 * 60);

        Self {
            rx,
            tx,
            ctx,
            t0: now,
            media: MediaState::default(),
            media_at: now,
            cover: None,
            lyrics: Vec::new(),
            lyrics_key: String::new(),
            battery: sys::battery(),
            battery_at: now,
            brightness_percent: 50,
            volume_percent: sys::get_volume().unwrap_or(50),
            vol_sync_at: now,
            mic_muted: sys::mic_muted().unwrap_or(false),
            mic_at: now,
            meeting: false,
            mode: Mode::Collapsed,
            tween: Tween::new(Mode::Collapsed.geom()),
            pinned: false,
            hovering: false,
            passthrough: true,
            pending: None,
            placed_frames: 0,
            tab: Tab::Home,
            alert: None,
            alerts: VecDeque::new(),
            pomo: Pomo {
                mode: FocusMode::Pomodoro,
                phase: Phase::Work,
                remaining_ms: work_ms,
                running: false,
                last: now,
                cycles: 0,
                timer_min: 10,
            },
            next_eye_at: now + eye,
            next_water_at: now + water,
            prayer_times: None,
            prayer_key: String::new(),
            prayer_fetch_key: String::new(),
            prayer_retry_at: now,
            weather: None,
            weather_key: String::new(),
            weather_fetch_key: String::new(),
            weather_retry_at: now,
            city_picking: false,
            city_scroll: 0,
            city_filter: String::new(),
            adzan_fired: [0; 6],
            resume_at: None,
            clips: Vec::new(),
            share_files: Arc::new(Mutex::new(shelf)),
            share_on: Arc::new(AtomicBool::new(false)),
            share_url: None,
            ag_title: String::new(),
            ag_hm: default_hm,
            ag_tomorrow: false,
            cal_off: 0,
            cal_sel: today.ymd(),
            search: String::new(),
            set_scroll: 0,
            capturing_hotkey: false,
            hotkey_shared,
            focus_search: false,
            settings_open: false,
            set_cat: settings_ui::Cat::Umum,
            set_scroll_px: 0.0,
            set_content_h: 0.0,
            ignore_input: String::new(),
            ignore_shared,
            last_slow: now.checked_sub(Duration::from_secs(5)).unwrap_or(now),
            cfg,
            data,
        }
    }

    // ---------------- mode / animasi ----------------

    pub fn music_active(&self) -> bool {
        self.cfg.lyrics && self.media.available && self.media.playing && !self.lyrics.is_empty()
    }

    pub fn rest_mode(&self) -> Mode {
        if self.alert.is_some() {
            Mode::Alert
        } else if self.music_active() {
            Mode::Music
        } else {
            Mode::Collapsed
        }
    }

    fn set_mode(&mut self, m: Mode, now: Instant) {
        if m == self.mode {
            return;
        }
        self.tween.retarget(now, m.geom(), m.anim_secs());
        self.mode = m;
    }

    fn on_hover(&mut self, h: bool, now: Instant, typing: bool) {
        self.hovering = h;
        if self.mode == Mode::Alert {
            return; // saat ada notifikasi, klik dipakai untuk aksinya
        }
        if h {
            self.pending = Some((now + Duration::from_millis(90), Mode::Expanded));
        } else if !self.pinned && !typing {
            self.pending = Some((now + Duration::from_millis(350), self.rest_mode()));
        }
    }

    fn tick(&mut self, now: Instant) {
        if let Some((t, m)) = self.pending {
            if now >= t {
                self.pending = None;
                self.set_mode(m, now);
            }
        }
        // alert kedaluwarsa
        let expired = self.alert.as_ref().and_then(|a| a.until).map(|u| now >= u).unwrap_or(false);
        if expired {
            self.dismiss_alert();
        }
        // sesuaikan pill istirahat dengan kondisi (alert / musik / biasa)
        if self.mode != Mode::Expanded && self.pending.is_none() {
            let want = self.rest_mode();
            if want != self.mode {
                self.set_mode(want, now);
            }
        }
        self.pomo_tick(now);
    }

    // ---------------- notifikasi ringkas ----------------

    pub fn push_alert(&mut self, kind: AlertKind, text: String, sub: String, secs: Option<u64>) {
        if !self.cfg.notif {
            return;
        }
        let mut a = Alert { kind, text, sub, secs, until: None };
        if self.alert.is_none() {
            a.until = secs.map(|s| Instant::now() + Duration::from_secs(s));
            self.alert = Some(a);
        } else {
            self.alerts.push_back(a);
        }
    }

    pub fn dismiss_alert(&mut self) {
        self.alert = self.alerts.pop_front();
        if let Some(a) = &mut self.alert {
            a.until = a.secs.map(|s| Instant::now() + Duration::from_secs(s));
        }
    }

    fn alert_click(&mut self, now: Instant) {
        let kind = self.alert.as_ref().map(|a| a.kind);
        match kind {
            Some(AlertKind::Water) => self.drink(self.cfg.water_cup as i32),
            Some(AlertKind::Resume) => {
                media::control("play", None);
                self.media.playing = true;
                self.media_at = now;
            }
            _ => {}
        }
        self.dismiss_alert();
    }

    fn drink(&mut self, ml: i32) {
        self.data.water_ml = (self.data.water_ml as i32 + ml).max(0) as u32;
        self.data.save();
    }

    // ---------------- pomodoro / timer ----------------

    pub fn pomo_phase_ms(&self) -> i64 {
        let m = match self.pomo.mode {
            FocusMode::Timer => self.pomo.timer_min,
            FocusMode::Pomodoro => match self.pomo.phase {
                Phase::Work => self.cfg.pomo_work,
                Phase::Short => self.cfg.pomo_short,
                Phase::Long => self.cfg.pomo_long,
            },
        };
        m as i64 * 60_000
    }

    fn pomo_tick(&mut self, now: Instant) {
        if !self.pomo.running {
            self.pomo.last = now;
            return;
        }
        let dt = now.duration_since(self.pomo.last).as_millis() as i64;
        self.pomo.last = now;
        self.pomo.remaining_ms -= dt;
        if self.pomo.remaining_ms <= 0 {
            self.pomo_done();
        }
    }

    fn pomo_done(&mut self) {
        match self.pomo.mode {
            FocusMode::Timer => {
                self.pomo.running = false;
                self.pomo.remaining_ms = self.pomo_phase_ms();
                self.push_alert(AlertKind::Timer, "Timer selesai".into(), "Waktu habis".into(), Some(12));
            }
            FocusMode::Pomodoro => {
                if self.pomo.phase == Phase::Work {
                    self.pomo.cycles += 1;
                    self.pomo.phase = if self.pomo.cycles % 4 == 0 { Phase::Long } else { Phase::Short };
                    self.push_alert(AlertKind::Timer, "Sesi fokus selesai".into(), "Saatnya istirahat".into(), Some(12));
                } else {
                    self.pomo.phase = Phase::Work;
                    self.push_alert(AlertKind::Timer, "Istirahat selesai".into(), "Kembali fokus".into(), Some(12));
                }
                self.pomo.remaining_ms = self.pomo_phase_ms();
            }
        }
    }

    pub fn pomo_reset(&mut self) {
        self.pomo.running = false;
        self.pomo.phase = Phase::Work;
        self.pomo.remaining_ms = self.pomo_phase_ms();
    }

    // ---------------- media ----------------

    pub fn position_ms(&self, now: Instant) -> i64 {
        let mut p = self.media.position_ms;
        if self.media.playing {
            p += now.duration_since(self.media_at).as_millis() as i64;
        }
        if self.media.duration_ms > 0 {
            p = p.min(self.media.duration_ms);
        }
        p
    }

    // ---------------- pekerjaan tiap detik ----------------

    fn slow_tick(&mut self, now: Instant) {
        if now.duration_since(self.last_slow) < Duration::from_secs(1) {
            return;
        }
        self.last_slow = now;
        let dt = sys::local_dt();
        let ymd = dt.ymd();
        let mins = dt.minutes();

        if self.data.water_ymd != ymd {
            self.data.water_ymd = ymd;
            self.data.water_ml = 0;
            self.data.agenda.retain(|a| a.ymd >= ymd);
            self.data.save();
        }

        // jadwal sholat
        let pkey = format!("{}|{}|{}", self.cfg.region, self.cfg.city, ymd);
        if self.prayer_key != pkey && self.prayer_fetch_key != pkey && now >= self.prayer_retry_at {
            self.prayer_fetch_key = pkey.clone();
            prayer::fetch_times(self.tx.clone(), self.ctx.clone(), pkey.clone(), self.cfg.region.clone(), self.cfg.city.clone());
        }
        // cuaca (disegarkan tiap 2 jam)
        let wkey = format!("{}|{}|{}|{}", self.cfg.region, self.cfg.city, ymd, dt.h / 2);
        if self.cfg.weather_on && self.weather_key != wkey && self.weather_fetch_key != wkey && now >= self.weather_retry_at {
            self.weather_fetch_key = wkey.clone();
            prayer::fetch_weather(self.tx.clone(), self.ctx.clone(), wkey, self.cfg.region.clone(), self.cfg.city.clone());
        }

        // adzan
        if self.cfg.adzan && self.prayer_key == pkey {
            if let Some(times) = self.prayer_times.clone() {
                for idx in [0usize, 2, 3, 4, 5] {
                    if let Some(m) = prayer::parse_hm(&times[idx]) {
                        if m == mins && self.adzan_fired[idx] != ymd {
                            self.adzan_fired[idx] = ymd;
                            self.fire_adzan(idx, now);
                        }
                    }
                }
            }
        }
        // tawaran melanjutkan musik setelah adzan
        if let Some(t) = self.resume_at {
            if now >= t {
                self.resume_at = None;
                self.push_alert(AlertKind::Resume, "Lanjutkan musik?".into(), "Klik untuk memutar lagi".into(), Some(600));
            }
        }

        // pengingat istirahat mata
        if self.cfg.eye_on && now >= self.next_eye_at {
            self.next_eye_at = now + Duration::from_secs(self.cfg.eye_min as u64 * 60);
            if !self.meeting {
                self.push_alert(AlertKind::Eye, "Istirahatkan matamu".into(), "Lihat objek jauh selama 20 detik".into(), Some(12));
            }
        }
        // pengingat minum
        if self.cfg.water_on && now >= self.next_water_at {
            self.next_water_at = now + Duration::from_secs(self.cfg.water_min as u64 * 60);
            if self.data.water_ml < self.cfg.water_goal && dt.h >= self.cfg.water_start && dt.h < self.cfg.water_end {
                self.push_alert(AlertKind::Water, "Haus nih, minum yuk".into(), "Klik kalau sudah minum".into(), Some(30));
            }
        }
        // agenda
        let mut due: Vec<usize> = Vec::new();
        for (i, a) in self.data.agenda.iter().enumerate() {
            if !a.done && !a.notified && a.ymd == ymd && a.hm <= mins && mins - a.hm <= 10 {
                due.push(i);
            }
        }
        for i in due {
            self.data.agenda[i].notified = true;
            let title = self.data.agenda[i].title.clone();
            let hm = self.data.agenda[i].hm;
            self.push_alert(AlertKind::Agenda, title, format!("Agenda {:02}:{:02}", hm / 60, hm % 60), Some(15));
        }
        if !self.data.agenda.is_empty() {
            self.data.save();
        }
    }

    fn fire_adzan(&mut self, idx: usize, now: Instant) {
        let name = prayer::names(&self.cfg.region)[idx];
        self.push_alert(AlertKind::Prayer, format!("Waktu {name}"), self.cfg.city.clone(), Some(20));
        if self.cfg.adzan_pause && self.media.playing {
            media::control("pause", None);
            self.media.playing = false;
            self.media_at = now;
            self.resume_at = Some(now + Duration::from_secs(240));
        }
        sys::play_adzan();
    }

    pub fn add_agenda(&mut self) {
        let title = self.ag_title.trim().to_string();
        if title.is_empty() {
            return;
        }
        let ymd = if self.cal_sel == 0 { sys::local_dt().ymd() } else { self.cal_sel };
        let id = self.data.next_id + 1;
        self.data.next_id = id;
        self.data.agenda.push(AgendaItem { id, title, ymd, hm: self.ag_hm, done: false, notified: false });
        self.data.agenda.sort_by_key(|a| (a.ymd, a.hm));
        self.ag_title.clear();
        self.data.save();
    }

    fn sync_shelf(&mut self) {
        let list: Vec<PathBuf> = self.data.shelf.iter().map(PathBuf::from).collect();
        if let Ok(mut g) = self.share_files.lock() {
            *g = list;
        }
        self.data.save();
    }

    // ---------------- pesan dari thread latar ----------------

    fn drain_msgs(&mut self, ctx: &egui::Context, now: Instant) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::State(s) => {
                    let changed = s.available && s.track_key != self.lyrics_key;
                    self.media = s;
                    self.media_at = now;
                    if changed && !self.media.title.is_empty() {
                        self.lyrics_key = self.media.track_key.clone();
                        self.lyrics.clear();
                        let artist = if self.media.artist.is_empty() { self.media.app.clone() } else { self.media.artist.clone() };
                        media::fetch_lyrics(self.tx.clone(), self.media.track_key.clone(), self.media.title.clone(), artist, self.media.duration_ms);
                    }
                }
                Msg::Cover { key, image } => {
                    let tex = ctx.load_texture("cover", image, egui::TextureOptions::LINEAR);
                    self.cover = Some((key, tex));
                }
                Msg::Lyrics { key, lines } => {
                    if key == self.media.track_key {
                        self.lyrics = lines;
                    }
                }
                Msg::Brightness(v) => self.brightness_percent = v,
                Msg::Prayer { key, times } => {
                    self.prayer_fetch_key.clear();
                    match times {
                        Some(t) => {
                            self.prayer_times = Some(t);
                            self.prayer_key = key;
                        }
                        None => self.prayer_retry_at = now + Duration::from_secs(60),
                    }
                }
                Msg::Weather { key, info } => {
                    self.weather_fetch_key.clear();
                    match info {
                        Some(i) => {
                            self.weather = Some(i);
                            self.weather_key = key;
                        }
                        None => self.weather_retry_at = now + Duration::from_secs(120),
                    }
                }
                Msg::Meeting(m) => self.meeting = m,
                Msg::Clip(t) => {
                    if !self.cfg.clip_on {
                        continue;
                    }
                    self.clips.retain(|c| *c != t);
                    self.clips.insert(0, t);
                    self.clips.truncate(20);
                }
                Msg::Hotkey => sys::screenshot(),
            }
        }
    }

    // ---------------- input: drop file, Ctrl+F, rekam pintasan ----------------

    fn handle_input(&mut self, ctx: &egui::Context, now: Instant) {
        // file yang diseret ke island masuk shelf
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if !dropped.is_empty() {
            for f in dropped {
                if let Some(p) = f.path {
                    let s = p.display().to_string();
                    if !self.data.shelf.contains(&s) {
                        self.data.shelf.push(s);
                    }
                }
            }
            if self.data.shelf.len() > 12 {
                let extra = self.data.shelf.len() - 12;
                self.data.shelf.drain(0..extra);
            }
            self.sync_shelf();
            self.tab = Tab::Shelf;
        }

        // Ctrl+F: fokus ke kotak cari pengaturan
        let ctrl_f = ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::F));
        if ctrl_f && self.mode == Mode::Expanded {
            self.tab = Tab::Settings;
            self.focus_search = true;
        }

        // merekam pintasan tangkap layar baru
        if self.capturing_hotkey {
            let events = ctx.input(|i| i.events.clone());
            for e in events {
                if let egui::Event::Key { key, pressed: true, modifiers, .. } = e {
                    if key == egui::Key::Escape {
                        self.capturing_hotkey = false;
                        break;
                    }
                    let name = format!("{:?}", key);
                    let mut combo = String::new();
                    if modifiers.ctrl {
                        combo.push_str("Ctrl+");
                    }
                    if modifiers.shift {
                        combo.push_str("Shift+");
                    }
                    if modifiers.alt {
                        combo.push_str("Alt+");
                    }
                    combo.push_str(&name);
                    if (modifiers.ctrl || modifiers.shift || modifiers.alt) && sys::parse_combo(&combo).is_some() {
                        self.cfg.hotkey = combo.clone();
                        self.sync_hotkey();
                        self.cfg.save();
                        self.capturing_hotkey = false;
                        break;
                    }
                }
            }
        }

        // roda mouse menggulir daftar (pengaturan / pilih kota)
        if self.hovering && self.mode == Mode::Expanded {
            let dy = ctx.input(|i| i.raw_scroll_delta.y);
            if dy != 0.0 {
                let step: i32 = if dy > 0.0 { -1 } else { 1 };
                if self.tab == Tab::Settings {
                    self.set_scroll = ((self.set_scroll as i32 + step).max(0) as usize).min(20);
                } else if self.tab == Tab::Prayer && self.city_picking {
                    self.city_scroll = ((self.city_scroll as i32 + step).max(0) as usize).min(60);
                }
            }
        }
        let _ = now;
    }

    // ---------------- penerapan aksi ----------------

    fn apply(&mut self, ctx: &egui::Context, a: Action, now: Instant) {
        match a {
            Action::PillClick => {
                if self.mode == Mode::Alert {
                    self.alert_click(now);
                } else if self.mode != Mode::Expanded {
                    self.pinned = true;
                    self.pending = None;
                    self.set_mode(Mode::Expanded, now);
                    ctx.send_viewport_cmd(ViewportCommand::Focus);
                }
            }
            Action::Pin => {
                self.pinned = !self.pinned;
                if !self.pinned && !self.hovering {
                    self.pending = Some((now + Duration::from_millis(350), self.rest_mode()));
                }
                ctx.send_viewport_cmd(ViewportCommand::Focus);
            }
            Action::Tab(t) => {
                self.tab = t;
                self.city_picking = false;
                ctx.send_viewport_cmd(ViewportCommand::Focus);
            }
            Action::Prev => media::control("prev", None),
            Action::Next => media::control("next", None),
            Action::Toggle => {
                media::control("toggle", None);
                self.media.position_ms = self.position_ms(now);
                self.media_at = now;
                self.media.playing = !self.media.playing;
            }
            Action::Seek(ms) => {
                self.media.position_ms = ms;
                self.media_at = now;
                media::control("seek", Some(ms));
            }
            Action::BrightnessValue(_) => {}
            Action::VolumeValue(v) => {
                self.vol_sync_at = now;
                if v != self.volume_percent {
                    self.volume_percent = v;
                    sys::set_volume(v);
                }
            }
            Action::MicToggle => {
                let m = !self.mic_muted;
                sys::set_mic_muted(m);
                self.mic_muted = m;
            }
            Action::Screenshot => sys::screenshot(),
            Action::Record => sys::record_shortcut(),
            Action::AlertClick => self.alert_click(now),
            Action::AlertClose => self.dismiss_alert(),
            Action::PomoToggle => {
                self.pomo.running = !self.pomo.running;
                self.pomo.last = now;
            }
            Action::PomoReset => self.pomo_reset(),
            Action::PomoSkip => {
                if self.pomo.mode == FocusMode::Pomodoro {
                    self.pomo_done();
                }
            }
            Action::PomoMode(m) => {
                self.pomo.mode = m;
                self.pomo_reset();
            }
            Action::TimerAdj(d) => {
                if !self.pomo.running && self.pomo.mode == FocusMode::Timer {
                    self.pomo.timer_min = (self.pomo.timer_min as i32 + d).clamp(1, 180) as u32;
                    self.pomo.remaining_ms = self.pomo_phase_ms();
                }
            }
            Action::Drink(ml) => self.drink(ml),
            Action::Set(id, d) => self.apply_setting(ctx, id, d, now),
            Action::AgAdd => self.add_agenda(),
            Action::AgToggle(id) => {
                if let Some(a) = self.data.agenda.iter_mut().find(|a| a.id == id) {
                    a.done = !a.done;
                }
                self.data.save();
            }
            Action::AgDel(id) => {
                self.data.agenda.retain(|a| a.id != id);
                self.data.save();
            }
            Action::AgTime(d) => {
                let v = self.ag_hm as i32 + d * 5;
                self.ag_hm = v.rem_euclid(24 * 60) as u32;
            }
            Action::AgDay => self.ag_tomorrow = !self.ag_tomorrow,
            Action::EyeSet(m) => {
                if m == 0 {
                    self.cfg.eye_on = false;
                } else {
                    self.cfg.eye_on = true;
                    self.cfg.eye_min = m;
                    self.next_eye_at = now + Duration::from_secs(m as u64 * 60);
                }
                self.cfg.save();
            }
            Action::EyeTest => {
                self.push_alert(AlertKind::Eye, "Istirahatkan matamu".into(), "Lihat objek jauh selama 20 detik".into(), Some(12));
            }
            Action::CalNav(d) => self.cal_off += d,
            Action::CalSel(y) => self.cal_sel = y,
            Action::OpenSettings => self.settings_open = true,
            Action::SetCat(c) => {
                self.set_cat = c;
                self.set_scroll_px = 0.0;
            }
            Action::SetF(id, v) => {
                if id == SetId::LyricOff {
                    let ms = ((v - 0.5) * 10_000.0 / 100.0).round() as i32 * 100;
                    self.cfg.lyric_offset_ms = ms.clamp(-5000, 5000);
                    self.cfg.save();
                }
            }
            Action::IgnoreAdd => {
                let k = self.ignore_input.trim().to_lowercase();
                if !k.is_empty() && !self.cfg.ignore_titles.contains(&k) {
                    self.cfg.ignore_titles.push(k);
                    self.cfg.save();
                    self.sync_ignore();
                }
                self.ignore_input.clear();
            }
            Action::IgnoreDel(i) => {
                if i < self.cfg.ignore_titles.len() {
                    self.cfg.ignore_titles.remove(i);
                    self.cfg.save();
                    self.sync_ignore();
                }
            }
            Action::TestAdzan => sys::play_adzan(),
            Action::OpenDataDir => sys::open_uri(&store::dir().display().to_string()),
            Action::OpenShots => sys::open_uri(&user_dir(r"Pictures\Screenshots")),
            Action::OpenCaps => sys::open_uri(&user_dir(r"Videos\Captures")),
            Action::CapFull => sys::screenshot_full(),
            Action::CapWindow => sys::screenshot_window(),
            Action::ShelfRemove(i) => {
                if i < self.data.shelf.len() {
                    self.data.shelf.remove(i);
                    self.sync_shelf();
                }
            }
            Action::ShelfOpen(i) => {
                if let Some(p) = self.data.shelf.get(i) {
                    sys::open_uri(p);
                }
            }
            Action::ShareToggle => {
                let on = !self.share_on.load(Ordering::Relaxed);
                if on && self.share_url.is_none() {
                    self.share_url = share::start(self.share_files.clone(), self.share_on.clone());
                }
                if on && self.share_url.is_none() {
                    return; // gagal memulai server
                }
                self.share_on.store(on, Ordering::Relaxed);
            }
            Action::ClipCopy(i) => {
                if let Some(t) = self.clips.get(i) {
                    sys::set_clipboard(t);
                }
            }
            Action::ClipClear => self.clips.clear(),
            Action::CityPick(open) => {
                self.city_picking = open;
                self.city_scroll = 0;
                self.city_filter.clear();
            }
            Action::CityChoose(name) => {
                self.cfg.city = name;
                self.cfg.save();
                self.city_picking = false;
                self.prayer_times = None;
                self.weather = None;
                self.prayer_retry_at = now;
                self.weather_retry_at = now;
            }
        }
    }

    fn apply_setting(&mut self, ctx: &egui::Context, id: SetId, d: i32, now: Instant) {
        let step = |v: u32, d: i32, s: u32, lo: u32, hi: u32| -> u32 {
            (v as i32 + d * s as i32).clamp(lo as i32, hi as i32) as u32
        };
        match id {
            SetId::Dark => self.cfg.dark = !self.cfg.dark,
            SetId::Lyrics => self.cfg.lyrics = !self.cfg.lyrics,
            SetId::MicBtn => self.cfg.mic_button = !self.cfg.mic_button,
            SetId::MeetingAuto => self.cfg.meeting_auto = !self.cfg.meeting_auto,
            SetId::Notif => self.cfg.notif = !self.cfg.notif,
            SetId::Hotkey => self.capturing_hotkey = !self.capturing_hotkey,
            SetId::Adzan => self.cfg.adzan = !self.cfg.adzan,
            SetId::AdzanPause => self.cfg.adzan_pause = !self.cfg.adzan_pause,
            SetId::Region => {
                self.cfg.region = if self.cfg.region == "MY" { "ID".into() } else { "MY".into() };
                self.cfg.city = prayer::cities(&self.cfg.region)[0].to_string();
                self.prayer_times = None;
                self.weather = None;
                self.prayer_retry_at = now;
                self.weather_retry_at = now;
            }
            SetId::EyeOn => {
                self.cfg.eye_on = !self.cfg.eye_on;
                self.next_eye_at = now + Duration::from_secs(self.cfg.eye_min as u64 * 60);
            }
            SetId::EyeInt => {
                self.cfg.eye_min = step(self.cfg.eye_min, d, 5, 5, 120);
                self.next_eye_at = now + Duration::from_secs(self.cfg.eye_min as u64 * 60);
            }
            SetId::PomoWork => {
                self.cfg.pomo_work = step(self.cfg.pomo_work, d, 5, 5, 90);
                self.refresh_idle_pomo();
            }
            SetId::PomoShort => {
                self.cfg.pomo_short = step(self.cfg.pomo_short, d, 1, 1, 30);
                self.refresh_idle_pomo();
            }
            SetId::PomoLong => {
                self.cfg.pomo_long = step(self.cfg.pomo_long, d, 5, 5, 60);
                self.refresh_idle_pomo();
            }
            SetId::Autostart => {
                self.cfg.autostart = !self.cfg.autostart;
                sys::set_autostart(self.cfg.autostart);
            }
            SetId::OpenWin => sys::open_uri("ms-settings:"),
            SetId::Quit => ctx.send_viewport_cmd(ViewportCommand::Close),
            SetId::WaterOn => {
                self.cfg.water_on = !self.cfg.water_on;
                self.next_water_at = now + Duration::from_secs(self.cfg.water_min as u64 * 60);
            }
            SetId::WaterInt => {
                self.cfg.water_min = step(self.cfg.water_min, d, 15, 15, 240);
                self.next_water_at = now + Duration::from_secs(self.cfg.water_min as u64 * 60);
            }
            SetId::Weather => {
                self.cfg.weather_on = !self.cfg.weather_on;
                if self.cfg.weather_on {
                    self.weather_retry_at = now;
                } else {
                    self.weather = None;
                }
            }
            SetId::Clip => {
                self.cfg.clip_on = !self.cfg.clip_on;
                if !self.cfg.clip_on {
                    self.clips.clear();
                }
            }
            SetId::Robot => self.cfg.robot_on = !self.cfg.robot_on,
            SetId::LyricOff => {}
            SetId::HotkeyOn => {
                self.cfg.hotkey_on = !self.cfg.hotkey_on;
                self.sync_hotkey();
            }
            SetId::HotkeyReset => {
                self.cfg.hotkey = "Ctrl+Shift+S".into();
                self.cfg.hotkey_on = true;
                self.sync_hotkey();
            }
            SetId::ThemeDark => self.cfg.dark = true,
            SetId::ThemeLight => self.cfg.dark = false,
            SetId::CityStep => {
                let list = prayer::cities(&self.cfg.region);
                let cur = list.iter().position(|c| *c == self.cfg.city).unwrap_or(0) as i32;
                let n = list.len() as i32;
                let next = (cur + d).rem_euclid(n) as usize;
                self.cfg.city = list[next].to_string();
                self.prayer_times = None;
                self.weather = None;
                self.prayer_retry_at = now;
                self.weather_retry_at = now;
            }
            SetId::WaterGoal => self.cfg.water_goal = step(self.cfg.water_goal, d, 250, 500, 5000),
            SetId::WaterStart => {
                self.cfg.water_start = step(self.cfg.water_start, d, 1, 0, self.cfg.water_end.saturating_sub(1));
            }
            SetId::WaterEnd => {
                self.cfg.water_end = step(self.cfg.water_end, d, 1, self.cfg.water_start + 1, 24);
            }
        }
        self.cfg.save();
    }

    fn sync_ignore(&mut self) {
        if let Ok(mut g) = self.ignore_shared.lock() {
            *g = self.cfg.ignore_titles.clone();
        }
    }

    pub fn sync_hotkey(&mut self) {
        if let Ok(mut g) = self.hotkey_shared.lock() {
            *g = if self.cfg.hotkey_on { self.cfg.hotkey.clone() } else { String::new() };
        }
    }

    fn refresh_idle_pomo(&mut self) {
        if !self.pomo.running {
            self.pomo.remaining_ms = self.pomo_phase_ms();
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _v: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _f: &mut eframe::Frame) {
        let now = Instant::now();
        let ppp = ctx.pixels_per_point();

        // 1) taruh window di tengah atas layar (beberapa frame pertama, tanpa menggambar)
        if self.placed_frames < 3 {
            let x = (sys::screen_width_px() / ppp - WIN_W) / 2.0;
            ctx.send_viewport_cmd(ViewportCommand::OuterPosition(pos2(x.max(0.0), 0.0)));
            self.placed_frames += 1;
            ctx.request_repaint();
            return;
        }

        // 2) pesan thread latar, pekerjaan berkala, input
        self.drain_msgs(ctx, now);
        if now.duration_since(self.battery_at) > Duration::from_secs(20) {
            self.battery = sys::battery();
            self.battery_at = now;
        }
        if self.mode == Mode::Expanded && now.duration_since(self.vol_sync_at) > Duration::from_secs(1) {
            if let Some(v) = sys::get_volume() {
                self.volume_percent = v;
            }
            self.vol_sync_at = now;
        }
        if self.mode == Mode::Expanded && now.duration_since(self.mic_at) > Duration::from_secs(1) {
            if let Some(m) = sys::mic_muted() {
                self.mic_muted = m;
            }
            self.mic_at = now;
        }
        self.slow_tick(now);
        self.handle_input(ctx, now);

        // 3) hover manual (window click-through, jadi event mouse tidak datang dari OS)
        let typing = ctx.wants_keyboard_input();
        let origin = ctx
            .input(|i| i.viewport().outer_rect.or(i.viewport().inner_rect))
            .map(|r| r.min)
            .unwrap_or(pos2(0.0, 0.0));
        let [_, th_h, th_top] = self.mode.geom();
        let hw = self.mode.hit_w();
        let hit = Rect::from_min_max(pos2((WIN_W - hw) / 2.0, 0.0), pos2((WIN_W + hw) / 2.0, th_top + th_h + 2.0));
        let inside = sys::cursor_pos()
            .map(|(x, y)| hit.contains(pos2(x / ppp - origin.x, y / ppp - origin.y)))
            .unwrap_or(false);
        if inside != self.hovering {
            self.on_hover(inside, now, typing);
        }
        if self.passthrough == inside {
            self.passthrough = !inside;
            ctx.send_viewport_cmd(ViewportCommand::MousePassthrough(self.passthrough));
        }

        self.tick(now);

        // 4) gambar + kumpulkan aksi
        let mut acts: Vec<Action> = Vec::new();
        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
            acts = self.paint(ui, now);
        });
        for a in acts {
            self.apply(ctx, a, now);
        }

        if self.settings_open {
            let sacts = self.show_settings(ctx);
            for a in sacts {
                self.apply(ctx, a, now);
            }
        }

        // 5) jadwalkan frame berikutnya
        let ms = if self.tween.running(now) {
            16
        } else if self.mode == Mode::Expanded {
            if self.media.playing || self.pomo.running { 50 } else { 100 }
        } else if self.mode == Mode::Music && self.media.playing {
            60
        } else {
            50 // kursor dipantau manual, jadi tidak boleh terlalu jarang
        };
        ctx.request_repaint_after(Duration::from_millis(ms));
    }
}

fn user_dir(sub: &str) -> String {
    match std::env::var_os("USERPROFILE") {
        Some(p) => PathBuf::from(p).join(sub).display().to_string(),
        None => sub.to_string(),
    }
}

fn log_error(msg: &str) {
    use std::io::Write;
    let dir = store::dir();
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("error.log")) {
        let _ = writeln!(f, "{msg}");
    }
}

fn main() -> eframe::Result<()> {
    std::panic::set_hook(Box::new(|info| log_error(&format!("PANIC: {info}"))));
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("RafLand")
            .with_inner_size([WIN_W, WIN_H])
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_resizable(false)
            .with_taskbar(false)
            .with_mouse_passthrough(true),
        ..Default::default()
    };
    let result = eframe::run_native("rafland", options, Box::new(|cc| Ok(Box::new(App::new(cc)))));
    if let Err(e) = &result {
        log_error(&format!("ERROR: {e}"));
    }
    result
}
