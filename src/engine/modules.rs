//! Les 6 modules d'effets. Chacun traite un échantillon stéréo à la fois.

use super::dsp::*;
use super::patch::*;
use super::rng::hash01;
use std::f32::consts::PI;

/// Contexte partagé pour un échantillon.
pub struct Ctx {
    pub sr: f32,
    /// Position en temps (noires)
    pub beat: f64,
    /// Échantillons par temps
    pub spb: f64,
    pub key: i32,
    pub scale: Scale,
    pub seed: u64,
}

#[inline]
fn step_index(beat: f64, div: f64) -> i64 {
    (beat / div).floor() as i64
}

/// Valeur de LFO synchronisée au tempo, dans [-1, 1].
pub fn lfo_value(l: &ModLfo, beat: f64, seed: u64, idx: u64) -> f32 {
    let period = LFO_BEATS[l.rate as usize % LFO_BEATS.len()];
    let ph = (beat / period).rem_euclid(1.0) as f32;
    match l.shape {
        0 => (2.0 * PI * ph).sin(),
        1 => 4.0 * (ph - 0.5).abs() - 1.0,
        2 => 2.0 * ph - 1.0,
        _ => hash01(seed ^ (idx << 32), step_index(beat, period) as u64) * 2.0 - 1.0,
    }
}

// ---------------------------------------------------------------- GATE
#[derive(Default)]
pub struct Gate {
    env: OnePole,
}

impl Gate {
    pub fn process(&mut self, l: f32, r: f32, p: &GateP, c: &Ctx, m: f32) -> (f32, f32) {
        let div = DIVS[p.division as usize % 4];
        let step = step_index(c.beat, div).rem_euclid(16) as u32;
        let on = (p.pattern >> step) & 1 == 1;
        let depth = (p.depth + m * 0.3).clamp(0.0, 1.0);
        let target = if on { 1.0 } else { 1.0 - depth };
        let ms = 1.0 + p.smooth * 25.0;
        let coef = 1.0 - (-1.0 / (ms * 0.001 * c.sr)).exp();
        let g = self.env.lp(target, coef);
        (l * g, r * g)
    }
    pub fn reset(&mut self) {
        self.env.z = 1.0;
    }
}

// ---------------------------------------------------------------- STUTTER / REVERSE
pub struct Stutter {
    dl: DelayLine,
    dr: DelayLine,
    cur_step: i64,
    t: f64,
    seg_len: f64,
    event: u8,
    prev_event: u8,
}

impl Stutter {
    pub fn new(sr: f32) -> Self {
        let n = (sr * 4.0) as usize;
        Stutter { dl: DelayLine::new(n), dr: DelayLine::new(n), cur_step: i64::MIN, t: 0.0, seg_len: 1.0, event: 0, prev_event: 0 }
    }

    pub fn process(&mut self, l: f32, r: f32, p: &StutterP, c: &Ctx) -> (f32, f32) {
        let div = DIVS[p.division as usize % 4];
        let step = step_index(c.beat, div);
        if step != self.cur_step {
            self.prev_event = self.event;
            self.event = p.events[step.rem_euclid(16) as usize];
            self.cur_step = step;
            self.t = 0.0;
            self.seg_len = (div * c.spb).max(64.0);
        }
        self.dl.write(l);
        self.dr.write(r);

        let w = (0.003 * c.sr) as f64; // fondu de 3 ms anti-clic
        let max_d = (self.dl.capacity() - 4) as f64;
        let t = self.t;
        let fade_in = if self.prev_event != 0 { (t / w).min(1.0) } else { 1.0 };

        let (ol, or, env) = match self.event {
            1 => {
                let len = (self.seg_len / p.repeats.max(1) as f64).max(32.0);
                let k = (t / len).floor();
                let tm = t - k * len;
                if k < 1.0 {
                    let env = ((len - tm) / w).min(1.0) * fade_in;
                    (l, r, env)
                } else {
                    let d = (t - tm).min(max_d) as f32;
                    let env = (tm / w).min((len - tm) / w).min(1.0) * (p.decay as f64).powf(k);
                    (self.dl.read(d), self.dr.read(d), env)
                }
            }
            2 => {
                let d = (2.0 * t + 1.0).min(max_d) as f32;
                let env = (t / w).min((self.seg_len - t) / w).clamp(0.0, 1.0);
                (self.dl.read(d), self.dr.read(d), env)
            }
            _ => (l, r, fade_in),
        };
        self.t += 1.0;
        let env = env.max(0.0) as f32;
        (ol * env, or * env)
    }

    pub fn reset(&mut self) {
        self.cur_step = i64::MIN;
        self.event = 0;
        self.prev_event = 0;
    }
}

// ---------------------------------------------------------------- PITCH (granulaire)
pub struct Pitch {
    dl: DelayLine,
    dr: DelayLine,
    phase: f32,
    ratio: f32,
}

