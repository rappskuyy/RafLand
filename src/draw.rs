// Primitif gambar bergaya "glass" (kaca buram ala macOS): tema, teks, ikon vektor,
// robot kecil, serta helper tombol/toggle/slider/ring.

use eframe::egui::{
    self,
    epaint::text::{LayoutJob, TextFormat, TextWrapping},
    pos2, vec2, Align2, Color32, CursorIcon, FontData, FontDefinitions, FontFamily, FontId, Painter, Pos2,
    Rect, Response, Sense, Shape, Stroke,
};

#[derive(Clone, Copy)]
pub struct Theme {
    pub dark: bool,
    pub bg: Color32,
    pub fg: Color32,
    pub muted: Color32,
    pub track: Color32,
    pub hover: Color32,
    pub card: Color32,
    pub line: Color32,
    pub accent: Color32,
    pub green: Color32,
    pub red: Color32,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            dark: true,
            bg: Color32::from_rgba_unmultiplied(18, 18, 26, 192),
            fg: Color32::from_rgb(245, 246, 250),
            muted: Color32::from_rgba_unmultiplied(235, 235, 250, 150),
            track: Color32::from_white_alpha(30),
            hover: Color32::from_white_alpha(48),
            card: Color32::from_white_alpha(16),
            line: Color32::from_white_alpha(46),
            accent: Color32::from_rgb(176, 148, 255),
            green: Color32::from_rgb(48, 209, 88),
            red: Color32::from_rgb(255, 69, 58),
        }
    }

    pub fn light() -> Self {
        Self {
            dark: false,
            bg: Color32::from_rgba_unmultiplied(248, 248, 252, 205),
            fg: Color32::from_rgb(24, 24, 32),
            muted: Color32::from_rgba_unmultiplied(40, 40, 60, 150),
            track: Color32::from_black_alpha(26),
            hover: Color32::from_black_alpha(42),
            card: Color32::from_black_alpha(12),
            line: Color32::from_black_alpha(34),
            accent: Color32::from_rgb(124, 92, 230),
            green: Color32::from_rgb(20, 168, 75),
            red: Color32::from_rgb(225, 45, 35),
        }
    }
}

pub fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let mut prop = fonts.families[&FontFamily::Proportional].clone();
    if let Ok(d) = std::fs::read(r"C:\Windows\Fonts\segoeui.ttf") {
        fonts.font_data.insert("segoe".into(), FontData::from_owned(d));
        prop.insert(0, "segoe".into());
    }
    let mut bold = prop.clone();
    if let Ok(d) = std::fs::read(r"C:\Windows\Fonts\segoeuib.ttf") {
        fonts.font_data.insert("segoe-bold".into(), FontData::from_owned(d));
        bold.insert(0, "segoe-bold".into());
    }
    fonts.families.insert(FontFamily::Proportional, prop);
    fonts.families.insert(FontFamily::Name("bold".into()), bold);
    ctx.set_fonts(fonts);
}

pub fn fade(c: Color32, a: f32) -> Color32 {
    c.gamma_multiply(a.clamp(0.0, 1.0))
}

fn family(bold: bool) -> FontFamily {
    if bold { FontFamily::Name("bold".into()) } else { FontFamily::Proportional }
}

pub fn text(p: &Painter, pos: Pos2, anchor: Align2, s: &str, size: f32, bold: bool, col: Color32) {
    p.text(pos, anchor, s, FontId::new(size, family(bold)), col);
}

