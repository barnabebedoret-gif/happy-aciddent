//! Moteur du Happy Accident RISE : un seul réglage TENSION (0 → 1) pilote
//! tous les ingrédients de la montée. À TENSION = 0 le son passe intact.
//! Indépendant du format plugin (tests et démos l'utilisent directement).

use crate::engine::dsp::{sanitize, DelayLine, OnePole, Svf};
use crate::engine::rng::Rng;
use crate::engine::Transport;
use std::f32::consts::PI;

pub const NUM_INGREDIENTS: usize = 8;
pub const LOW_CUT: usize = 0;
pub const SPACE: usize = 1;
pub const WIDTH: usize = 2;
pub const NOISE: usize = 3;
pub const ROLL: usize = 4;
pub const DRIVE: usize = 5;
pub const PITCH: usize = 6;
pub const GAP: usize = 7;

pub const INGREDIENT_NAMES: [&str; NUM_INGREDIENTS] = ["LOW CUT", "SPACE", "WIDTH", "NOISE", "ROLL", "DRIVE", "PITCH", "GAP"];
/// Ingrédients allumés par défaut
pub const DEFAULT_ON: [bool; NUM_INGREDIENTS] = [true, true, true, false, false, false, false, false];

/// Au-delà de ce niveau de TENSION (réglage brut), GAP coupe le son.
pub const GAP_THRESHOLD: f32 = 0.97;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Curve {
    Linear,
    Late,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Amount {
    Subtle,
    Normal,
    Extreme,
}

impl Amount {
    pub fn scale(self) -> f32 {
        match self {
            Amount::Subtle => 0.55,
            Amount::Normal => 1.0,
            Amount::Extreme => 1.6,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tail {
    Cut,
    Keep,
}

#[derive(Clone, Copy, Debug)]
pub struct RiseMacros {
    pub tension: f32,
    pub curve: Curve,
    pub amount: Amount,
    pub tail: Tail,
    pub on: [bool; NUM_INGREDIENTS],
    pub out_gain: f32,
}

impl Default for RiseMacros {
    fn default() -> Self {
        RiseMacros { tension: 0.0, curve: Curve::Late, amount: Amount::Normal, tail: Tail::Cut, on: DEFAULT_ON, out_gain: 1.0 }
    }
}

/// Recette tirée par ACCIDENT : quels ingrédients, quelle courbe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Recipe {
    pub on: [bool; NUM_INGREDIENTS],
    pub curve: Curve,
}

impl Recipe {
    /// Tirage déterministe. Toujours au moins un ingrédient « de base »
    /// (LOW CUT ou SPACE) et entre 3 et 6 ingrédients au total.
    pub fn roll(seed: u64) -> Recipe {
        let mut rng = Rng::new(seed);
        // Probabilités d'apparition par ingrédient
        let p = [0.85, 0.8, 0.6, 0.45, 0.45, 0.35, 0.3, 0.3];
        loop {
            let mut on = [false; NUM_INGREDIENTS];
            for (o, &pp) in on.iter_mut().zip(p.iter()) {
                *o = rng.chance(pp);
            }
            let n = on.iter().filter(|&&b| b).count();
            if (on[LOW_CUT] || on[SPACE]) && (3..=6).contains(&n) {
                let curve = if rng.chance(0.65) { Curve::Late } else { Curve::Linear };
                return Recipe { on, curve };
            }
        }
    }
}

/// État visible par l'interface.
#[derive(Clone, Copy, Debug, Default)]
pub struct RiseState {
    /// Tension après lissage et courbe
    pub shaped: f32,
    pub gap_active: bool,
}

// ------------------------------------------------------------ briques

/// Pitch shifter à deux têtes de lecture croisées (type « delay glissant »).
struct PitchShifter {
    line: DelayLine,
    phase: f32,
    window: f32,
}

impl PitchShifter {
    fn new(sr: f32) -> Self {
        let window = (0.045 * sr).max(64.0);
        PitchShifter { line: DelayLine::new(window as usize * 2 + 8), phase: 0.0, window }
    }
    #[inline]
    fn tick(&mut self, x: f32, ratio: f32) -> f32 {
        self.line.write(x);
        self.phase += (1.0 - ratio) / self.window;
        self.phase -= self.phase.floor();
        let p2 = (self.phase + 0.5).fract();
        let a = self.line.read(self.phase * self.window + 1.0) * (PI * self.phase).sin();
        let b = self.line.read(p2 * self.window + 1.0) * (PI * p2).sin();
        a + b
    }
    fn clear(&mut self) {
        self.line.clear();
        self.phase = 0.0;
    }
}

/// Allpass de Schroeder (diffusion d'entrée de la reverb).
struct Allpass {
    line: DelayLine,
    len: f32,
}

impl Allpass {
    fn new(len: usize) -> Self {
        Allpass { line: DelayLine::new(len + 4), len: len as f32 }
    }
    #[inline]
    fn tick(&mut self, x: f32, g: f32) -> f32 {
        let d = self.line.read(self.len - 1.0);
        let w = x + g * d;
        self.line.write(w);
        d - g * w
    }
}

/// Reverb FDN 4 lignes (matrice de Hadamard), stéréo.
struct Reverb {
    lines: [DelayLine; 4],
    lens: [f32; 4],
    damp: [OnePole; 4],
    diff: [Allpass; 4],
    hp: [Svf; 2],
    sr: f32,
}

impl Reverb {
    fn new(sr: f32) -> Self {
        let k = sr / 44100.0;
        let ms = [41.3_f32, 53.9, 67.1, 79.7];
        let lens = ms.map(|m| m * 0.001 * sr);
        Reverb {
            lines: lens.map(|l| DelayLine::new(l as usize + 8)),
            lens,
            damp: [OnePole::default(); 4],
            diff: [142.0, 107.0, 379.0, 277.0].map(|l: f32| Allpass::new((l * k) as usize)),
            hp: [Svf::default(); 2],
            sr,
        }
    }

    /// `fb` : feedback (durée de la queue), retourne (gauche, droite) 100 % wet.
    #[inline]
    fn tick(&mut self, l: f32, r: f32, fb: f32) -> (f32, f32) {
        let (g, k) = Svf::coefs(180.0, 0.707, self.sr);
        let l = self.hp[0].tick(l, g, k).2;
        let r = self.hp[1].tick(r, g, k).2;
        let l = self.diff[0].tick(l, 0.6);
        let l = self.diff[1].tick(l, 0.6);
        let r = self.diff[2].tick(r, 0.6);
        let r = self.diff[3].tick(r, 0.6);

        let c = OnePole::coef(5500.0, self.sr);
        let mut o = [0.0f32; 4];
        for i in 0..4 {
            o[i] = self.damp[i].lp(self.lines[i].read(self.lens[i]), c);
        }
        // Hadamard 4×4 normalisée
        let h = [
            0.5 * (o[0] + o[1] + o[2] + o[3]),
            0.5 * (o[0] - o[1] + o[2] - o[3]),
            0.5 * (o[0] + o[1] - o[2] - o[3]),
            0.5 * (o[0] - o[1] - o[2] + o[3]),
        ];
        let ins = [l, r, l, r];
        for i in 0..4 {
            self.lines[i].write(sanitize(ins[i] * 0.5 + h[i] * fb));
        }
        (o[0] + 0.6 * o[2], o[1] + 0.6 * o[3])
    }

    fn clear(&mut self) {
        self.lines.iter_mut().for_each(|l| l.clear());
        self.diff.iter_mut().for_each(|a| a.line.clear());
        self.damp = [OnePole::default(); 4];
        self.hp = [Svf::default(); 2];
    }
}

// ------------------------------------------------------------ moteur

pub struct RiseEngine {
    sr: f32,
    /// Tension brute lissée
    smooth: f32,
    pitch: [PitchShifter; 2],
    hp1: [Svf; 2],
    hp2: [Svf; 2],
    noise_f: [Svf; 2],
    reverb: Reverb,
    rng: Rng,
    roll_gain: f32,
    gap_gain: f32,
    /// Niveau de retour de la reverb et feedback « tenus » (mode TAIL Keep)
    rev_ret: f32,
    rev_fb: f32,
    internal_beats: f64,
    lim_env: f32,
    state: RiseState,
}

impl RiseEngine {
    pub fn new(sr: f32, seed: u64) -> Self {
        RiseEngine {
            sr,
            smooth: 0.0,
            pitch: [PitchShifter::new(sr), PitchShifter::new(sr)],
            hp1: [Svf::default(); 2],
            hp2: [Svf::default(); 2],
            noise_f: [Svf::default(); 2],
            reverb: Reverb::new(sr),
            rng: Rng::new(seed),
            roll_gain: 1.0,
            gap_gain: 1.0,
            rev_ret: 0.0,
            rev_fb: 0.0,
            internal_beats: 0.0,
            lim_env: 0.0,
            state: RiseState::default(),
        }
    }

    pub fn state(&self) -> RiseState {
        self.state
    }

    pub fn clear(&mut self) {
        self.smooth = 0.0;
        self.pitch.iter_mut().for_each(|p| p.clear());
        self.hp1 = [Svf::default(); 2];
        self.hp2 = [Svf::default(); 2];
        self.noise_f = [Svf::default(); 2];
        self.reverb.clear();
        self.roll_gain = 1.0;
        self.gap_gain = 1.0;
        self.rev_ret = 0.0;
        self.rev_fb = 0.0;
        self.lim_env = 0.0;
        self.state = RiseState::default();
    }

    /// Le drop : tout revient d'un coup.
    fn drop_now(&mut self, target: f32, tail: Tail) {
        self.smooth = target;
        self.pitch.iter_mut().for_each(|p| p.clear());
        self.hp1 = [Svf::default(); 2];
        self.hp2 = [Svf::default(); 2];
        if tail == Tail::Cut {
            self.reverb.clear();
            self.rev_ret = 0.0;
            self.rev_fb = 0.0;
        }
    }

    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], m: &RiseMacros, tr: &Transport) {
        let n = left.len().min(right.len());
        let sr = self.sr;
        let a = m.amount.scale();
        let target = m.tension.clamp(0.0, 1.0);

        // Chute brutale de TENSION = drop
        if target < self.smooth - 0.15 {
            self.drop_now(target, m.tail);
        }
        let up = 1.0 - (-1.0 / (0.025 * sr)).exp();
        let gate_c = 1.0 - (-1.0 / (0.0015 * sr)).exp();
        let gap_c = 1.0 - (-1.0 / (0.004 * sr)).exp();
        let tail_rel = (-1.0 / (2.5 * sr)).exp();

        let tempo = if tr.tempo.is_finite() && tr.tempo > 1.0 { tr.tempo } else { 120.0 };
        let beats_per_sample = tempo / 60.0 / sr as f64;
        let start_beats = match (tr.playing, tr.pos_beats) {
            (true, Some(p)) if p.is_finite() => p,
            _ => self.internal_beats,
        };

        let gap_wanted = m.on[GAP] && target >= GAP_THRESHOLD;

        for i in 0..n {
            self.smooth += up * (target - self.smooth);
            let raw = self.smooth;
            let t = match m.curve {
                Curve::Linear => raw,
                Curve::Late => raw.powf(2.5),
            };
            let mut l = left[i];
            let mut r = right[i];
            let dry_l = l;
            let dry_r = r;

            // PITCH : monte seulement dans le dernier tiers
            if m.on[PITCH] {
                let ramp = ((t - 0.7) / 0.3).clamp(0.0, 1.0);
                if ramp > 0.0 {
                    let semis = ramp * 2.0 * a;
                    let ratio = 2f32.powf(semis / 12.0);
                    let w = (ramp * 6.0).min(1.0);
                    let pl = self.pitch[0].tick(l, ratio);
                    let pr = self.pitch[1].tick(r, ratio);
                    l += w * (pl - l);
                    r += w * (pr - r);
                } else {
                    self.pitch[0].tick(l, 1.0);
                    self.pitch[1].tick(r, 1.0);
                }
            }

            // DRIVE
            if m.on[DRIVE] && t > 0.0 {
                let mix = (t * a).min(1.0);
                let g = 1.0 + 6.0 * t * a;
                let norm = g.powf(-0.85);
                l += mix * ((g * l).tanh() * norm - l);
                r += mix * ((g * r).tanh() * norm - r);
            }

            // LOW CUT : passe-haut 24 dB qui monte, résonance en fin de course
            if m.on[LOW_CUT] {
                let fmax = (600.0 * a).min(1500.0);
                let cutoff = 20.0 * (fmax / 20.0).powf(t);
                let q = 0.707 + 1.0 * t.powi(4) * a.min(1.3);
                let (g, k1) = Svf::coefs(cutoff, 0.707, sr);
                let (_, k2) = Svf::coefs(cutoff, q, sr);
                let hl = self.hp2[0].tick(self.hp1[0].tick(l, g, k1).2, g, k2).2;
                let hr = self.hp2[1].tick(self.hp1[1].tick(r, g, k1).2, g, k2).2;
                // À TENSION 0 le son reste strictement intact
                let w = (t * 25.0).min(1.0);
                l += w * (hl - l);
                r += w * (hr - r);
            }

            // NOISE : bruit filtré qui monte en niveau et en fréquence
            if m.on[NOISE] && t > 0.0 {
                let level = 0.07 * a * t.powf(1.6);
                let f = 400.0 * (9000.0f32 / 400.0).powf(t);
                let (g, k) = Svf::coefs(f, 0.9, sr);
                let nl = self.rng.f32() * 2.0 - 1.0;
                let nr = self.rng.f32() * 2.0 - 1.0;
                l += level * self.noise_f[0].tick(nl, g, k).1;
                r += level * self.noise_f[1].tick(nr, g, k).1;
            }

            // ROLL : gate qui accélère (1/4 → 1/8 → 1/16 → 1/32)
            let beat = start_beats + i as f64 * beats_per_sample;
            let roll_target = if m.on[ROLL] && t > 0.0 {
                let steps = match t {
                    x if x < 0.25 => 1.0,
                    x if x < 0.5 => 2.0,
                    x if x < 0.75 => 4.0,
                    _ => 8.0,
                };
                let open = (beat * steps).rem_euclid(1.0) < 0.5;
                let depth = (t * 1.2 * a).min(1.0);
                if open { 1.0 } else { 1.0 - depth }
            } else {
                1.0
            };
            self.roll_gain += gate_c * (roll_target - self.roll_gain);
            l *= self.roll_gain;
            r *= self.roll_gain;

            // GAP : le vide juste avant le drop (la reverb continue)
            let gap_target = if gap_wanted { 0.0 } else { 1.0 };
            self.gap_gain += gap_c * (gap_target - self.gap_gain);
            l *= self.gap_gain;
            r *= self.gap_gain;

            // SPACE
            if m.on[SPACE] || self.rev_ret > 1e-4 {
                let send = if m.on[SPACE] { t } else { 0.0 };
                let wet = (0.45 * a).min(0.7) * send;
                let fb = 0.72 + 0.21 * send;
                if m.tail == Tail::Keep {
                    self.rev_ret = wet.max(self.rev_ret * tail_rel);
                    self.rev_fb = fb.max(self.rev_fb * tail_rel + fb * (1.0 - tail_rel));
                } else {
                    self.rev_ret = wet;
                    self.rev_fb = fb;
                }
                if self.rev_ret > 1e-4 || send > 0.0 {
                    let (wl, wr) = self.reverb.tick(l * send.sqrt(), r * send.sqrt(), self.rev_fb.min(0.93));
                    l += self.rev_ret * wl;
                    r += self.rev_ret * wr;
                }
            }

            // WIDTH : on élargit la stéréo (mid / side)
            if m.on[WIDTH] && t > 0.0 {
                let side_gain = (1.0 + 1.3 * a * t).min(3.0);
                let mid = 0.5 * (l + r);
                let side = 0.5 * (l - r) * side_gain;
                l = mid + side;
                r = mid - side;
            }

            // Sortie + limiteur de sécurité
            l *= m.out_gain;
            r *= m.out_gain;
            l = sanitize(l);
            r = sanitize(r);
            let peak = l.abs().max(r.abs());
            if peak > self.lim_env {
                self.lim_env = peak;
            } else {
                self.lim_env *= 0.9997;
            }
            if self.lim_env > 0.95 {
                let g = 0.95 / self.lim_env;
                l *= g;
                r *= g;
            }
            // À TENSION 0 et sans limiteur : sortie identique à l'entrée
            if raw == 0.0 && self.rev_ret <= 1e-4 && self.lim_env <= 0.95 && m.out_gain == 1.0 {
                l = dry_l;
                r = dry_r;
            }
            left[i] = l.clamp(-0.99, 0.99);
            right[i] = r.clamp(-0.99, 0.99);
        }

        self.internal_beats = start_beats + n as f64 * beats_per_sample;
        let raw = self.smooth;
        self.state = RiseState {
            shaped: match m.curve {
                Curve::Linear => raw,
                Curve::Late => raw.powf(2.5),
            },
            gap_active: gap_wanted,
        };
    }
}