impl Pitch {
    pub fn new(sr: f32) -> Self {
        let n = (sr * 0.5) as usize;
        Pitch { dl: DelayLine::new(n), dr: DelayLine::new(n), phase: 0.0, ratio: 1.0 }
    }

    pub fn process(&mut self, l: f32, r: f32, p: &PitchP, c: &Ctx, m: f32) -> (f32, f32) {
        let deg = if p.patterned {
            let div = DIVS[p.division as usize % 4];
            p.steps[step_index(c.beat, div).rem_euclid(8) as usize]
        } else {
            p.steps[0]
        };
        let semis = c.scale.degree_to_semis(deg as i32) as f32 + m * 0.35;
        let target = 2f32.powf(semis / 12.0);
        self.ratio += (target - self.ratio) * 0.004;

        self.dl.write(l);
        self.dr.write(r);
        let grain = (0.02 + p.grain * 0.07) * c.sr;
        self.phase = (self.phase + (1.0 - self.ratio) / grain).rem_euclid(1.0);

        let mut sl = 0.0;
        let mut sr_ = 0.0;
        for tap in [self.phase, (self.phase + 0.5).rem_euclid(1.0)] {
            let d = tap * grain + 1.0;
            let win = (PI * tap).sin().powi(2);
            sl += self.dl.read(d) * win;
            sr_ += self.dr.read(d) * win;
        }
        let b = p.blend;
        (l * (1.0 - b) + sl * b, r * (1.0 - b) + sr_ * b)
    }

    pub fn reset(&mut self) {
        self.ratio = 1.0;
    }
}

// ---------------------------------------------------------------- DRIVE
#[derive(Default)]
pub struct Drive {
    counter: f32,
    held: (f32, f32),
    tl: OnePole,
    tr: OnePole,
}

impl Drive {
    pub fn process(&mut self, l: f32, r: f32, p: &DriveP, c: &Ctx, m: f32) -> (f32, f32) {
        let amt = (p.amount + m * 0.25).clamp(0.0, 1.0);
        let (yl, yr) = match p.mode {
            0 => {
                let g = db_to_gain(amt * 30.0);
                ((l * g).tanh(), (r * g).tanh())
            }
            1 => {
                let g = 1.0 + amt * 8.0;
                ((l * g).sin(), (r * g).sin())
            }
            _ => {
                let bits = 16.0 - amt * 12.0;
                let levels = 2f32.powf(bits);
                let decim = 1.0 + amt * amt * 24.0;
                self.counter += 1.0;
                if self.counter >= decim {
                    self.counter -= decim;
                    let q = |x: f32| ((x * 1.5).clamp(-1.0, 1.0) * levels).round() / levels;
                    self.held = (q(l), q(r));
                }
                self.held
            }
        };
        let cutoff = 600.0 * 30f32.powf(p.tone);
        let k = OnePole::coef(cutoff, c.sr);
        (self.tl.lp(yl, k), self.tr.lp(yr, k))
    }

    pub fn reset(&mut self) {
        *self = Drive::default();
    }
}

// ---------------------------------------------------------------- RESO / FORMANT
const FORMANTS: [[f32; 3]; 5] = [
    [800.0, 1150.0, 2900.0], // A
    [400.0, 1600.0, 2700.0], // E
    [270.0, 2290.0, 3010.0], // I
    [450.0, 800.0, 2830.0],  // O
    [325.0, 700.0, 2700.0],  // U
];

pub struct Reso {
    dl: DelayLine,
    dr: DelayLine,
    damp_l: OnePole,
    damp_r: OnePole,
    bp_l: [Svf; 3],
    bp_r: [Svf; 3],
}

impl Reso {
    pub fn new(sr: f32) -> Self {
        let n = (sr * 0.1) as usize;
        Reso {
            dl: DelayLine::new(n),
            dr: DelayLine::new(n),
            damp_l: OnePole::default(),
            damp_r: OnePole::default(),
            bp_l: [Svf::default(); 3],
            bp_r: [Svf::default(); 3],
        }
    }

    pub fn process(&mut self, l: f32, r: f32, p: &ResoP, c: &Ctx, m: f32) -> (f32, f32) {
        let (yl, yr) = if p.mode == 0 {
            let note = 36 + c.key + c.scale.degree_to_semis(p.degree as i32) + 12 * p.octave as i32;
            let d = c.sr / midi_to_hz(note as f32);
            let fb = p.feedback;
            let damp = OnePole::coef(1500.0 + 7000.0 * (0.5 + 0.5 * m), c.sr);
            let ol = self.dl.read(d - 1.0);
            let or = self.dr.read(d * 1.003 - 1.0);
            let fl = self.damp_l.lp(ol, damp);
            let fr = self.damp_r.lp(or, damp);
            self.dl.write(sanitize(l + fb * fl));
            self.dr.write(sanitize(r + fb * fr));
            let norm = (1.0 - fb).sqrt();
            (ol * norm, or * norm)
        } else {
            let v = ((p.vowel + m * 0.4).clamp(0.0, 1.0)) * 4.0;
            let i = (v.floor() as usize).min(3);
            let f = v - i as f32;
            let gains = [1.0, 0.7, 0.5];
            let mut yl = 0.0;
            let mut yr = 0.0;
            for k in 0..3 {
                let freq = FORMANTS[i][k] + (FORMANTS[i + 1][k] - FORMANTS[i][k]) * f;
                yl += self.bp_l[k].process(l, freq, 9.0, c.sr).1 * gains[k];
                yr += self.bp_r[k].process(r, freq * 1.01, 9.0, c.sr).1 * gains[k];
            }
            (yl * 3.0, yr * 3.0)
        };
        let mx = p.mix;
        (l * (1.0 - mx) + yl * mx, r * (1.0 - mx) + yr * mx)
    }