/// Teks satu baris dengan elipsis; `pos` = pojok kiri-atas.
pub fn text_ellipsis(p: &Painter, pos: Pos2, s: &str, size: f32, bold: bool, col: Color32, max_w: f32) {
    let mut job = LayoutJob::single_section(
        s.to_owned(),
        TextFormat { font_id: FontId::new(size, family(bold)), color: col, ..Default::default() },
    );
    job.wrap = TextWrapping {
        max_width: max_w.max(10.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let galley = p.layout_job(job);
    p.galley(pos, galley, col);
}

pub fn tri(p: &Painter, a: Pos2, b: Pos2, c: Pos2, col: Color32) {
    p.add(Shape::convex_polygon(vec![a, b, c], col, Stroke::NONE));
}

pub fn icon_play(p: &Painter, c: Pos2, s: f32, col: Color32) {
    tri(
        p,
        pos2(c.x - s * 0.34, c.y - s * 0.5),
        pos2(c.x - s * 0.34, c.y + s * 0.5),
        pos2(c.x + s * 0.56, c.y),
        col,
    );
}

pub fn icon_pause(p: &Painter, c: Pos2, s: f32, col: Color32) {
    for dx in [-0.24f32, 0.24] {
        let r = Rect::from_center_size(pos2(c.x + s * dx, c.y), vec2(s * 0.28, s));
        p.rect_filled(r, 1.5, col);
    }
}

pub fn icon_next(p: &Painter, c: Pos2, s: f32, col: Color32) {
    tri(
        p,
        pos2(c.x - s * 0.55, c.y - s * 0.5),
        pos2(c.x - s * 0.55, c.y + s * 0.5),
        pos2(c.x + s * 0.35, c.y),
        col,
    );
    p.rect_filled(Rect::from_center_size(pos2(c.x + s * 0.5, c.y), vec2(s * 0.16, s)), 1.0, col);
}

pub fn icon_prev(p: &Painter, c: Pos2, s: f32, col: Color32) {
    tri(
        p,
        pos2(c.x + s * 0.55, c.y - s * 0.5),
        pos2(c.x + s * 0.55, c.y + s * 0.5),
        pos2(c.x - s * 0.35, c.y),
        col,
    );
    p.rect_filled(Rect::from_center_size(pos2(c.x - s * 0.5, c.y), vec2(s * 0.16, s)), 1.0, col);
}

pub fn icon_close(p: &Painter, c: Pos2, s: f32, col: Color32) {
    let st = Stroke::new(1.6_f32, col);
    let h = s * 0.5;
    p.line_segment([c + vec2(-h, -h), c + vec2(h, h)], st);
    p.line_segment([c + vec2(-h, h), c + vec2(h, -h)], st);
}

pub fn icon_battery(p: &Painter, tl: Pos2, level: f32, outline: Color32, fill: Color32) {
    let body = Rect::from_min_size(tl, vec2(22.0, 11.0));
    p.rect_stroke(body, 3.0, Stroke::new(1.3_f32, outline));
    let inner = Rect::from_min_size(tl + vec2(2.0, 2.0), vec2(18.0 * level.clamp(0.08, 1.0), 7.0));
    p.rect_filled(inner, 1.6, fill);
    p.rect_filled(Rect::from_min_size(tl + vec2(23.0, 3.5), vec2(2.0, 4.0)), 1.0, outline);
}

/// Mikrofon sederhana; `slashed` = dibisukan (diberi garis coret).
pub fn icon_mic(p: &Painter, c: Pos2, s: f32, col: Color32, slashed: bool) {
    let body = Rect::from_center_size(c + vec2(0.0, -s * 0.12), vec2(s * 0.42, s * 0.74));
    p.rect_filled(body, s * 0.21, col);
    let st = Stroke::new(1.4_f32, col);
    p.line_segment([c + vec2(-s * 0.3, s * 0.12), c + vec2(-s * 0.3, s * 0.2)], st);
    p.line_segment([c + vec2(s * 0.3, s * 0.12), c + vec2(s * 0.3, s * 0.2)], st);
    p.line_segment([c + vec2(-s * 0.3, s * 0.2), c + vec2(s * 0.3, s * 0.2)], st);
    p.line_segment([c + vec2(0.0, s * 0.2), c + vec2(0.0, s * 0.45)], st);
    if slashed {
        p.line_segment([c + vec2(-s * 0.5, -s * 0.5), c + vec2(s * 0.5, s * 0.5)], Stroke::new(1.8_f32, col));
    }
}


fn arc(p: &Painter, c: Pos2, r: f32, a0: f32, a1: f32, st: Stroke) {
    let n = 18;
    let mut prev = pos2(c.x + r * a0.cos(), c.y + r * a0.sin());
    for i in 1..=n {
        let a = a0 + (a1 - a0) * (i as f32 / n as f32);
        let pt = pos2(c.x + r * a.cos(), c.y + r * a.sin());
        p.line_segment([prev, pt], st);
        prev = pt;
    }
}

/// Ikon navigasi: 0 musik, 1 kalender, 2 bulan, 3 folder, 4 timer, 5 mata, 6 tetes air, 7 kamera, 8 gigi.
pub fn nav_icon(p: &Painter, kind: u8, c: Pos2, s: f32, col: Color32) {
    let st = Stroke::new(1.5_f32, col);
    let h = s * 0.5;
    match kind {
        0 => {
            p.circle_filled(c + vec2(-h * 0.45, h * 0.6), h * 0.38, col);
            p.line_segment([c + vec2(h * 0.0, h * 0.6), c + vec2(h * 0.0, -h)], st);
            p.line_segment([c + vec2(0.0, -h), c + vec2(h * 0.7, -h * 0.5)], st);
        }
        1 => {
            let r = Rect::from_center_size(c + vec2(0.0, h * 0.1), vec2(s, s * 0.86));
            p.rect_stroke(r, 3.0, st);
            p.line_segment([r.min + vec2(0.0, s * 0.28), r.max - vec2(0.0, s * 0.58)], st);
            p.line_segment([c + vec2(-h * 0.5, -h * 0.9), c + vec2(-h * 0.5, -h * 0.5)], st);
            p.line_segment([c + vec2(h * 0.5, -h * 0.9), c + vec2(h * 0.5, -h * 0.5)], st);
        }
        2 => arc(p, c, h * 0.9, 0.9, 5.4, Stroke::new(2.4_f32, col)),
        3 => {
            let r = Rect::from_center_size(c + vec2(0.0, h * 0.15), vec2(s, s * 0.72));
            p.rect_stroke(r, 3.0, st);
            p.line_segment([r.min + vec2(0.0, -1.0), r.min + vec2(s * 0.4, -1.0)], st);
        }
        4 => {
            p.circle_stroke(c + vec2(0.0, h * 0.1), h * 0.9, st);
            p.line_segment([c + vec2(0.0, h * 0.1), c + vec2(0.0, -h * 0.5)], st);
            p.line_segment([c + vec2(-h * 0.3, -h * 1.15), c + vec2(h * 0.3, -h * 1.15)], st);
        }
        5 => {
            let a = std::f32::consts::PI;
            arc(p, c + vec2(0.0, h * 0.9), h * 1.4, a * 1.2, a * 1.8, st);
            arc(p, c - vec2(0.0, h * 0.9), h * 1.4, a * 0.2, a * 0.8, st);
            p.circle_filled(c, h * 0.35, col);
        }
        6 => {
            p.circle_filled(c + vec2(0.0, h * 0.3), h * 0.7, col);
            tri(p, c + vec2(-h * 0.62, h * 0.1), c + vec2(h * 0.62, h * 0.1), c + vec2(0.0, -h * 1.0), col);
        }
        7 => {
            let r = Rect::from_center_size(c + vec2(0.0, h * 0.12), vec2(s, s * 0.7));
            p.rect_stroke(r, 3.0, st);
            p.circle_stroke(r.center(), h * 0.35, st);
            p.line_segment([r.min + vec2(s * 0.28, -1.5), r.min + vec2(s * 0.5, -1.5)], st);
        }
        _ => {
            p.circle_stroke(c, h * 0.55, st);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::FRAC_PI_4;
                let (sn, cs) = a.sin_cos();
                p.line_segment([c + vec2(cs * h * 0.8, sn * h * 0.8), c + vec2(cs * h * 1.05, sn * h * 1.05)], Stroke::new(2.0_f32, col));
            }
        }
    }
}

/// Efek kaca: bayangan lembut, isian tembus pandang, kilau atas bertingkat, dan tepi tipis terang.
pub fn glass(p: &Painter, rect: Rect, radius: f32, th: &Theme, k: f32) {
    let sh = (4.0 * k) as u8;
    for i in (1..=8).rev() {
        let g = i as f32 * 2.5;
        p.rect_filled(rect.expand(g).translate(vec2(0.0, 5.0)), radius + g, Color32::from_black_alpha(sh));
    }
    p.rect_filled(rect, radius, th.bg);
    let boost: u8 = if th.dark { 0 } else { 18 };
    for (frac, a) in [(0.60f32, 8u8), (0.38, 9), (0.18, 12)] {
        let clip = Rect::from_min_max(rect.min, pos2(rect.max.x, rect.min.y + rect.height() * frac));
        p.with_clip_rect(clip).rect_filled(rect, radius, Color32::from_white_alpha(a + boost));
    }
    p.rect_stroke(rect, radius, Stroke::new(1.0_f32, th.line));
}

#[derive(Clone, Copy, PartialEq)]
pub enum Mood {
    Normal,
    Happy,
    Thirsty,
    Alert,
}

/// Robot kecil (kepala bulat dengan antena). Berkedip tiap beberapa detik.
pub fn robot(p: &Painter, c: Pos2, s: f32, mood: Mood, t: f32, accent: Color32, fg: Color32) {
    let head = Rect::from_center_size(c + vec2(0.0, s * 0.1), vec2(s * 1.6, s * 1.2));
    p.rect_filled(head, s * 0.45, accent.gamma_multiply(0.28));
    p.rect_stroke(head, s * 0.45, Stroke::new(1.3_f32, accent));
    let top = pos2(c.x, head.min.y);
    p.line_segment([top, top + vec2(0.0, -s * 0.28)], Stroke::new(1.2_f32, accent));
    p.circle_filled(top + vec2(0.0, -s * 0.34), s * 0.1, accent);

    let ey = head.center().y - s * 0.05;
    let ex = s * 0.32;
    let blink = (t % 4.2) > 4.05;
    for dx in [-ex, ex] {
        let e = pos2(c.x + dx, ey);
        match mood {
            Mood::Happy => {
                let r = s * 0.16;
                let st = Stroke::new(1.5_f32, fg);
                p.line_segment([e + vec2(-r, r * 0.5), e + vec2(0.0, -r * 0.6)], st);
                p.line_segment([e + vec2(0.0, -r * 0.6), e + vec2(r, r * 0.5)], st);
            }
            Mood::Thirsty => {
                p.circle_filled(e, s * 0.12, fg);
                let r = s * 0.16;
                let sign = if dx < 0.0 { 1.0 } else { -1.0 };
                p.line_segment(
                    [e + vec2(-sign * r, -s * 0.30), e + vec2(sign * r, -s * 0.42)],
                    Stroke::new(1.3_f32, fg),
                );
            }
            Mood::Alert => {
                p.circle_filled(e, s * 0.17, fg);
            }
            Mood::Normal => {
                if blink {
                    p.line_segment(
                        [e + vec2(-s * 0.12, 0.0), e + vec2(s * 0.12, 0.0)],
                        Stroke::new(1.5_f32, fg),
                    );
                } else {
                    p.circle_filled(e, s * 0.13, fg);
                }
            }
        }
    }
    // mulut
    let my = head.center().y + s * 0.3;
    let st = Stroke::new(1.3_f32, fg);
    match mood {
        Mood::Thirsty => p.circle_stroke(pos2(c.x, my), s * 0.1, st),
        Mood::Happy => p.line_segment([pos2(c.x - s * 0.16, my), pos2(c.x + s * 0.16, my)], st),
        _ => p.line_segment([pos2(c.x - s * 0.1, my), pos2(c.x + s * 0.1, my)], st),
    }
    if mood == Mood::Thirsty {
        // tetes air di samping kepala
        let d = c + vec2(s * 1.15, -s * 0.05);
        let drop = Color32::from_rgb(90, 190, 255);
        p.circle_filled(d + vec2(0.0, s * 0.1), s * 0.14, drop);
        tri(p, d + vec2(-s * 0.12, s * 0.05), d + vec2(s * 0.12, s * 0.05), d + vec2(0.0, -s * 0.3), drop);
    }
}

/// Tiga batang equalizer kecil yang bergerak saat musik diputar.
pub fn eq_bars(p: &Painter, c: Pos2, t: f32, playing: bool, col: Color32) {
    for i in 0..3 {
        let phase = t * 6.0 + i as f32 * 1.7;
        let h = if playing { 5.0 + 7.0 * (0.5 + 0.5 * phase.sin()) } else { 5.0 };
        let x = c.x + (i as f32 - 1.0) * 6.0;
        p.rect_filled(Rect::from_center_size(pos2(x, c.y), vec2(3.0, h)), 1.5, col);
    }
}

/// Kumpulan helper interaktif; memegang painter yang sudah di-clip ke bentuk island.
pub struct G<'a> {
    pub ui: &'a egui::Ui,
    pub p: Painter,
    pub th: Theme,
}

