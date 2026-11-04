// Seluruh gambar UI: pill (collapsed / musik / notifikasi) dan panel expanded dengan 7 tab.

use super::*;
use crate::draw::{self, text, text_ellipsis, Mood, Theme, G};
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontFamily, FontId, Rect, Sense, Stroke};

const PAD: f32 = 18.0;
const ROWS_VISIBLE: usize = 9;

#[derive(Clone, Copy, PartialEq)]
pub enum Edit {
    Search,
    Agenda,
    City,
}

enum Ctl {
    Toggle(bool),
    Stepper(String),
    Button(String),
}

struct Row {
    id: SetId,
    label: &'static str,
    kw: &'static str,
    ctl: Ctl,
}

fn fmt_time(ms: i64) -> String {
    let s = ms.max(0) / 1000;
    format!("{}:{:02}", s / 60, s % 60)
}

impl App {
    pub fn theme(&self) -> Theme {
        if self.cfg.dark { Theme::dark() } else { Theme::light() }
    }

    fn cover_tex(&self) -> Option<&egui::TextureHandle> {
        match &self.cover {
            Some((k, t)) if *k == self.media.track_key => Some(t),
            _ => None,
        }
    }

    /// Indeks baris lirik yang sedang dinyanyikan.
    fn lyric_index(&self, now: Instant) -> Option<usize> {
        if self.lyrics.is_empty() {
            return None;
        }
        let pos = self.position_ms(now) + 200;
        let i = self.lyrics.partition_point(|l| l.0 <= pos);
        if i == 0 { None } else { Some(i - 1) }
    }

    fn next_prayer(&self) -> Option<(usize, u32)> {
        let times = self.prayer_times.as_ref()?;
        let now_m = sys::local_dt().minutes();
        let mut best: Option<(usize, u32)> = None;
        for idx in [0usize, 2, 3, 4, 5] {
            let m = prayer::parse_hm(&times[idx])?;
            let until = if m > now_m { m - now_m } else { m + 1440 - now_m };
            if best.map_or(true, |b| until < b.1) {
                best = Some((idx, until));
            }
        }
        best
    }

    // ------------------------------------------------------------------
    // titik masuk gambar
    // ------------------------------------------------------------------

    pub fn paint(&mut self, ui: &mut egui::Ui, now: Instant) -> Vec<Action> {
        let mut acts: Vec<Action> = Vec::new();
        let th = self.theme();
        let [w, h, top] = self.tween.value(now);
        let rect = Rect::from_min_size(pos2((WIN_W - w) / 2.0, top), vec2(w.max(1.0), h.max(1.0)));
        let radius = (h / 2.0).min(30.0);
        let painter = ui.painter().clone();
        draw::glass(&painter, rect, radius, &th, ((h - 14.0) / 24.0).clamp(0.0, 1.0));

        let mut edit: Option<(Edit, Rect)> = None;
        {
            let g = G { ui: &*ui, p: painter.with_clip_rect(rect), th };
            let pill = g.ui.interact(rect, g.id("pill"), Sense::click());
            let pill_click = pill.clicked();
            if h >= 150.0 {
                self.paint_expanded(&g, rect, now, &mut acts, &mut edit);
            } else {
                self.paint_pill(&g, rect, now, &mut acts);
            }
            if pill_click && acts.is_empty() {
                acts.push(Action::PillClick);
            }
        }
        if let Some((e, r)) = edit {
            self.do_edit(ui, e, r, &th);
        }
        acts
    }

