// Jendela Pengaturan RafLand (viewport terpisah): sidebar kategori + kotak cari + kartu pengaturan.
// Semua opsi di sini terhubung ke fitur yang benar-benar ada.

use super::*;
use crate::draw::{self, text};
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontFamily, FontId, Painter, Rect, Sense, Stroke};

#[derive(Clone, Copy, PartialEq)]
pub enum Cat {
    Umum,
    Tampilan,
    Musik,
    Hari,
    Sholat,
    Robot,
    Tangkap,
    Shelf,
    Tentang,
}

const CATS: [(Cat, &str); 9] = [
    (Cat::Umum, "Umum"),
    (Cat::Tampilan, "Tampilan"),
    (Cat::Musik, "Musik & Audio"),
    (Cat::Hari, "Hari ini"),
    (Cat::Sholat, "Sholat"),
    (Cat::Robot, "Robot"),
    (Cat::Tangkap, "Tangkap Layar"),
    (Cat::Shelf, "Shelf & Clipboard"),
    (Cat::Tentang, "Tentang"),
];

#[derive(Clone)]
enum Ctl {
    Toggle(bool, Action),
    Btn(String, Action, bool),
    Btns(Vec<(String, Action, bool)>),
    Seg(Vec<(String, Action)>, usize),
    Slider(f32, SetId),
    Step(String, SetId),
    Value(String),
    Input,
}

#[derive(Clone)]
struct Row {
    label: String,
    sub: String,
    ctl: Ctl,
}

#[derive(Clone)]
enum Block {
    Sec(String),
    Card(Vec<Row>),
    Note(String),
}

#[derive(Clone, Copy)]
struct Pal {
    bg: Color32,
    side: Color32,
    card: Color32,
    line: Color32,
    hover: Color32,
    fg: Color32,
    sub: Color32,
    accent: Color32,
    off: Color32,
    chip: Color32,
}

fn pal(dark: bool) -> Pal {
    let c = |r, g, b| Color32::from_rgb(r, g, b);
    if dark {
        Pal {
            bg: c(20, 20, 28),
            side: c(25, 25, 35),
            card: c(35, 35, 48),
            line: c(56, 56, 76),
            hover: c(46, 46, 64),
            fg: c(244, 245, 250),
            sub: c(160, 164, 182),
            accent: c(176, 148, 255),
            off: c(80, 80, 100),
            chip: c(56, 56, 76),
        }
    } else {
        Pal {
            bg: c(244, 244, 249),
            side: c(236, 236, 244),
            card: c(255, 255, 255),
            line: c(214, 214, 228),
            hover: c(232, 232, 242),
            fg: c(24, 24, 32),
            sub: c(100, 104, 122),
            accent: c(124, 92, 230),
            off: c(190, 190, 206),
            chip: c(228, 228, 238),
        }
    }
}

fn row(label: &str, sub: &str, ctl: Ctl) -> Row {
    Row { label: label.to_string(), sub: sub.to_string(), ctl }
}

fn tog(on: bool, id: SetId) -> Ctl {
    Ctl::Toggle(on, Action::Set(id, 0))
}

fn text_w(p: &Painter, s: &str, size: f32, bold: bool) -> f32 {
    let fam = if bold { FontFamily::Name("bold".into()) } else { FontFamily::Proportional };
    p.layout_no_wrap(s.to_string(), FontId::new(size, fam), Color32::WHITE).size().x
}

fn ctl_width(p: &Painter, c: &Ctl) -> f32 {
    match c {
        Ctl::Toggle(..) => 44.0,
        Ctl::Btn(l, _, _) => (text_w(p, l, 11.5, true) + 30.0).max(76.0),
        Ctl::Btns(v) => v.iter().map(|b| (text_w(p, &b.0, 11.5, true) + 30.0).max(76.0) + 8.0).sum::<f32>() - 8.0,
        Ctl::Seg(v, _) => v.iter().map(|b| (text_w(p, &b.0, 11.5, true) + 30.0).max(76.0)).sum::<f32>(),
        Ctl::Slider(..) => 260.0,
        Ctl::Step(..) => 190.0,
        Ctl::Value(v) => text_w(p, v, 12.0, false).min(360.0),
        Ctl::Input => 330.0,
    }
}