impl<'a> G<'a> {
    pub fn id(&self, s: &str) -> egui::Id {
        self.ui.id().with(s)
    }

    pub fn click(&self, id: &str, r: Rect) -> Response {
        let resp = self.ui.interact(r, self.id(id), Sense::click());
        if resp.hovered() {
            self.ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        resp
    }

    pub fn card(&self, r: Rect, radius: f32) {
        self.p.rect_filled(r, radius, self.th.card);
        self.p.rect_stroke(r, radius, Stroke::new(1.0_f32, self.th.line));
    }

    /// Tombol kapsul. `primary` = berwarna aksen.
    pub fn button(&self, id: &str, r: Rect, label: &str, primary: bool) -> bool {
        let resp = self.click(id, r);
        let fill = if primary {
            self.th.accent
        } else if resp.hovered() {
            self.th.hover
        } else {
            self.th.track
        };
        self.p.rect_filled(r, r.height() / 2.0, fill);
        let col = if primary { Color32::WHITE } else { self.th.fg };
        text(&self.p, r.center(), Align2::CENTER_CENTER, label, 11.0, true, col);
        resp.clicked()
    }

    /// Tombol kapsul dengan warna isian sendiri.
    pub fn tint_button(&self, id: &str, r: Rect, label: &str, fill: Color32, fg: Color32) -> bool {
        let resp = self.click(id, r);
        self.p.rect_filled(r, r.height() / 2.0, fill);
        if resp.hovered() {
            self.p.rect_filled(r, r.height() / 2.0, Color32::from_white_alpha(18));
        }
        text(&self.p, r.center(), Align2::CENTER_CENTER, label, 11.0, true, fg);
        resp.clicked()
    }

    /// Sakelar 38x22.
    pub fn toggle(&self, id: &str, r: Rect, on: bool) -> bool {
        let resp = self.click(id, r);
        let fill = if on { self.th.accent } else { self.th.track };
        self.p.rect_filled(r, r.height() / 2.0, fill);
        let kx = if on { r.max.x - r.height() / 2.0 } else { r.min.x + r.height() / 2.0 };
        self.p.circle_filled(pos2(kx, r.center().y), r.height() / 2.0 - 3.0, Color32::WHITE);
        resp.clicked()
    }

    /// [-] nilai [+]; mengembalikan -1, 0, atau +1.
    pub fn stepper(&self, id: &str, c: Pos2, label: &str) -> i32 {
        let m = Rect::from_center_size(pos2(c.x - 50.0, c.y), vec2(24.0, 24.0));
        let pl = Rect::from_center_size(pos2(c.x + 50.0, c.y), vec2(24.0, 24.0));
        let rm = self.click(&format!("{id}_m"), m);
        let rp = self.click(&format!("{id}_p"), pl);
        self.p.circle_filled(m.center(), 12.0, if rm.hovered() { self.th.hover } else { self.th.track });
        self.p.circle_filled(pl.center(), 12.0, if rp.hovered() { self.th.hover } else { self.th.track });
        text(&self.p, m.center(), Align2::CENTER_CENTER, "-", 14.0, true, self.th.fg);
        text(&self.p, pl.center(), Align2::CENTER_CENTER, "+", 14.0, true, self.th.fg);
        text(&self.p, c, Align2::CENTER_CENTER, label, 11.0, true, self.th.fg);
        if rm.clicked() {
            -1
        } else if rp.clicked() {
            1
        } else {
            0
        }
    }

    /// Slider yang bisa diklik/digeser; mengembalikan persen baru (0-100).
    pub fn slider(&self, id: &str, r: Rect, label: &str, frac: f32) -> Option<u8> {
        let resp = self.ui.interact(r, self.id(id), Sense::click_and_drag());
        let f = frac.clamp(0.0, 1.0);
        let rad = r.height() / 2.0;
        self.p.rect_filled(r, rad, self.th.track);
        let clip = Rect::from_min_max(r.min, pos2(r.min.x + r.width() * f, r.max.y));
        self.p.with_clip_rect(clip).rect_filled(r, rad, self.th.accent.gamma_multiply(0.85));
        text(&self.p, pos2(r.min.x + 16.0, r.center().y), Align2::LEFT_CENTER, label, 11.0, true, self.th.fg);
        text(
            &self.p,
            pos2(r.max.x - 14.0, r.center().y),
            Align2::RIGHT_CENTER,
            &format!("{}%", (f * 100.0).round() as u32),
            11.0,
            true,
            self.th.fg,
        );
        if resp.hovered() || resp.dragged() {
            self.ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if resp.clicked() || resp.dragged() {
            if let Some(pt) = resp.interact_pointer_pos() {
                let v = ((pt.x - r.min.x) / r.width()).clamp(0.0, 1.0);
                return Some((v * 100.0).round() as u8);
            }
        }
        None
    }

    /// Cincin progres (mulai dari atas, searah jarum jam).
    pub fn ring(&self, c: Pos2, r: f32, frac: f32, col: Color32, w: f32) {
        self.p.circle_stroke(c, r, Stroke::new(w, self.th.track));
        let f = frac.clamp(0.0, 1.0);
        if f <= 0.0 {
            return;
        }
        let n = ((f * 90.0).ceil() as usize).max(2);
        let mut prev = pos2(c.x, c.y - r);
        for i in 1..=n {
            let a = -std::f32::consts::FRAC_PI_2 + f * std::f32::consts::TAU * (i as f32 / n as f32);
            let pt = pos2(c.x + r * a.cos(), c.y + r * a.sin());
            self.p.line_segment([prev, pt], Stroke::new(w, col));
            prev = pt;
        }
    }

    /// Batang progres tipis.
    pub fn bar(&self, r: Rect, frac: f32, col: Color32) {
        let rad = r.height() / 2.0;
        self.p.rect_filled(r, rad, self.th.track);
        let w = r.width() * frac.clamp(0.0, 1.0);
        if w > 1.0 {
            self.p.rect_filled(Rect::from_min_size(r.min, vec2(w, r.height())), rad, col);
        }
    }
}