    pub fn reset(&mut self) {
        self.dl.clear();
        self.dr.clear();
        self.damp_l = OnePole::default();
        self.damp_r = OnePole::default();
        self.bp_l.iter_mut().chain(self.bp_r.iter_mut()).for_each(|f| f.reset());
    }
}

// ---------------------------------------------------------------- SPACE
const SPACE_BEATS: [f64; 4] = [0.25, 0.5, 0.75, 1.0];

pub struct Space {
    dl: DelayLine,
    dr: DelayLine,
    d_smooth: f32,
    lp_l: OnePole,
    lp_r: OnePole,
}

impl Space {
    pub fn new(sr: f32) -> Self {
        let n = (sr * 2.5) as usize;
        Space { dl: DelayLine::new(n), dr: DelayLine::new(n), d_smooth: 0.0, lp_l: OnePole::default(), lp_r: OnePole::default() }
    }

    pub fn process(&mut self, l: f32, r: f32, p: &SpaceP, c: &Ctx, m: f32) -> (f32, f32) {
        let mx = p.mix;
        match p.mode {
            0 => {
                // Ping-pong synchronisé
                let target = (SPACE_BEATS[p.time as usize % 4] * c.spb) as f32;
                let target = target.min((self.dl.capacity() - 8) as f32);
                if self.d_smooth <= 0.0 {
                    self.d_smooth = target;
                }
                self.d_smooth += (target - self.d_smooth) * 0.0005;
                let d = self.d_smooth;
                let ol = self.dl.read(d - 1.0);
                let or = self.dr.read(d - 1.0);
                let k = OnePole::coef(4500.0 + 2500.0 * m, c.sr);
                let fl = self.lp_l.lp(ol, k);
                let fr = self.lp_r.lp(or, k);
                let mono = (l + r) * 0.5;
                self.dl.write(sanitize(mono + p.feedback * fr));
                self.dr.write(sanitize(p.feedback * fl));
                (l + ol * mx, r + or * mx)
            }
            1 => {
                // Flanger
                let period = 2.0 + 6.0 * (1.0 - p.time as f64 / 3.0);
                let ph = (c.beat / period).rem_euclid(1.0) as f32;
                let depth = 0.6 + 0.4 * m.abs();
                let ms_l = 1.0 + 3.0 * depth * (1.0 + (2.0 * PI * ph).sin());
                let ms_r = 1.0 + 3.0 * depth * (1.0 + (2.0 * PI * (ph + 0.25)).sin());
                let ol = self.dl.read(ms_l * 0.001 * c.sr);
                let or = self.dr.read(ms_r * 0.001 * c.sr);
                let fb = p.feedback * 0.9;
                self.dl.write(sanitize(l + fb * ol));
                self.dr.write(sanitize(r + fb * or));
                (l * (1.0 - mx * 0.5) + ol * mx, r * (1.0 - mx * 0.5) + or * mx)
            }
            _ => {
                // Métallique : deux peignes courts à longueurs "aléatoires"
                let d_l = c.sr * (0.003 + 0.011 * hash01(c.seed, 11));
                let d_r = c.sr * (0.003 + 0.011 * hash01(c.seed, 12));
                let fb = 0.55 + 0.35 * p.feedback;
                let ol = self.dl.read(d_l - 1.0);
                let or = self.dr.read(d_r - 1.0);
                let k = OnePole::coef(6000.0, c.sr);
                let fl = self.lp_l.lp(ol, k);
                let fr = self.lp_r.lp(or, k);
                self.dl.write(sanitize(l + fb * fl));
                self.dr.write(sanitize(r + fb * fr));
                let norm = (1.0 - fb).sqrt();
                (l * (1.0 - mx) + ol * norm * mx * 1.5, r * (1.0 - mx) + or * norm * mx * 1.5)
            }
        }
    }

    pub fn reset(&mut self) {
        self.dl.clear();
        self.dr.clear();
        self.d_smooth = 0.0;
        self.lp_l = OnePole::default();
        self.lp_r = OnePole::default();
    }
}
