// Interpolasi ease-out untuk animasi expand / collapse (dasar dari azay).

use std::time::Instant;

/// Ease-out cubic: cepat di awal, melambat halus di akhir.
pub fn ease_out_cubic(t: f32) -> f32 {
    let u = 1.0 - t.clamp(0.0, 1.0);
    1.0 - u * u * u
}

/// Tween 3 nilai sekaligus: [lebar, tinggi, offset-atas].
#[derive(Clone, Copy)]
pub struct Tween {
    from: [f32; 3],
    to: [f32; 3],
    start: Instant,
    dur: f32,
}

impl Tween {
    pub fn new(v: [f32; 3]) -> Self {
        Self { from: v, to: v, start: Instant::now(), dur: 0.0 }
    }

    fn progress(&self, now: Instant) -> f32 {
        if self.dur <= 0.0 {
            1.0
        } else {
            (now.duration_since(self.start).as_secs_f32() / self.dur).clamp(0.0, 1.0)
        }
    }

    pub fn value(&self, now: Instant) -> [f32; 3] {
        let e = ease_out_cubic(self.progress(now));
        let mut out = [0.0; 3];
        for i in 0..3 {
            out[i] = self.from[i] + (self.to[i] - self.from[i]) * e;
        }
        out
    }

    /// Mulai animasi baru dari posisi saat ini (tidak melompat).
    pub fn retarget(&mut self, now: Instant, to: [f32; 3], dur: f32) {
        self.from = self.value(now);
        self.to = to;
        self.start = now;
        self.dur = dur;
    }

    pub fn running(&self, now: Instant) -> bool {
        self.progress(now) < 1.0
    }
}