/// (diklik, sedang di-hover); tidak ada interaksi bila area di luar jendela gulir.
fn hit(ui: &egui::Ui, clip: Rect, r: Rect, id: &str) -> (bool, bool) {
    if !clip.intersects(r) {
        return (false, false);
    }
    let resp = ui.interact(r, ui.id().with(id), Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    (resp.clicked(), resp.hovered())
}

fn side_icon(p: &Painter, cat: Cat, c: egui::Pos2, col: Color32) {
    let st = Stroke::new(1.5_f32, col);
    match cat {
        Cat::Umum => draw::nav_icon(p, 8, c, 15.0, col),
        Cat::Musik => draw::nav_icon(p, 0, c, 15.0, col),
        Cat::Hari => draw::nav_icon(p, 1, c, 15.0, col),
        Cat::Sholat => draw::nav_icon(p, 2, c, 15.0, col),
        Cat::Tangkap => draw::nav_icon(p, 7, c, 15.0, col),
        Cat::Shelf => draw::nav_icon(p, 3, c, 15.0, col),
        Cat::Tampilan => {
            p.circle_stroke(c, 7.0, st);
            p.circle_filled(c + vec2(-2.5, -2.0), 1.8, col);
            p.circle_filled(c + vec2(2.5, -2.0), 1.8, col);
            p.circle_filled(c + vec2(0.0, 3.0), 1.8, col);
        }
        Cat::Robot => {
            p.rect_stroke(Rect::from_center_size(c + vec2(0.0, 1.0), vec2(15.0, 12.0)), 4.0, st);
            p.circle_filled(c + vec2(-3.0, 1.0), 1.6, col);
            p.circle_filled(c + vec2(3.0, 1.0), 1.6, col);
            p.line_segment([c + vec2(0.0, -5.0), c + vec2(0.0, -8.0)], st);
        }
        Cat::Tentang => {
            p.circle_stroke(c, 7.5, st);
            p.circle_filled(c + vec2(0.0, -3.0), 1.2, col);
            p.line_segment([c + vec2(0.0, -0.5), c + vec2(0.0, 4.0)], st);
        }
    }
}

fn fmt_off(ms: i32) -> String {
    format!("{}{},{} s", if ms < 0 { "-" } else { "" }, ms.abs() / 1000, (ms.abs() % 1000) / 100)
}

impl App {
    /// Isi tiap kategori: (judul, subjudul, blok).
    fn settings_page(&self, cat: Cat) -> (String, String, Vec<Block>) {
        let c = &self.cfg;
        let mut b: Vec<Block> = Vec::new();
        let (title, subtitle) = match cat {
            Cat::Umum => ("Umum", "Perilaku dasar aplikasi."),
            Cat::Tampilan => ("Tampilan", "Tema island dan jendela."),
            Cat::Musik => ("Musik & Audio", "Lirik, mode rapat, dan daftar jendela yang diabaikan."),
            Cat::Hari => ("Hari ini", "Lokasi, cuaca, dan pengingat harian."),
            Cat::Sholat => ("Sholat", "Jadwal, pengingat, dan suara adzan."),
            Cat::Robot => ("Robot", "Maskot kecil di pill island."),
            Cat::Tangkap => ("Tangkap Layar", "Pintasan dan lokasi hasil tangkapan."),
            Cat::Shelf => ("Shelf & Clipboard", "File sementara, riwayat salin, dan kirim ke HP."),
            Cat::Tentang => ("Tentang", "Informasi aplikasi."),
        };
        match cat {
            Cat::Umum => {
                b.push(Block::Sec("Aplikasi".into()));
                b.push(Block::Card(vec![
                    row("Jalankan saat Windows mulai", "RafLand otomatis aktif setelah kamu login.", tog(c.autostart, SetId::Autostart)),
                    row("Notifikasi ringkas", "Pengingat, timer, dan adzan muncul sebagai pill di island.", tog(c.notif, SetId::Notif)),
                ]));
                b.push(Block::Sec("Lainnya".into()));
                b.push(Block::Card(vec![
                    row("Pengaturan Windows", "Buka aplikasi Pengaturan milik Windows.", Ctl::Btn("Buka".into(), Action::Set(SetId::OpenWin, 0), false)),
                    row("Keluar dari RafLand", "Tutup island sampai dijalankan lagi.", Ctl::Btn("Keluar".into(), Action::Set(SetId::Quit, 0), false)),
                ]));
            }
            Cat::Tampilan => {
                b.push(Block::Sec("Tema".into()));
                b.push(Block::Card(vec![row(
                    "Warna tema",
                    "Berlaku untuk island dan jendela pengaturan ini.",
                    Ctl::Seg(
                        vec![("Gelap".into(), Action::Set(SetId::ThemeDark, 0)), ("Terang".into(), Action::Set(SetId::ThemeLight, 0))],
                        if c.dark { 0 } else { 1 },
                    ),
                )]));
            }
            Cat::Musik => {
                b.push(Block::Sec("Lirik".into()));
                b.push(Block::Card(vec![
                    row("Lirik tersinkron", "Tampilkan lirik berjalan di island saat musik diputar.", tog(c.lyrics, SetId::Lyrics)),
                    row(
                        &format!("Geser waktu lirik  ({})", fmt_off(c.lyric_offset_ms)),
                        "Geser ke kiri bila lirik terlalu cepat, ke kanan bila terlalu lambat.",
                        Ctl::Slider((c.lyric_offset_ms as f32 / 10_000.0) + 0.5, SetId::LyricOff),
                    ),
                ]));
                b.push(Block::Sec("Mode rapat".into()));
                b.push(Block::Card(vec![
                    row("Deteksi rapat otomatis", "Mengenali Zoom, Meet, dan Teams dari judul jendela. Telepon WhatsApp tidak dianggap rapat.", tog(c.meeting_auto, SetId::MeetingAuto)),
                    row("Tombol mic saat rapat", "Tampilkan tombol bisukan mikrofon di halaman Musik saat rapat berlangsung.", tog(c.mic_button, SetId::MicBtn)),
                ]));
                b.push(Block::Note("Rapat di browser hanya terdeteksi saat tab rapatnya yang sedang aktif.".into()));
                b.push(Block::Sec("Abaikan jendela saat rapat".into()));
                let mut rows = vec![row(
                    "Kata kunci judul jendela",
                    "Jendela yang judulnya memuat kata ini tidak dihitung sebagai rapat, mis. discord.",
                    Ctl::Input,
                )];
                for (i, k) in c.ignore_titles.iter().enumerate() {
                    rows.push(row(k, "", Ctl::Btn("Hapus".into(), Action::IgnoreDel(i), false)));
                }
                b.push(Block::Card(rows));
            }
            Cat::Hari => {
                b.push(Block::Sec("Lokasi".into()));
                b.push(Block::Card(vec![
                    row(
                        "Wilayah",
                        "Menentukan metode hitung sholat dan daftar kota.",
                        Ctl::Seg(
                            vec![("Indonesia".into(), Action::Set(SetId::Region, 0)), ("Malaysia".into(), Action::Set(SetId::Region, 0))],
                            if c.region == "MY" { 1 } else { 0 },
                        ),
                    ),
                    row("Kota", &format!("{} kota tersedia untuk wilayah ini.", prayer::cities(&c.region).len()), Ctl::Step(c.city.clone(), SetId::CityStep)),
                    row("Tampilkan cuaca", "Mengambil data dari open-meteo.com dengan nama kota yang dipilih.", tog(c.weather_on, SetId::Weather)),
                ]));
                b.push(Block::Sec("Pengingat istirahat mata".into()));
                b.push(Block::Card(vec![
                    row("Aktifkan", "Ingatkan untuk melihat objek jauh secara berkala.", tog(c.eye_on, SetId::EyeOn)),
                    row("Selang", "", Ctl::Step(format!("{} menit", c.eye_min), SetId::EyeInt)),
                ]));
                b.push(Block::Sec("Pengingat minum".into()));
                b.push(Block::Card(vec![
                    row("Aktifkan", "Awalnya mati. Klik notifikasinya untuk mencatat satu gelas.", tog(c.water_on, SetId::WaterOn)),
                    row("Selang", "", Ctl::Step(format!("{} menit", c.water_min), SetId::WaterInt)),
                    row("Target harian", "", Ctl::Step(format!("{} ml", c.water_goal), SetId::WaterGoal)),
                    row("Mulai", "", Ctl::Step(format!("{:02}.00", c.water_start), SetId::WaterStart)),
                    row("Sampai", "", Ctl::Step(format!("{:02}.00", c.water_end), SetId::WaterEnd)),
                ]));
                b.push(Block::Sec("Pomodoro".into()));
                b.push(Block::Card(vec![
                    row("Fokus", "", Ctl::Step(format!("{} menit", c.pomo_work), SetId::PomoWork)),
                    row("Istirahat pendek", "", Ctl::Step(format!("{} menit", c.pomo_short), SetId::PomoShort)),
                    row("Istirahat panjang", "Setiap empat sesi fokus.", Ctl::Step(format!("{} menit", c.pomo_long), SetId::PomoLong)),
                ]));
            }
            Cat::Sholat => {
                b.push(Block::Sec("Pengingat".into()));
                b.push(Block::Card(vec![
                    row("Adzan saat waktu masuk", "Mainkan suara dan tampilkan notifikasi di island.", tog(c.adzan, SetId::Adzan)),
                    row("Jeda musik saat adzan", "Musik tidak lanjut sendiri; island menawarkan memutarnya lagi.", tog(c.adzan_pause, SetId::AdzanPause)),
                ]));
                b.push(Block::Note(
                    "Jadwal dihitung lewat layanan Aladhan (butuh internet): metode Kemenag untuk Indonesia dan sudut JAKIM untuk Malaysia. Nama waktu mengikuti wilayah (Dzuhur/Ashar/Isya atau Zohor/Asar/Isyak).".into(),
                ));
                b.push(Block::Sec("Suara adzan".into()));
                b.push(Block::Card(vec![
                    row("Berkas suara", "Taruh adzan.wav di folder data RafLand. Bila tidak ada, dipakai bunyi sistem.", Ctl::Btns(vec![
                        ("Tes".into(), Action::TestAdzan, false),
                        ("Buka folder".into(), Action::OpenDataDir, false),
                    ])),
                ]));
            }
            Cat::Robot => {
                b.push(Block::Card(vec![row(
                    "Tampilkan robot",
                    "Robot di pill berkedip, tersenyum saat musik, dan kehausan saat pengingat minum.",
                    tog(c.robot_on, SetId::Robot),
                )]));
            }
            Cat::Tangkap => {
                b.push(Block::Sec("Pintasan".into()));
                let hot = if self.capturing_hotkey { "Tekan tombol...".to_string() } else { c.hotkey.replace('+', " + ") };
                b.push(Block::Card(vec![row(
                    "Buka tangkap layar",
                    "Klik tombol pintasan lalu tekan kombinasi baru (wajib memakai Ctrl, Shift, atau Alt). Esc untuk batal.",
                    Ctl::Btns(vec![
                        (hot, Action::Set(SetId::Hotkey, 0), true),
                        ("Bawaan".into(), Action::Set(SetId::HotkeyReset, 0), false),
                        (if c.hotkey_on { "Matikan".to_string() } else { "Aktifkan".to_string() }, Action::Set(SetId::HotkeyOn, 0), false),
                    ]),
                )]));
                b.push(Block::Sec("Lokasi file".into()));
                b.push(Block::Card(vec![
                    row("Screenshot", "Gambar\\Screenshots", Ctl::Btn("Buka".into(), Action::OpenShots, false)),
                    row("Rekaman layar", "Video\\Captures", Ctl::Btn("Buka".into(), Action::OpenCaps, false)),
                ]));
            }
            Cat::Shelf => {
                b.push(Block::Sec("Clipboard".into()));
                b.push(Block::Card(vec![row(
                    "Riwayat clipboard",
                    "Menyimpan 20 teks terakhir yang kamu salin, hanya di memori dan hilang saat RafLand ditutup.",
                    tog(c.clip_on, SetId::Clip),
                )]));
                b.push(Block::Sec("Kirim ke HP".into()));
                let on = self.share_on.load(Ordering::Relaxed);
                let mut rows = vec![row(
                    "Bagikan file shelf",
                    "Server lokal untuk jaringan Wi-Fi yang sama. Windows mungkin meminta izin firewall.",
                    Ctl::Toggle(on, Action::ShareToggle),
                )];
                if let (true, Some(u)) = (on, &self.share_url) {
                    rows.push(row("Alamat", "Buka alamat ini di browser HP.", Ctl::Value(u.clone())));
                }
                b.push(Block::Card(rows));
            }
            Cat::Tentang => {
                b.push(Block::Card(vec![
                    row("Versi", "", Ctl::Value("0.1.0".into())),
                    row("Folder data", &store::dir().display().to_string(), Ctl::Btn("Buka".into(), Action::OpenDataDir, false)),
                ]));
                b.push(Block::Note("RafLand dibuat dengan Rust dan egui. Kode dasar island berasal dari proyek azay.".into()));
            }
        }
        (title.to_string(), subtitle.to_string(), b)
    }

    /// Tampilkan jendela pengaturan; mengembalikan aksi yang harus diterapkan.
    pub fn show_settings(&mut self, ctx: &egui::Context) -> Vec<Action> {
        let mut acts: Vec<Action> = Vec::new();
        let mut close = false;
        let bg = pal(self.cfg.dark).bg;
        let id = egui::ViewportId::from_hash_of("rafland_settings");
        let builder = egui::ViewportBuilder::default()
            .with_title("Pengaturan RafLand")
            .with_inner_size([980.0, 640.0])
            .with_min_inner_size([760.0, 480.0]);
        ctx.show_viewport_immediate(id, builder, |ctx, _class| {
            if ctx.input(|i| i.viewport().close_requested()) {
                close = true;
            }
            if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::F)) {
                self.focus_search = true;
            }
            self.capture_hotkey(ctx);
            egui::CentralPanel::default().frame(egui::Frame::none().fill(bg)).show(ctx, |ui| {
                self.paint_settings(ui, &mut acts);
            });
        });
        if close {
            self.settings_open = false;
            self.capturing_hotkey = false;
        }
        acts
    }

    /// Menangkap kombinasi tombol baru untuk pintasan tangkap layar.
    fn capture_hotkey(&mut self, ctx: &egui::Context) {
        if !self.capturing_hotkey {
            return;
        }
        let events = ctx.input(|i| i.events.clone());
        for e in events {
            if let egui::Event::Key { key, pressed: true, modifiers, .. } = e {
                if key == egui::Key::Escape {
                    self.capturing_hotkey = false;
                    return;
                }
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
                combo.push_str(&format!("{:?}", key));
                if (modifiers.ctrl || modifiers.shift || modifiers.alt) && sys::parse_combo(&combo).is_some() {
                    self.cfg.hotkey = combo;
                    self.cfg.hotkey_on = true;
                    self.sync_hotkey();
                    self.cfg.save();
                    self.capturing_hotkey = false;
                    return;
                }
            }
        }
    }

    fn paint_settings(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let pal = pal(self.cfg.dark);
        let full = ui.max_rect();
        let side = Rect::from_min_max(full.min, pos2(full.min.x + 240.0, full.max.y));
        let content = Rect::from_min_max(pos2(side.max.x, full.min.y), full.max);

        // roda mouse menggulir isi
        if ui.rect_contains_pointer(content) {
            let dy = ui.input(|i| i.raw_scroll_delta.y);
            let max_scroll = (self.set_content_h - content.height()).max(0.0);
            self.set_scroll_px = (self.set_scroll_px - dy).clamp(0.0, max_scroll);
        }

        let painter = ui.painter().clone();
        painter.rect_filled(full, 0.0, pal.bg);
        painter.rect_filled(side, 0.0, pal.side);
        painter.line_segment([side.right_top(), side.right_bottom()], Stroke::new(1.0_f32, pal.line));

        let query = self.search.trim().to_lowercase();
        let mut edits: Vec<(u8, Rect)> = Vec::new();
        let content_h: f32;

        // halaman yang ditampilkan (hasil pencarian atau kategori terpilih)
        let page: (String, String, Vec<Block>) = if query.is_empty() {
            self.settings_page(self.set_cat)
        } else {
            let mut blocks: Vec<Block> = Vec::new();
            let mut n = 0;
            for (cat, name) in CATS.iter() {
                let (_, _, bl) = self.settings_page(*cat);
                let mut found: Vec<Row> = Vec::new();
                for b in bl {
                    if let Block::Card(rows) = b {
                        for r in rows {
                            if r.label.to_lowercase().contains(&query) || r.sub.to_lowercase().contains(&query) {
                                found.push(r);
                            }
                        }
                    }
                }
                if !found.is_empty() {
                    n += found.len();
                    blocks.push(Block::Sec(name.to_string()));
                    blocks.push(Block::Card(found));
                }
            }
            ("Hasil pencarian".to_string(), format!("{} pengaturan cocok dengan \"{}\"", n, query), blocks)
        };

        {
            let u: &egui::Ui = &*ui;

            // ---------- sidebar ----------
            let sb = Rect::from_min_size(pos2(side.min.x + 14.0, side.min.y + 18.0), vec2(212.0, 34.0));
            painter.rect_filled(sb, 10.0, pal.card);
            painter.rect_stroke(sb, 10.0, Stroke::new(1.0_f32, pal.line));
            edits.push((0, sb));
            let mut y = side.min.y + 66.0;
            for (cat, name) in CATS.iter() {
                let r = Rect::from_min_size(pos2(side.min.x + 14.0, y), vec2(212.0, 38.0));
                let (clicked, hov) = hit(u, side, r, &format!("cat_{name}"));
                let active = self.set_cat == *cat && query.is_empty();
                if active {
                    painter.rect_filled(r, 10.0, pal.accent.gamma_multiply(0.28));
                } else if hov {
                    painter.rect_filled(r, 10.0, pal.hover);
                }
                let col = if active { pal.fg } else { pal.sub };
                side_icon(&painter, *cat, pos2(r.min.x + 22.0, r.center().y), col);
                text(&painter, pos2(r.min.x + 46.0, r.center().y), Align2::LEFT_CENTER, name, 13.0, active, col);
                if clicked {
                    acts.push(Action::SetCat(*cat));
                    self.search.clear();
                }
                y += 42.0;
            }
            text(&painter, pos2(side.min.x + 20.0, side.max.y - 20.0), Align2::LEFT_CENTER, "RafLand 0.1.0", 10.5, false, pal.sub);

            // ---------- isi ----------
            let p = painter.with_clip_rect(content);
            let cx0 = content.min.x + 28.0;
            let cx1 = content.max.x - 28.0;
            let cw = cx1 - cx0;
            let mut y = content.min.y + 26.0 - self.set_scroll_px;
            text(&p, pos2(cx0, y), Align2::LEFT_TOP, &page.0, 23.0, true, pal.fg);
            text(&p, pos2(cx0, y + 33.0), Align2::LEFT_TOP, &page.1, 12.0, false, pal.sub);
            y += 72.0;

            for (bi, block) in page.2.iter().enumerate() {
                match block {
                    Block::Sec(s) => {
                        text(&p, pos2(cx0 + 2.0, y), Align2::LEFT_TOP, s, 13.0, true, pal.fg);
                        y += 26.0;
                    }
                    Block::Note(s) => {
                        let g = p.layout(s.clone(), FontId::new(11.5, FontFamily::Proportional), pal.sub, cw - 4.0);
                        let h = g.size().y;
                        p.galley(pos2(cx0 + 2.0, y), g, pal.sub);
                        y += h + 18.0;
                    }
                    Block::Card(rows) => {
                        // ukur tinggi tiap baris
                        let mut hs: Vec<f32> = Vec::new();
                        for r in rows.iter() {
                            let tw = cw - 32.0 - ctl_width(&p, &r.ctl) - 24.0;
                            let sub_h = if r.sub.is_empty() {
                                0.0
                            } else {
                                p.layout(r.sub.clone(), FontId::new(11.5, FontFamily::Proportional), pal.sub, tw.max(80.0)).size().y + 4.0
                            };
                            hs.push(if r.sub.is_empty() { 52.0 } else { (14.0 + 18.0 + sub_h + 14.0).max(56.0) });
                        }
                        let total: f32 = hs.iter().sum();
                        let card = Rect::from_min_size(pos2(cx0, y), vec2(cw, total));
                        p.rect_filled(card, 14.0, pal.card);
                        p.rect_stroke(card, 14.0, Stroke::new(1.0_f32, pal.line));

                        let mut ry = y;
                        for (ri, r) in rows.iter().enumerate() {
                            let h = hs[ri];
                            if ri > 0 {
                                p.line_segment([pos2(cx0 + 14.0, ry), pos2(cx1 - 14.0, ry)], Stroke::new(1.0_f32, pal.line));
                            }
                            let cy = ry + h / 2.0;
                            let cwid = ctl_width(&p, &r.ctl);
                            let tw = (cw - 32.0 - cwid - 24.0).max(80.0);
                            if r.sub.is_empty() {
                                text(&p, pos2(cx0 + 16.0, cy), Align2::LEFT_CENTER, &r.label, 13.5, true, pal.fg);
                            } else {
                                text(&p, pos2(cx0 + 16.0, ry + 14.0), Align2::LEFT_TOP, &r.label, 13.5, true, pal.fg);
                                let g = p.layout(r.sub.clone(), FontId::new(11.5, FontFamily::Proportional), pal.sub, tw);
                                p.galley(pos2(cx0 + 16.0, ry + 36.0), g, pal.sub);
                            }
                            let right = cx1 - 16.0;
                            let rid = format!("s{bi}_{ri}");
                            self.draw_ctl(u, &p, &pal, content, &r.ctl, right, cy, &rid, acts, &mut edits);
                            ry += h;
                        }
                        y += total + 18.0;
                    }
                }
            }
            content_h = y + self.set_scroll_px - content.min.y + 16.0;
        }
        self.set_content_h = content_h;

        // kolom teks sungguhan ditaruh terakhir
        for (k, r) in edits {
            let font = FontId::new(12.0, FontFamily::Proportional);
            if k == 0 {
                let resp = ui.put(
                    r,
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text("Cari pengaturan (Ctrl+F)")
                        .frame(false)
                        .text_color(pal.fg)
                        .font(font)
                        .margin(vec2(12.0, 8.0)),
                );
                if self.focus_search {
                    resp.request_focus();
                    self.focus_search = false;
                }
            } else if content.intersects(r) {
                let resp = ui.put(
                    r,
                    egui::TextEdit::singleline(&mut self.ignore_input)
                        .hint_text("mis. discord")
                        .frame(false)
                        .text_color(pal.fg)
                        .font(font)
                        .margin(vec2(12.0, 8.0)),
                );
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    acts.push(Action::IgnoreAdd);
                    resp.request_focus();
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_ctl(
        &self,
        u: &egui::Ui,
        p: &Painter,
        pal: &Pal,
        clip: Rect,
        ctl: &Ctl,
        right: f32,
        cy: f32,
        id: &str,
        acts: &mut Vec<Action>,
        edits: &mut Vec<(u8, Rect)>,
    ) {
        match ctl {
            Ctl::Toggle(on, act) => {
                let r = Rect::from_center_size(pos2(right - 22.0, cy), vec2(44.0, 24.0));
                let (clicked, _) = hit(u, clip, r, id);
                p.rect_filled(r, 12.0, if *on { pal.accent } else { pal.off });
                let kx = if *on { r.max.x - 12.0 } else { r.min.x + 12.0 };
                p.circle_filled(pos2(kx, cy), 9.0, Color32::WHITE);
                if clicked {
                    acts.push(act.clone());
                }
            }
            Ctl::Btn(label, act, primary) => {
                let w = (text_w(p, label, 11.5, true) + 30.0).max(76.0);
                let r = Rect::from_center_size(pos2(right - w / 2.0, cy), vec2(w, 30.0));
                self.small_btn(u, p, pal, clip, r, label, *primary, id, act, acts);
            }
            Ctl::Btns(list) => {
                let mut x = right;
                for (i, (label, act, primary)) in list.iter().enumerate().rev() {
                    let w = (text_w(p, label, 11.5, true) + 30.0).max(76.0);
                    let r = Rect::from_center_size(pos2(x - w / 2.0, cy), vec2(w, 30.0));
                    self.small_btn(u, p, pal, clip, r, label, *primary, &format!("{id}_{i}"), act, acts);
                    x -= w + 8.0;
                }
            }
            Ctl::Seg(list, sel) => {
                let widths: Vec<f32> = list.iter().map(|b| (text_w(p, &b.0, 11.5, true) + 30.0).max(76.0)).collect();
                let total: f32 = widths.iter().sum();
                let bar = Rect::from_min_size(pos2(right - total, cy - 16.0), vec2(total, 32.0));
                p.rect_filled(bar, 10.0, pal.chip);
                let mut x = bar.min.x;
                for (i, (label, act)) in list.iter().enumerate() {
                    let r = Rect::from_min_size(pos2(x, bar.min.y), vec2(widths[i], 32.0));
                    let (clicked, hov) = hit(u, clip, r, &format!("{id}_{i}"));
                    if i == *sel {
                        p.rect_filled(r.shrink(2.0), 8.0, pal.accent);
                    } else if hov {
                        p.rect_filled(r.shrink(2.0), 8.0, pal.hover);
                    }
                    text(p, r.center(), Align2::CENTER_CENTER, label, 11.5, true, if i == *sel { Color32::WHITE } else { pal.sub });
                    if clicked && i != *sel {
                        acts.push(act.clone());
                    }
                    x += widths[i];
                }
            }
            Ctl::Slider(frac, sid) => {
                let tr = Rect::from_min_size(pos2(right - 260.0, cy - 3.0), vec2(260.0, 6.0));
                let hitr = tr.expand2(vec2(8.0, 12.0));
                p.rect_filled(tr, 3.0, pal.off);
                let f = frac.clamp(0.0, 1.0);
                p.rect_filled(Rect::from_min_size(tr.min, vec2(tr.width() * f, 6.0)), 3.0, pal.accent);
                p.circle_filled(pos2(tr.min.x + tr.width() * f, cy), 8.0, Color32::WHITE);
                if clip.intersects(hitr) {
                    let resp = u.interact(hitr, u.id().with(id), Sense::click_and_drag());
                    if resp.hovered() || resp.dragged() {
                        u.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if resp.clicked() || resp.dragged() {
                        if let Some(pt) = resp.interact_pointer_pos() {
                            acts.push(Action::SetF(*sid, ((pt.x - tr.min.x) / tr.width()).clamp(0.0, 1.0)));
                        }
                    }
                }
            }
            Ctl::Step(label, sid) => {
                let c = pos2(right - 95.0, cy);
                for (k, dx, d) in [(0usize, -95.0_f32, -1), (1, 95.0, 1)] {
                    let r = Rect::from_center_size(pos2(c.x + dx * 0.83, cy), vec2(28.0, 28.0));
                    let (clicked, hov) = hit(u, clip, r, &format!("{id}_{k}"));
                    p.circle_filled(r.center(), 14.0, if hov { pal.hover } else { pal.chip });
                    text(p, r.center(), Align2::CENTER_CENTER, if d < 0 { "-" } else { "+" }, 15.0, true, pal.fg);
                    if clicked {
                        acts.push(Action::Set(*sid, d));
                    }
                }
                text(p, c, Align2::CENTER_CENTER, label, 12.0, true, pal.fg);
            }
            Ctl::Value(v) => {
                text(p, pos2(right, cy), Align2::RIGHT_CENTER, v, 12.0, false, pal.sub);
            }
            Ctl::Input => {
                let r = Rect::from_min_size(pos2(right - 330.0, cy - 16.0), vec2(236.0, 32.0));
                p.rect_filled(r, 10.0, pal.chip);
                edits.push((1, r));
                let w = 82.0;
                let b = Rect::from_center_size(pos2(right - w / 2.0, cy), vec2(w, 32.0));
                self.small_btn(u, p, pal, clip, b, "Tambah", true, id, &Action::IgnoreAdd, acts);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn small_btn(
        &self,
        u: &egui::Ui,
        p: &Painter,
        pal: &Pal,
        clip: Rect,
        r: Rect,
        label: &str,
        primary: bool,
        id: &str,
        act: &Action,
        acts: &mut Vec<Action>,
    ) {
        let (clicked, hov) = hit(u, clip, r, id);
        let fill = if primary {
            pal.accent
        } else if hov {
            pal.hover
        } else {
            pal.chip
        };
        p.rect_filled(r, r.height() / 2.0, fill);
        text(p, r.center(), Align2::CENTER_CENTER, label, 11.5, true, if primary { Color32::WHITE } else { pal.fg });
        if clicked {
            acts.push(act.clone());
        }
    }
}