    /// Kolom teks sungguhan (egui TextEdit) diletakkan setelah semua yang dilukis.
    fn do_edit(&mut self, ui: &mut egui::Ui, e: Edit, r: Rect, th: &Theme) {
        let font = FontId::new(12.0, FontFamily::Proportional);
        match e {
            Edit::Search => {
                let resp = ui.put(
                    r,
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text("Cari pengaturan...  (Ctrl+F)")
                        .frame(false)
                        .text_color(th.fg)
                        .font(font)
                        .margin(vec2(12.0, 7.0)),
                );
                if self.focus_search {
                    resp.request_focus();
                    self.focus_search = false;
                }
                if resp.changed() {
                    self.set_scroll = 0;
                }
            }
            Edit::Agenda => {
                let resp = ui.put(
                    r,
                    egui::TextEdit::singleline(&mut self.ag_title)
                        .hint_text("Tulis agenda...")
                        .frame(false)
                        .text_color(th.fg)
                        .font(font)
                        .margin(vec2(12.0, 8.0)),
                );
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.add_agenda();
                    resp.request_focus();
                }
            }
            Edit::City => {
                let resp = ui.put(
                    r,
                    egui::TextEdit::singleline(&mut self.city_filter)
                        .hint_text("Cari kota...")
                        .frame(false)
                        .text_color(th.fg)
                        .font(font)
                        .margin(vec2(12.0, 7.0)),
                );
                if resp.changed() {
                    self.city_scroll = 0;
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // pill
    // ------------------------------------------------------------------

    fn paint_pill(&self, g: &G, rect: Rect, now: Instant, acts: &mut Vec<Action>) {
        let th = g.th;
        let p = &g.p;
        let cy = rect.center().y;
        let t = self.t0.elapsed().as_secs_f32();

        match self.rest_mode() {
            Mode::Alert => {
                if let Some(a) = &self.alert {
                    let mood = if a.kind == AlertKind::Water { Mood::Thirsty } else { Mood::Alert };
                    draw::robot(p, pos2(rect.min.x + 30.0, cy + 2.0), 14.0, mood, t, th.accent, th.fg);
                    let tx = rect.min.x + 62.0;
                    let maxw = rect.width() - 62.0 - 46.0;
                    text_ellipsis(p, pos2(tx, cy - 18.0), &a.text, 13.0, true, th.fg, maxw);
                    text_ellipsis(p, pos2(tx, cy + 3.0), &a.sub, 10.0, false, th.muted, maxw);
                    let cr = Rect::from_center_size(pos2(rect.max.x - 24.0, cy), vec2(28.0, 28.0));
                    let resp = g.click("alert_close", cr);
                    if resp.hovered() {
                        p.circle_filled(cr.center(), 13.0, th.hover);
                    }
                    draw::icon_close(p, cr.center(), 9.0, th.muted);
                    if resp.clicked() {
                        acts.push(Action::AlertClose);
                    }
                }
            }
            Mode::Music => {
                let cr = Rect::from_center_size(pos2(rect.min.x + 24.0, cy), vec2(24.0, 24.0));
                match self.cover_tex() {
                    Some(tex) => {
                        p.image(tex.id(), cr, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                    }
                    None => {
                        p.rect_filled(cr, 6.0, th.track);
                    }
                }
                let line = match self.lyric_index(now) {
                    Some(i) => self.lyrics[i].1.as_str(),
                    None => self.media.title.as_str(),
                };
                text_ellipsis(p, pos2(rect.min.x + 44.0, cy - 9.0), line, 13.0, true, th.fg, rect.width() - 44.0 - 52.0);
                draw::eq_bars(p, pos2(rect.max.x - 30.0, cy), t, self.media.playing, th.accent);
            }
            _ => {
                let mood = if self.media.playing { Mood::Happy } else { Mood::Normal };
                draw::robot(p, pos2(rect.min.x + 26.0, cy + 1.0), 11.0, mood, t, th.accent, th.fg);
                let dt = sys::local_dt();
                text(p, pos2(rect.min.x + 52.0, cy), Align2::LEFT_CENTER, &format!("{:02}:{:02}", dt.h, dt.mi), 14.0, true, th.fg);
                if self.meeting && self.cfg.meeting_auto && self.cfg.mic_button {
                    let col = if self.mic_muted { th.red } else { th.green };
                    draw::icon_mic(p, pos2(rect.min.x + 112.0, cy), 12.0, col, self.mic_muted);
                }
                let r = rect.max.x - 14.0;
                let b = self.battery;
                let col = if b.charging {
                    th.green
                } else if b.percent <= 20 && b.has_battery {
                    th.red
                } else {
                    th.fg
                };
                let level = if b.has_battery { b.percent as f32 / 100.0 } else { 1.0 };
                draw::icon_battery(p, pos2(r - 25.0, cy - 5.5), level, th.muted, col);
                let label = if b.has_battery { format!("{}%", b.percent) } else { "AC".to_string() };
                text(p, pos2(r - 31.0, cy), Align2::RIGHT_CENTER, &label, 13.0, true, th.fg);
            }
        }
    }

    // ------------------------------------------------------------------
    // panel expanded
    // ------------------------------------------------------------------

    fn paint_expanded(
        &self,
        g: &G,
        rect: Rect,
        now: Instant,
        acts: &mut Vec<Action>,
        edit: &mut Option<(Edit, Rect)>,
    ) {
        let th = g.th;
        let p = &g.p;
        let left = rect.min.x + PAD;
        let right = rect.max.x - PAD;
        let top = rect.min.y + 18.0;

        // bar navigasi kapsul di bawah island; tab aktif melebar dan menampilkan label
        let bar = Rect::from_min_size(pos2(left, rect.max.y - 14.0 - 40.0), vec2(right - left, 40.0));
        p.rect_filled(bar, 20.0, th.track);
        p.rect_stroke(bar, 20.0, Stroke::new(1.0_f32, th.line));
        let tabs = [
            (Tab::Home, "Musik", 0u8),
            (Tab::Agenda, "Hari ini", 1),
            (Tab::Prayer, "Sholat", 2),
            (Tab::Shelf, "Shelf", 3),
            (Tab::Focus, "Timer", 4),
            (Tab::Eye, "Mata", 5),
            (Tab::Water, "Minum", 6),
            (Tab::Capture, "Tangkap", 7),
        ];
        let mut x = bar.min.x + 8.0;
        for (i, (tab, label, ic)) in tabs.iter().enumerate() {
            let active = self.tab == *tab;
            let w = if active { 96.0 } else { 36.0 };
            let r = Rect::from_min_size(pos2(x, bar.min.y + 4.0), vec2(w, bar.height() - 8.0));
            x += w + 4.0;
            let resp = g.click(&format!("nav{i}"), r);
            if active {
                p.rect_filled(r, r.height() / 2.0, th.accent);
                draw::nav_icon(p, *ic, pos2(r.min.x + 19.0, r.center().y), 14.0, Color32::WHITE);
                text(p, pos2(r.min.x + 36.0, r.center().y), Align2::LEFT_CENTER, label, 11.0, true, Color32::WHITE);
            } else {
                if resp.hovered() {
                    p.rect_filled(r, r.height() / 2.0, th.hover);
                }
                draw::nav_icon(p, *ic, r.center(), 14.0, th.muted);
            }
            if resp.clicked() {
                acts.push(Action::Tab(*tab));
            }
        }
        // pin
        let pc = pos2(bar.max.x - 62.0, bar.center().y);
        let pr = g.click("pin", Rect::from_center_size(pc, vec2(30.0, 30.0)));
        if self.pinned {
            p.circle_filled(pc, 14.0, th.accent);
        } else if pr.hovered() {
            p.circle_filled(pc, 14.0, th.hover);
        }
        let pcol = if self.pinned { Color32::WHITE } else { th.muted };
        p.circle_filled(pc + vec2(0.0, -2.5), 3.5, pcol);
        p.line_segment([pc + vec2(0.0, 1.0), pc + vec2(0.0, 7.0)], Stroke::new(1.6_f32, pcol));
        if pr.clicked() {
            acts.push(Action::Pin);
        }
        // pengaturan
        let gc = pos2(bar.max.x - 22.0, bar.center().y);
        let gr = g.click("gear", Rect::from_center_size(gc, vec2(30.0, 30.0)));
        let gactive = self.tab == Tab::Settings;
        if gactive {
            p.circle_filled(gc, 15.0, th.accent);
        } else if gr.hovered() {
            p.circle_filled(gc, 15.0, th.hover);
        }
        draw::nav_icon(p, 8, gc, 14.0, if gactive { Color32::WHITE } else { th.muted });
        if gr.clicked() {
            acts.push(Action::Tab(Tab::Settings));
        }

        match self.tab {
            Tab::Home => self.tab_home(g, left, right, top, now, acts),
            Tab::Focus => self.tab_focus(g, left, right, top, acts),
            Tab::Prayer => self.tab_prayer(g, left, right, top, acts, edit),
            Tab::Water => self.tab_water(g, left, right, top, acts),
            Tab::Shelf => self.tab_shelf(g, left, right, top, acts),
            Tab::Agenda => self.tab_agenda(g, left, right, top, acts, edit),
            Tab::Eye => self.tab_eye(g, left, right, top, now, acts),
            Tab::Capture => self.tab_capture(g, left, right, top, acts),
            Tab::Settings => self.tab_settings(g, left, right, top, acts, edit),
        }
    }


    // ------------------------------------------------------------------
    // tab Beranda: musik, lirik, kecerahan/volume, pintasan
    // ------------------------------------------------------------------

    fn tab_home(&self, g: &G, left: f32, right: f32, top: f32, now: Instant, acts: &mut Vec<Action>) {
        let th = g.th;
        let p = &g.p;
        let m = &self.media;

        let cr = Rect::from_min_size(pos2(left, top), vec2(56.0, 56.0));
        match self.cover_tex() {
            Some(tex) => {
                p.image(tex.id(), cr, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
            None => {
                g.card(cr, 14.0);
                text(p, cr.center(), Align2::CENTER_CENTER, "♪", 22.0, true, th.muted);
            }
        }
        p.rect_stroke(cr, 4.0, Stroke::new(1.0_f32, th.line));

        let x0 = left + 68.0;
        let ctl_left = right - 205.0;
        let (title, artist) = if m.available && !m.title.is_empty() {
            (m.title.clone(), if m.artist.is_empty() { m.app.clone() } else { m.artist.clone() })
        } else {
            ("Tidak ada media".to_string(), "Putar lagu di pemutar apa pun".to_string())
        };
        text_ellipsis(p, pos2(x0, top + 7.0), &title, 16.0, true, th.fg, ctl_left - x0 - 6.0);
        text_ellipsis(p, pos2(x0, top + 32.0), &artist, 11.5, false, th.muted, ctl_left - x0 - 6.0);

        // kontrol
        let cy = top + 28.0;
        let r = Rect::from_center_size(pos2(right - 178.0, cy), vec2(34.0, 34.0));
        let resp = g.click("m_prev", r);
        if resp.hovered() {
            p.circle_filled(r.center(), 17.0, th.hover);
        }
        draw::icon_prev(p, r.center(), 13.0, th.fg);
        if resp.clicked() {
            acts.push(Action::Prev);
        }
        let r = Rect::from_center_size(pos2(right - 126.0, cy), vec2(46.0, 46.0));
        let resp = g.click("m_play", r);
        p.circle_filled(r.center(), 23.0, th.fg);
        let ic = if th.dark { Color32::from_rgb(20, 20, 28) } else { Color32::WHITE };
        if m.playing {
            draw::icon_pause(p, r.center(), 15.0, ic);
        } else {
            draw::icon_play(p, r.center(), 15.0, ic);
        }
        if resp.clicked() {
            acts.push(Action::Toggle);
        }
        let r = Rect::from_center_size(pos2(right - 76.0, cy), vec2(34.0, 34.0));
        let resp = g.click("m_next", r);
        if resp.hovered() {
            p.circle_filled(r.center(), 17.0, th.hover);
        }
        draw::icon_next(p, r.center(), 13.0, th.fg);
        if resp.clicked() {
            acts.push(Action::Next);
        }
        // chip jam
        let dt = sys::local_dt();
        let chip = Rect::from_min_size(pos2(right - 54.0, cy - 13.0), vec2(54.0, 26.0));
        p.rect_filled(chip, 13.0, th.track);
        text(p, chip.center(), Align2::CENTER_CENTER, &format!("{:02}.{:02}", dt.h, dt.mi), 12.5, true, th.fg);

        // lirik besar
        let lw = right - left;
        if !self.cfg.lyrics {
            text(p, pos2(left, top + 120.0), Align2::LEFT_CENTER, "Lirik dimatikan di Pengaturan", 13.0, false, th.muted);
        } else if let Some(i) = self.lyric_index(now) {
            text_ellipsis(p, pos2(left, top + 98.0), &self.lyrics[i].1, 22.0, true, th.fg, lw);
            if let Some(n) = self.lyrics.get(i + 1) {
                text_ellipsis(p, pos2(left, top + 138.0), &n.1, 22.0, true, th.muted, lw);
            }
        } else if self.lyrics.is_empty() {
            let msg = if m.available { "Lirik tidak ditemukan" } else { "Belum ada lagu yang diputar" };
            text(p, pos2(left, top + 120.0), Align2::LEFT_CENTER, msg, 13.0, false, th.muted);
        } else {
            text(p, pos2(left, top + 120.0), Align2::LEFT_CENTER, "♪", 22.0, true, th.muted);
        }

        // progres + volume
        let pos = self.position_ms(now);
        let dur = m.duration_ms;
        let yy = top + 228.0;
        text(p, pos2(left, yy), Align2::LEFT_CENTER, &fmt_time(pos), 11.0, true, th.muted);
        let br = Rect::from_min_size(pos2(left + 44.0, yy - 3.0), vec2(right - left - 44.0 - 44.0 - 150.0, 6.0));
        let resp = g.click("m_seek", br.expand2(vec2(0.0, 10.0)));
        let frac = if dur > 0 { pos as f32 / dur as f32 } else { 0.0 };
        g.bar(br, frac, th.accent);
        if resp.clicked() && dur > 0 {
            if let Some(pt) = resp.interact_pointer_pos() {
                let f = ((pt.x - br.min.x) / br.width()).clamp(0.0, 1.0);
                acts.push(Action::Seek((f * dur as f32) as i64));
            }
        }
        text(p, pos2(br.max.x + 10.0, yy), Align2::LEFT_CENTER, &fmt_time(dur), 11.0, true, th.muted);
        let vr = Rect::from_min_size(pos2(right - 110.0, yy - 3.0), vec2(110.0, 6.0));
        let vresp = g.ui.interact(vr.expand2(vec2(0.0, 10.0)), g.id("m_vol"), Sense::click_and_drag());
        g.bar(vr, self.volume_percent as f32 / 100.0, th.fg);
        draw::icon_play(p, pos2(vr.min.x - 14.0, yy), 7.0, th.muted);
        if vresp.clicked() || vresp.dragged() {
            if let Some(pt) = vresp.interact_pointer_pos() {
                let v = ((pt.x - vr.min.x) / vr.width()).clamp(0.0, 1.0);
                acts.push(Action::VolumeValue((v * 100.0).round() as u8));
            }
        }

        // tombol mic hanya saat rapat
        if self.meeting && self.cfg.meeting_auto && self.cfg.mic_button {
            let (lab, fill) = if self.mic_muted { ("Mic mati - rapat", th.red.gamma_multiply(0.6)) } else { ("Mic aktif - rapat", th.track) };
            let r = Rect::from_min_size(pos2(left, top + 262.0), vec2(150.0, 30.0));
            if g.tint_button("mic", r, lab, fill, th.fg) {
                acts.push(Action::MicToggle);
            }
        }
    }


    // ------------------------------------------------------------------
    // tab Fokus: Pomodoro / Timer + istirahat mata
    // ------------------------------------------------------------------

    fn tab_focus(&self, g: &G, left: f32, right: f32, top: f32, acts: &mut Vec<Action>) {
        let th = g.th;
        let p = &g.p;
        let po = &self.pomo;

        let c = pos2(left + 100.0, top + 130.0);
        let total = self.pomo_phase_ms().max(1);
        let frac = po.remaining_ms.max(0) as f32 / total as f32;
        let col = match (po.mode, po.phase) {
            (FocusMode::Pomodoro, Phase::Short) | (FocusMode::Pomodoro, Phase::Long) => th.green,
            _ => th.accent,
        };
        g.ring(c, 88.0, frac, col, 8.0);
        let secs = (po.remaining_ms.max(0) + 999) / 1000;
        text(p, c + vec2(0.0, -8.0), Align2::CENTER_CENTER, &format!("{:02}:{:02}", secs / 60, secs % 60), 34.0, true, th.fg);
        let label = match (po.mode, po.phase) {
            (FocusMode::Timer, _) => "Timer",
            (FocusMode::Pomodoro, Phase::Work) => "Fokus",
            (FocusMode::Pomodoro, Phase::Short) => "Istirahat pendek",
            (FocusMode::Pomodoro, Phase::Long) => "Istirahat panjang",
        };
        text(p, c + vec2(0.0, 26.0), Align2::CENTER_CENTER, label, 11.0, false, th.muted);

        let rx = left + 224.0;
        let rw = right - rx;
        let half = (rw - 8.0) / 2.0;
        let pomo_on = po.mode == FocusMode::Pomodoro;
        let r1 = Rect::from_min_size(pos2(rx, top + 20.0), vec2(half, 34.0));
        let r2 = Rect::from_min_size(pos2(rx + half + 8.0, top + 20.0), vec2(half, 34.0));
        if g.tint_button("fm_pomo", r1, "Pomodoro", if pomo_on { th.accent } else { th.track }, if pomo_on { Color32::WHITE } else { th.fg }) {
            acts.push(Action::PomoMode(FocusMode::Pomodoro));
        }
        if g.tint_button("fm_timer", r2, "Timer", if !pomo_on { th.accent } else { th.track }, if !pomo_on { Color32::WHITE } else { th.fg }) {
            acts.push(Action::PomoMode(FocusMode::Timer));
        }
        let bw = (rw - 16.0) / 3.0;
        let by = top + 70.0;
        if g.button("fm_go", Rect::from_min_size(pos2(rx, by), vec2(bw, 38.0)), if po.running { "Jeda" } else { "Mulai" }, true) {
            acts.push(Action::PomoToggle);
        }
        if g.button("fm_reset", Rect::from_min_size(pos2(rx + bw + 8.0, by), vec2(bw, 38.0)), "Reset", false) {
            acts.push(Action::PomoReset);
        }
        if pomo_on && g.button("fm_skip", Rect::from_min_size(pos2(rx + 2.0 * (bw + 8.0), by), vec2(bw, 38.0)), "Lewati", false) {
            acts.push(Action::PomoSkip);
        }
        if pomo_on {
            text(p, pos2(rx, top + 134.0), Align2::LEFT_CENTER, &format!("Sesi fokus selesai: {}", po.cycles), 12.0, false, th.muted);
            text(
                p,
                pos2(rx, top + 156.0),
                Align2::LEFT_CENTER,
                &format!("{} / {} / {} mnt (fokus / pendek / panjang)", self.cfg.pomo_work, self.cfg.pomo_short, self.cfg.pomo_long),
                10.0,
                false,
                th.muted,
            );
        } else {
            let hw = (rw - 8.0) / 2.0;
            for (k, (label, d)) in [("- 1 mnt", -1), ("+ 1 mnt", 1), ("- 10 mnt", -10), ("+ 10 mnt", 10)].iter().enumerate() {
                let r = Rect::from_min_size(pos2(rx + (k % 2) as f32 * (hw + 8.0), top + 124.0 + (k / 2) as f32 * 36.0), vec2(hw, 30.0));
                if g.button(&format!("tm_{k}"), r, label, false) {
                    acts.push(Action::TimerAdj(*d));
                }
            }
        }
    }


    // ------------------------------------------------------------------
    // tab Sholat: jadwal + cuaca (Indonesia / 56 kota Malaysia)
    // ------------------------------------------------------------------

    fn tab_prayer(
        &self,
        g: &G,
        left: f32,
        right: f32,
        top: f32,
        acts: &mut Vec<Action>,
        edit: &mut Option<(Edit, Rect)>,
    ) {
        let th = g.th;
        let p = &g.p;
        let region = self.cfg.region.as_str();
        let w = right - left;

        if self.city_picking {
            text(p, pos2(left, top + 12.0), Align2::LEFT_CENTER, &format!("Pilih kota - {}", prayer::region_label(region)), 13.0, true, th.fg);
            if g.button("city_cancel", Rect::from_min_size(pos2(right - 70.0, top), vec2(70.0, 26.0)), "Batal", false) {
                acts.push(Action::CityPick(false));
            }
            let fr = Rect::from_min_size(pos2(left, top + 34.0), vec2(w, 32.0));
            g.card(fr, 16.0);
            *edit = Some((Edit::City, fr));

            let q = self.city_filter.to_lowercase();
            let all = prayer::cities(region);
            let filtered: Vec<&&str> = all.iter().filter(|c| q.is_empty() || c.to_lowercase().contains(&q)).collect();
            let visible = 6usize;
            let off = self.city_scroll.min(filtered.len().saturating_sub(visible));
            for (row, name) in filtered.iter().skip(off).take(visible).enumerate() {
                let r = Rect::from_min_size(pos2(left, top + 74.0 + row as f32 * 31.0), vec2(w, 28.0));
                let resp = g.click(&format!("city_{row}"), r);
                let current = **name == self.cfg.city.as_str();
                if current {
                    p.rect_filled(r, 14.0, th.accent.gamma_multiply(0.45));
                } else if resp.hovered() {
                    p.rect_filled(r, 14.0, th.hover);
                }
                text(p, pos2(r.min.x + 14.0, r.center().y), Align2::LEFT_CENTER, name, 12.0, current, th.fg);
                if resp.clicked() {
                    acts.push(Action::CityChoose(name.to_string()));
                }
            }
            text(
                p,
                pos2(left, top + 74.0 + 6.0 * 31.0 + 8.0),
                Align2::LEFT_CENTER,
                &format!("{} kota - gulir mouse untuk melihat lainnya", filtered.len()),
                9.0,
                false,
                th.muted,
            );
            return;
        }

        // header: kota + cuaca
        text(p, pos2(left, top + 10.0), Align2::LEFT_CENTER, &self.cfg.city, 15.0, true, th.fg);
        let wtxt = match &self.weather {
            Some((t, d)) => format!("{:.0}°C  {}", t, d),
            None => "Memuat cuaca...".to_string(),
        };
        text(p, pos2(left, top + 30.0), Align2::LEFT_CENTER, &wtxt, 11.0, false, th.muted);
        if g.button("city_change", Rect::from_min_size(pos2(right - 70.0, top), vec2(70.0, 26.0)), "Ubah", false) {
            acts.push(Action::CityPick(true));
        }

        // panel lengkung matahari
        let names = prayer::names(region);
        let next = self.next_prayer();
        let ar = Rect::from_min_size(pos2(left, top + 48.0), vec2(w, 164.0));
        g.card(ar, 22.0);
        match &self.prayer_times {
            Some(times) => {
                if let Some((idx, mins)) = next {
                    text(p, pos2(ar.min.x + 18.0, ar.min.y + 24.0), Align2::LEFT_CENTER, &format!("{}  {}", names[idx], times[idx]), 19.0, true, th.fg);
                    text(p, pos2(ar.max.x - 18.0, ar.min.y + 24.0), Align2::RIGHT_CENTER, &format!("{} j {:02} m lagi", mins / 60, mins % 60), 12.0, true, th.accent);
                }
                let x0 = ar.min.x + 20.0;
                let x1 = ar.max.x - 20.0;
                let base = ar.max.y - 22.0;
                let peak = 72.0_f32;
                let at = |f: f32| pos2(x0 + (x1 - x0) * f, base - 4.0 * peak * f * (1.0 - f));
                p.line_segment([pos2(x0 - 6.0, base), pos2(x1 + 6.0, base)], Stroke::new(1.0_f32, th.line));
                let n = 48;
                let mut prev = at(0.0);
                for i in 1..=n {
                    let q = at(i as f32 / n as f32);
                    p.line_segment([prev, q], Stroke::new(1.6_f32, th.muted));
                    prev = q;
                }
                let sub = prayer::parse_hm(&times[0]);
                let isy = prayer::parse_hm(&times[5]);
                if let (Some(s), Some(e)) = (sub, isy) {
                    if e > s {
                        let span = (e - s) as f32;
                        for idx in [0usize, 2, 3, 4, 5] {
                            if let Some(m) = prayer::parse_hm(&times[idx]) {
                                p.circle_filled(at(((m as f32 - s as f32) / span).clamp(0.0, 1.0)), 3.0, th.fg);
                            }
                        }
                        let nowm = sys::local_dt().minutes() as f32;
                        let f = ((nowm - s as f32) / span).clamp(0.0, 1.0);
                        p.circle_filled(at(f), 9.0, th.accent.gamma_multiply(0.35));
                        p.circle_filled(at(f), 6.0, th.accent);
                        p.circle_stroke(at(f), 6.0, Stroke::new(1.5_f32, Color32::WHITE));
                    }
                }
            }
            None => {
                text(p, ar.center(), Align2::CENTER_CENTER, "Memuat jadwal sholat... (perlu internet)", 12.0, false, th.muted);
            }
        }

        // lima kartu waktu
        if let Some(times) = &self.prayer_times {
            let cw = (w - 4.0 * 8.0) / 5.0;
            for (k, idx) in [0usize, 2, 3, 4, 5].iter().enumerate() {
                let r = Rect::from_min_size(pos2(left + k as f32 * (cw + 8.0), top + 224.0), vec2(cw, 56.0));
                let is_next = next.map(|n| n.0) == Some(*idx);
                if is_next {
                    p.rect_filled(r, 16.0, th.accent.gamma_multiply(0.30));
                    p.rect_stroke(r, 16.0, Stroke::new(1.5_f32, th.accent));
                } else {
                    p.rect_filled(r, 16.0, th.track);
                }
                text(p, pos2(r.center().x, r.min.y + 17.0), Align2::CENTER_CENTER, names[*idx], 10.5, false, th.muted);
                text(p, pos2(r.center().x, r.min.y + 38.0), Align2::CENTER_CENTER, &times[*idx].replace(':', "."), 15.0, true, if is_next { th.accent } else { th.fg });
            }
        }
    }


    fn tab_eye(&self, g: &G, left: f32, right: f32, top: f32, now: Instant, acts: &mut Vec<Action>) {
        let th = g.th;
        let p = &g.p;
        let w = right - left;
        text(p, pos2(left, top + 14.0), Align2::LEFT_CENTER, "Pengingat istirahat mata", 17.0, true, th.fg);
        let (st, sc) = if self.cfg.eye_on { ("Aktif", th.green) } else { ("Mati", th.muted) };
        text(p, pos2(right, top + 14.0), Align2::RIGHT_CENTER, st, 12.0, true, sc);

        let opts: [u32; 5] = [0, 20, 30, 45, 60];
        let bw = (w - 4.0 * 8.0) / 5.0;
        for (i, m) in opts.iter().enumerate() {
            let active = if *m == 0 { !self.cfg.eye_on } else { self.cfg.eye_on && self.cfg.eye_min == *m };
            let r = Rect::from_min_size(pos2(left + i as f32 * (bw + 8.0), top + 46.0), vec2(bw, 42.0));
            let label = if *m == 0 { "Mati".to_string() } else { format!("{} menit", m) };
            let fill = if active { th.green } else { th.track };
            let fg = if active { Color32::WHITE } else { th.fg };
            if g.tint_button(&format!("eye{i}"), r, &label, fill, fg) {
                acts.push(Action::EyeSet(*m));
            }
        }
        if self.cfg.eye_on {
            let rem = self.next_eye_at.saturating_duration_since(now).as_secs() / 60 + 1;
            text(p, pos2(left, top + 116.0), Align2::LEFT_CENTER, &format!("Pengingat berikutnya dalam {} menit", rem), 12.0, true, th.fg);
        } else {
            text(p, pos2(left, top + 116.0), Align2::LEFT_CENTER, "Pilih selang untuk mengaktifkan pengingat", 12.0, false, th.muted);
        }
        if g.button("eye_test", Rect::from_min_size(pos2(right - 150.0, top + 100.0), vec2(150.0, 32.0)), "Coba sekarang", false) {
            acts.push(Action::EyeTest);
        }
        text(p, pos2(left, top + 170.0), Align2::LEFT_CENTER, "Aturan 20-20-20: tiap 20 menit, lihat objek sejauh 6 meter", 11.0, false, th.muted);
        text(p, pos2(left, top + 190.0), Align2::LEFT_CENTER, "selama 20 detik supaya mata tidak tegang.", 11.0, false, th.muted);
    }

    fn tab_capture(&self, g: &G, left: f32, right: f32, top: f32, acts: &mut Vec<Action>) {
        let th = g.th;
        let p = &g.p;
        let w = right - left;
        text(p, pos2(left, top + 14.0), Align2::LEFT_CENTER, "Tangkap layar", 17.0, true, th.fg);
        let chip = Rect::from_min_size(pos2(right - 130.0, top + 2.0), vec2(130.0, 26.0));
        p.rect_filled(chip, 13.0, th.track);
        text(p, chip.center(), Align2::CENTER_CENTER, &self.cfg.hotkey.replace('+', " + "), 11.0, true, th.fg);

        let tw = (w - 3.0 * 12.0) / 4.0;
        let names = ["Layar penuh", "Area", "Jendela", "Rekam layar"];
        for i in 0..4usize {
            let r = Rect::from_min_size(pos2(left + i as f32 * (tw + 12.0), top + 48.0), vec2(tw, 120.0));
            let resp = g.click(&format!("cap{i}"), r);
            let rec = i == 3;
            let fill = if rec { th.red.gamma_multiply(0.55) } else if resp.hovered() { th.hover } else { th.track };
            p.rect_filled(r, 20.0, fill);
            let c = pos2(r.center().x, r.min.y + 46.0);
            let st = Stroke::new(2.0_f32, th.fg);
            match i {
                0 => {
                    p.rect_stroke(Rect::from_center_size(c, vec2(34.0, 24.0)), 4.0, st);
                    p.line_segment([c + vec2(-8.0, 18.0), c + vec2(8.0, 18.0)], st);
                }
                1 => {
                    let h = 15.0;
                    for (sx, sy) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                        let corner = c + vec2(sx * h, sy * h);
                        p.line_segment([corner, corner + vec2(-sx * 8.0, 0.0)], st);
                        p.line_segment([corner, corner + vec2(0.0, -sy * 8.0)], st);
                    }
                }
                2 => {
                    let wr = Rect::from_center_size(c, vec2(34.0, 26.0));
                    p.rect_stroke(wr, 5.0, st);
                    p.line_segment([wr.min + vec2(0.0, 7.0), wr.max - vec2(0.0, 19.0)], st);
                }
                _ => {
                    p.circle_stroke(c, 14.0, Stroke::new(2.0_f32, Color32::from_rgb(255, 140, 120)));
                    p.circle_filled(c, 7.0, Color32::from_rgb(255, 120, 100));
                }
            }
            text(p, pos2(r.center().x, r.max.y - 24.0), Align2::CENTER_CENTER, names[i], 11.5, true, th.fg);
            if resp.clicked() {
                acts.push(match i {
                    0 => Action::CapFull,
                    1 => Action::Screenshot,
                    2 => Action::CapWindow,
                    _ => Action::Record,
                });
            }
        }
        text(p, pos2(left, top + 196.0), Align2::LEFT_CENTER, "Hasil disimpan oleh Windows (Gambar\\Screenshots, Video\\Captures).", 10.5, false, th.muted);
        text(p, pos2(left, top + 214.0), Align2::LEFT_CENTER, "Ganti pintasan di Pengaturan (ikon gigi).", 10.5, false, th.muted);
    }

    // ------------------------------------------------------------------
    // tab Minum
    // ------------------------------------------------------------------

    fn tab_water(&self, g: &G, left: f32, right: f32, top: f32, acts: &mut Vec<Action>) {
        let th = g.th;
        let p = &g.p;
        let goal = self.cfg.water_goal.max(1);
        let ml = self.data.water_ml;

        text(p, pos2(left, top + 18.0), Align2::LEFT_CENTER, &format!("{} ml", ml), 30.0, true, th.fg);
        text(p, pos2(left, top + 46.0), Align2::LEFT_CENTER, &format!("dari target {} ml hari ini", goal), 11.0, false, th.muted);
        let col = if ml >= goal { th.green } else { th.accent };
        g.bar(Rect::from_min_size(pos2(left, top + 62.0), vec2(right - left, 12.0)), ml as f32 / goal as f32, col);

        let cup = self.cfg.water_cup as i32;
        if g.button("w_add", Rect::from_min_size(pos2(left, top + 86.0), vec2(150.0, 34.0)), &format!("+ {} ml", cup), true) {
            acts.push(Action::Drink(cup));
        }
        if g.button("w_undo", Rect::from_min_size(pos2(left + 158.0, top + 86.0), vec2(100.0, 34.0)), "Batalkan", false) {
            acts.push(Action::Drink(-cup));
        }

        // pengaturan pengingat
        let cr = Rect::from_min_size(pos2(left, top + 132.0), vec2(right - left, 128.0));
        g.card(cr, 18.0);
        let row_y = |i: f32| cr.min.y + 20.0 + i * 30.0;

        text(p, pos2(cr.min.x + 16.0, row_y(0.0)), Align2::LEFT_CENTER, "Pengingat minum", 12.0, true, th.fg);
        let tr = Rect::from_min_size(pos2(cr.max.x - 16.0 - 38.0, row_y(0.0) - 11.0), vec2(38.0, 22.0));
        if g.toggle("w_on", tr, self.cfg.water_on) {
            acts.push(Action::Set(SetId::WaterOn, 0));
        }

        text(p, pos2(cr.min.x + 16.0, row_y(1.0)), Align2::LEFT_CENTER, "Selang", 12.0, false, th.fg);
        let d = g.stepper("w_int", pos2(cr.max.x - 80.0, row_y(1.0)), &format!("{} mnt", self.cfg.water_min));
        if d != 0 {
            acts.push(Action::Set(SetId::WaterInt, d));
        }

        text(p, pos2(cr.min.x + 16.0, row_y(2.0)), Align2::LEFT_CENTER, "Jam aktif", 12.0, false, th.fg);
        let d = g.stepper("w_start", pos2(cr.max.x - 230.0, row_y(2.0)), &format!("{:02}:00", self.cfg.water_start));
        if d != 0 {
            acts.push(Action::Set(SetId::WaterStart, d));
        }
        let d = g.stepper("w_end", pos2(cr.max.x - 80.0, row_y(2.0)), &format!("{:02}:00", self.cfg.water_end));
        if d != 0 {
            acts.push(Action::Set(SetId::WaterEnd, d));
        }

        text(p, pos2(cr.min.x + 16.0, row_y(3.0)), Align2::LEFT_CENTER, "Target harian", 12.0, false, th.fg);
        let d = g.stepper("w_goal", pos2(cr.max.x - 80.0, row_y(3.0)), &format!("{} ml", goal));
        if d != 0 {
            acts.push(Action::Set(SetId::WaterGoal, d));
        }
    }

    // ------------------------------------------------------------------
    // tab Shelf: file, riwayat clipboard, kirim ke HP
    // ------------------------------------------------------------------

    fn tab_shelf(&self, g: &G, left: f32, right: f32, top: f32, acts: &mut Vec<Action>) {
        let th = g.th;
        let p = &g.p;
        let cw = (right - left - 10.0) / 2.0;

        // kolom file
        let fr = Rect::from_min_size(pos2(left, top), vec2(cw, 190.0));
        g.card(fr, 18.0);
        text(p, pos2(fr.min.x + 14.0, fr.min.y + 16.0), Align2::LEFT_CENTER, "File shelf", 12.0, true, th.fg);
        if self.data.shelf.is_empty() {
            text(p, fr.center(), Align2::CENTER_CENTER, "Seret file ke sini", 11.0, false, th.muted);
        }
        for (i, path) in self.data.shelf.iter().enumerate().take(5) {
            let r = Rect::from_min_size(pos2(fr.min.x + 8.0, fr.min.y + 34.0 + i as f32 * 29.0), vec2(cw - 16.0, 26.0));
            let resp = g.click(&format!("sh_open{i}"), r);
            if resp.hovered() {
                p.rect_filled(r, 13.0, th.hover);
            }
            let name = std::path::Path::new(path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.clone());
            text_ellipsis(p, pos2(r.min.x + 10.0, r.min.y + 5.0), &name, 11.0, false, th.fg, r.width() - 40.0);
            if resp.clicked() {
                acts.push(Action::ShelfOpen(i));
            }
            let xr = Rect::from_center_size(pos2(r.max.x - 14.0, r.center().y), vec2(22.0, 22.0));
            let xresp = g.click(&format!("sh_del{i}"), xr);
            draw::icon_close(p, xr.center(), 8.0, th.muted);
            if xresp.clicked() {
                acts.push(Action::ShelfRemove(i));
            }
        }
        if self.data.shelf.len() > 5 {
            text(p, pos2(fr.min.x + 14.0, fr.max.y - 10.0), Align2::LEFT_CENTER, &format!("+{} lainnya", self.data.shelf.len() - 5), 9.0, false, th.muted);
        }

        // kolom clipboard
        let cr = Rect::from_min_size(pos2(left + cw + 10.0, top), vec2(cw, 190.0));
        g.card(cr, 18.0);
        text(p, pos2(cr.min.x + 14.0, cr.min.y + 16.0), Align2::LEFT_CENTER, "Riwayat clipboard", 12.0, true, th.fg);
        if !self.clips.is_empty() {
            let hr = Rect::from_min_size(pos2(cr.max.x - 58.0, cr.min.y + 6.0), vec2(48.0, 20.0));
            if g.button("clip_clear", hr, "Hapus", false) {
                acts.push(Action::ClipClear);
            }
        } else {
            text(p, cr.center(), Align2::CENTER_CENTER, "Belum ada salinan", 11.0, false, th.muted);
        }
        for (i, clip) in self.clips.iter().enumerate().take(5) {
            let r = Rect::from_min_size(pos2(cr.min.x + 8.0, cr.min.y + 34.0 + i as f32 * 29.0), vec2(cw - 16.0, 26.0));
            let resp = g.click(&format!("clip{i}"), r);
            if resp.hovered() {
                p.rect_filled(r, 13.0, th.hover);
            }
            let one = clip.replace(['\n', '\r', '\t'], " ");
            text_ellipsis(p, pos2(r.min.x + 10.0, r.min.y + 5.0), &one, 11.0, false, th.fg, r.width() - 20.0);
            if resp.clicked() {
                acts.push(Action::ClipCopy(i));
            }
        }

        // kirim ke HP
        let sr = Rect::from_min_size(pos2(left, top + 200.0), vec2(right - left, 58.0));
        g.card(sr, 18.0);
        text(p, pos2(sr.min.x + 16.0, sr.min.y + 18.0), Align2::LEFT_CENTER, "Kirim ke HP (Wi-Fi yang sama)", 12.0, true, th.fg);
        let on = self.share_on.load(Ordering::Relaxed);
        let tr = Rect::from_min_size(pos2(sr.max.x - 16.0 - 38.0, sr.min.y + 7.0), vec2(38.0, 22.0));
        if g.toggle("share_on", tr, on) {
            acts.push(Action::ShareToggle);
        }
        let sub = match (&self.share_url, on) {
            (Some(u), true) => u.clone(),
            _ => "Aktifkan, lalu buka tautan yang muncul di browser HP".to_string(),
        };
        let subcol = if on { th.accent } else { th.muted };
        text_ellipsis(p, pos2(sr.min.x + 16.0, sr.min.y + 36.0), &sub, 11.0, false, subcol, sr.width() - 32.0);
    }

    // ------------------------------------------------------------------
    // tab Agenda
    // ------------------------------------------------------------------

    fn tab_agenda(
        &self,
        g: &G,
        left: f32,
        right: f32,
        top: f32,
        acts: &mut Vec<Action>,
        edit: &mut Option<(Edit, Rect)>,
    ) {
        const MONTHS: [&str; 12] = [
            "Januari", "Februari", "Maret", "April", "Mei", "Juni", "Juli", "Agustus", "September", "Oktober", "November", "Desember",
        ];
        let th = g.th;
        let p = &g.p;
        let dt = sys::local_dt();
        let today = dt.ymd();
        let mut y = dt.y as i32;
        let mut m = dt.mo as i32 + self.cal_off;
        while m > 12 {
            m -= 12;
            y += 1;
        }
        while m < 1 {
            m += 12;
            y -= 1;
        }
        let (y, m) = (y as u32, m as u32);

        let cw = 250.0;
        let cell = cw / 7.0;
        text(p, pos2(left, top + 12.0), Align2::LEFT_CENTER, &format!("{} {}", MONTHS[(m - 1) as usize], y), 14.0, true, th.fg);
        for (k, (id, d, cx)) in [("cal_prev", -1, left + cw - 48.0), ("cal_next", 1, left + cw - 14.0)].iter().enumerate() {
            let r = Rect::from_center_size(pos2(*cx, top + 12.0), vec2(26.0, 26.0));
            let resp = g.click(id, r);
            if resp.hovered() {
                p.circle_filled(r.center(), 13.0, th.hover);
            }
            if k == 0 {
                draw::icon_prev(p, r.center(), 8.0, th.fg);
            } else {
                draw::icon_next(p, r.center(), 8.0, th.fg);
            }
            if resp.clicked() {
                acts.push(Action::CalNav(*d));
            }
        }
        for (i, d) in ["Sen", "Sel", "Rab", "Kam", "Jum", "Sab", "Min"].iter().enumerate() {
            text(p, pos2(left + cell * i as f32 + cell / 2.0, top + 42.0), Align2::CENTER_CENTER, d, 10.0, true, th.muted);
        }
        let first = store::weekday(y, m, 1);
        let dim = store::days_in_month(y, m);
        for day in 1..=dim {
            let idx = first + day - 1;
            let c = pos2(left + (idx % 7) as f32 * cell + cell / 2.0, top + 68.0 + (idx / 7) as f32 * 30.0);
            let ymd = y * 10000 + m * 100 + day;
            let resp = g.click(&format!("cal{day}"), Rect::from_center_size(c, vec2(cell - 2.0, 28.0)));
            let is_today = ymd == today;
            if is_today {
                p.circle_filled(c, 13.0, th.accent);
            } else if ymd == self.cal_sel {
                p.circle_stroke(c, 13.0, Stroke::new(1.6_f32, th.accent));
            } else if resp.hovered() {
                p.circle_filled(c, 13.0, th.hover);
            }
            text(p, c, Align2::CENTER_CENTER, &day.to_string(), 12.0, true, if is_today { Color32::WHITE } else { th.fg });
            if self.data.agenda.iter().any(|a| a.ymd == ymd) {
                p.circle_filled(c + vec2(0.0, 11.0), 2.0, if is_today { Color32::WHITE } else { th.accent });
            }
            if resp.clicked() {
                acts.push(Action::CalSel(ymd));
            }
        }

        // panel kanan
        let px = left + cw + 16.0;
        let pw = right - px;
        p.line_segment([pos2(px - 8.0, top + 4.0), pos2(px - 8.0, top + 270.0)], Stroke::new(1.0_f32, th.line));
        let sel = if self.cal_sel == 0 { today } else { self.cal_sel };
        let title = if sel == today {
            "Hari ini".to_string()
        } else {
            format!("{} {}", sel % 100, MONTHS[(((sel / 100) % 100).max(1) - 1) as usize])
        };
        text(p, pos2(px, top + 12.0), Align2::LEFT_CENTER, &title, 14.0, true, th.fg);
        let items: Vec<&AgendaItem> = self.data.agenda.iter().filter(|a| a.ymd == sel).collect();
        if items.is_empty() {
            text(p, pos2(px, top + 50.0), Align2::LEFT_CENTER, "Tidak ada acara atau tugas", 11.0, false, th.muted);
        }
        for (i, a) in items.iter().enumerate().take(4) {
            let r = Rect::from_min_size(pos2(px, top + 32.0 + i as f32 * 32.0), vec2(pw, 28.0));
            g.card(r, 14.0);
            let cc = pos2(r.min.x + 16.0, r.center().y);
            let cresp = g.click(&format!("ag_chk{i}"), Rect::from_center_size(cc, vec2(22.0, 22.0)));
            p.circle_stroke(cc, 7.0, Stroke::new(1.6_f32, if a.done { th.accent } else { th.muted }));
            if a.done {
                p.circle_filled(cc, 4.0, th.accent);
            }
            if cresp.clicked() {
                acts.push(Action::AgToggle(a.id));
            }
            text(p, pos2(r.min.x + 32.0, r.center().y), Align2::LEFT_CENTER, &format!("{:02}:{:02}", a.hm / 60, a.hm % 60), 10.5, true, th.accent);
            text_ellipsis(p, pos2(r.min.x + 76.0, r.min.y + 6.0), &a.title, 11.5, false, if a.done { th.muted } else { th.fg }, pw - 76.0 - 28.0);
            let xr = Rect::from_center_size(pos2(r.max.x - 14.0, r.center().y), vec2(22.0, 22.0));
            let xresp = g.click(&format!("ag_del{i}"), xr);
            draw::icon_close(p, xr.center(), 7.0, th.muted);
            if xresp.clicked() {
                acts.push(Action::AgDel(a.id));
            }
        }
        let er = Rect::from_min_size(pos2(px, top + 170.0), vec2(pw, 30.0));
        g.card(er, 15.0);
        *edit = Some((Edit::Agenda, er));
        let d = g.stepper("ag_time", pos2(px + 62.0, top + 222.0), &format!("{:02}:{:02}", self.ag_hm / 60, self.ag_hm % 60));
        if d != 0 {
            acts.push(Action::AgTime(d));
        }
        if g.button("ag_add", Rect::from_min_size(pos2(px + 132.0, top + 208.0), vec2(pw - 132.0, 28.0)), "Tambah", true) {
            acts.push(Action::AgAdd);
        }
    }


    // ------------------------------------------------------------------
    // tab Atur: pengaturan dengan kotak cari (Ctrl+F)
    // ------------------------------------------------------------------

    fn settings_rows(&self) -> Vec<Row> {
        let c = &self.cfg;
        let hot = if self.capturing_hotkey { "Tekan tombol...".to_string() } else { c.hotkey.clone() };
        vec![
            Row { id: SetId::Dark, label: "Tema gelap", kw: "tema tampilan terang gelap mode", ctl: Ctl::Toggle(c.dark) },
            Row { id: SetId::Lyrics, label: "Lirik tersinkron", kw: "lirik musik lagu", ctl: Ctl::Toggle(c.lyrics) },
            Row { id: SetId::Notif, label: "Notifikasi ringkas", kw: "notifikasi pemberitahuan alert", ctl: Ctl::Toggle(c.notif) },
            Row { id: SetId::MicBtn, label: "Tombol mic saat rapat", kw: "mic mikrofon bisu rapat meeting", ctl: Ctl::Toggle(c.mic_button) },
            Row { id: SetId::MeetingAuto, label: "Deteksi mode rapat", kw: "rapat meeting zoom teams meet otomatis", ctl: Ctl::Toggle(c.meeting_auto) },
            Row { id: SetId::Hotkey, label: "Pintasan tangkap layar", kw: "tangkap layar screenshot pintasan hotkey shortcut", ctl: Ctl::Button(hot) },
            Row { id: SetId::Adzan, label: "Adzan", kw: "adzan azan sholat salat suara", ctl: Ctl::Toggle(c.adzan) },
            Row { id: SetId::AdzanPause, label: "Jeda musik saat adzan", kw: "adzan musik jeda", ctl: Ctl::Toggle(c.adzan_pause) },
            Row { id: SetId::Region, label: "Wilayah sholat", kw: "sholat wilayah indonesia malaysia jakim kemenag kota", ctl: Ctl::Button(prayer::region_label(&c.region).to_string()) },
            Row { id: SetId::EyeOn, label: "Istirahat mata", kw: "mata istirahat pengingat 20-20-20", ctl: Ctl::Toggle(c.eye_on) },
            Row { id: SetId::EyeInt, label: "Selang istirahat mata", kw: "mata istirahat selang menit", ctl: Ctl::Stepper(format!("{} mnt", c.eye_min)) },
            Row { id: SetId::PomoWork, label: "Pomodoro: fokus", kw: "pomodoro fokus timer durasi", ctl: Ctl::Stepper(format!("{} mnt", c.pomo_work)) },
            Row { id: SetId::PomoShort, label: "Pomodoro: istirahat pendek", kw: "pomodoro istirahat pendek timer", ctl: Ctl::Stepper(format!("{} mnt", c.pomo_short)) },
            Row { id: SetId::PomoLong, label: "Pomodoro: istirahat panjang", kw: "pomodoro istirahat panjang timer", ctl: Ctl::Stepper(format!("{} mnt", c.pomo_long)) },
            Row { id: SetId::Autostart, label: "Mulai bersama Windows", kw: "startup autostart boot mulai windows", ctl: Ctl::Toggle(c.autostart) },
            Row { id: SetId::OpenWin, label: "Pengaturan Windows", kw: "windows settings pengaturan sistem", ctl: Ctl::Button("Buka".to_string()) },
            Row { id: SetId::Quit, label: "Keluar dari RafLand", kw: "keluar tutup quit exit", ctl: Ctl::Button("Keluar".to_string()) },
        ]
    }

    fn tab_settings(
        &self,
        g: &G,
        left: f32,
        right: f32,
        top: f32,
        acts: &mut Vec<Action>,
        edit: &mut Option<(Edit, Rect)>,
    ) {
        let th = g.th;
        let p = &g.p;

        let sr = Rect::from_min_size(pos2(left, top), vec2(right - left, 32.0));
        g.card(sr, 16.0);
        *edit = Some((Edit::Search, sr));

        let q = self.search.to_lowercase();
        let rows: Vec<Row> = self
            .settings_rows()
            .into_iter()
            .filter(|r| q.is_empty() || r.label.to_lowercase().contains(&q) || r.kw.contains(&q))
            .collect();
        if rows.is_empty() {
            text(p, pos2((left + right) / 2.0, top + 120.0), Align2::CENTER_CENTER, "Tidak ada pengaturan yang cocok", 11.0, false, th.muted);
            return;
        }
        let off = self.set_scroll.min(rows.len().saturating_sub(ROWS_VISIBLE));
        for (i, row) in rows.iter().skip(off).take(ROWS_VISIBLE).enumerate() {
            let y = top + 42.0 + i as f32 * 30.0;
            let cy = y + 15.0;
            p.line_segment([pos2(left + 6.0, y + 29.0), pos2(right - 6.0, y + 29.0)], Stroke::new(1.0_f32, th.track));
            text(p, pos2(left + 8.0, cy), Align2::LEFT_CENTER, row.label, 12.0, false, th.fg);
            match &row.ctl {
                Ctl::Toggle(on) => {
                    let tr = Rect::from_min_size(pos2(right - 8.0 - 38.0, cy - 11.0), vec2(38.0, 22.0));
                    if g.toggle(&format!("set_t{i}"), tr, *on) {
                        acts.push(Action::Set(row.id, 0));
                    }
                }
                Ctl::Stepper(label) => {
                    let d = g.stepper(&format!("set_s{i}"), pos2(right - 8.0 - 62.0, cy), label);
                    if d != 0 {
                        acts.push(Action::Set(row.id, d));
                    }
                }
                Ctl::Button(label) => {
                    let br = Rect::from_min_size(pos2(right - 8.0 - 150.0, cy - 12.0), vec2(150.0, 24.0));
                    if g.button(&format!("set_b{i}"), br, label, false) {
                        acts.push(Action::Set(row.id, 0));
                    }
                }
            }
        }
        if rows.len() > ROWS_VISIBLE {
            text(
                p,
                pos2(right, top + 42.0 + ROWS_VISIBLE as f32 * 30.0 + 8.0),
                Align2::RIGHT_CENTER,
                &format!("{}-{} dari {} (gulir mouse)", off + 1, (off + ROWS_VISIBLE).min(rows.len()), rows.len()),
                9.0,
                false,
                th.muted,
            );
        }
    }
}
