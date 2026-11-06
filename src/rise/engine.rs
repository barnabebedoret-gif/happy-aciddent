//! Moteur du Happy Accident RISE : TENSION (0 → 1) pilote la montée,
//! CUT, SPACE et ROLL dosent ce qui se passe à 100 %.
//! À TENSION = 0 le son passe intact. Indépendant du format plugin.

use crate::engine::dsp::{sanitize, DelayLine, OnePole, Svf};
use crate::engine::Transport;

/// Courbe fixe « Late » : presque rien au début, tout monte à la fin.
#[inline]
pub fn shape(raw: f32) -> f32 {
    raw.clamp(0.0, 1.0).powf(2.5)
}

/// Fréquence de coupure du passe-haut à 100 % de TENSION selon CUT (150 Hz → 1 kHz).
#[inline]
pub fn cut_max_hz(cut: f32) -> f32 {
    150.0 * (1000.0f32 / 150.0).powf(cut.clamp(0.0, 1.0))
}

/// Division du ROLL selon la tension : 1/4 → 1/8 → 1/16 → 1/32 (pas par temps).
#[inline]
pub fn roll_steps(t: f32) -> f64 {
    match t {
        x if x < 0.25 => 1.0,
        x if x < 0.5 => 2.0,
        x if x < 0.75 => 4.0,
        _ => 8.0,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RiseMacros {
    pub tension: f32,
    pub cut: f32,
    pub space: f32,
    pub roll: f32,
}

impl Default for RiseMacros {
    fn default() -> Self {
        RiseMacros { tension: 0.0, cut: 0.5, space: 0.5, roll: 0.0 }
    }
}

/// État visible par l'interface.
#[derive(Clone, Copy, Debug, Default)]
pub struct RiseState {
    /// Tension après lissage et courbe
    pub shaped: f32,
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
        let lens = [41.3_f32, 53.9, 67.1, 79.7].map(|m| m * 0.001 * sr);
        Reverb {
            lines: lens.map(|l| DelayLine::new(l as usize + 8)),
            lens,
            damp: [OnePole::default(); 4],
            diff: [142.0, 107.0, 379.0, 277.0].map(|l: f32| Allpass::new((l * k) as usize)),
            hp: [Svf::default(); 2],
            sr,
        }
    }

    /// Retourne (gauche, droite) 100 % wet.
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

pub struct RiseEngine {
    sr: f32,
    /// Tension brute lissée
    smooth: f32,
    hp1: [Svf; 2],
    hp2: [Svf; 2],
    reverb: Reverb,
    roll_gain: f32,
    internal_beats: f64,
    lim_env: f32,
    state: RiseState,
}

impl RiseEngine {
    pub fn new(sr: f32) -> Self {
        RiseEngine {
            sr,
            smooth: 0.0,
            hp1: [Svf::default(); 2],
            hp2: [Svf::default(); 2],
            reverb: Reverb::new(sr),
            roll_gain: 1.0,
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
        self.hp1 = [Svf::default(); 2];
        self.hp2 = [Svf::default(); 2];
        self.reverb.clear();
        self.roll_gain = 1.0;
        self.lim_env = 0.0;
        self.state = RiseState::default();
    }

    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], m: &RiseMacros, tr: &Transport) {
        let n = left.len().min(right.len());
        let sr = self.sr;
        let target = m.tension.clamp(0.0, 1.0);
        let space = m.space.clamp(0.0, 1.0);
        let roll = m.roll.clamp(0.0, 1.0);
        let fmax = cut_max_hz(m.cut);

        // Chute brutale de TENSION = le drop : tout revient d'un coup, la reverb est coupée net
        if target < self.smooth - 0.15 {
            let keep_beats = self.internal_beats;
            self.clear();
            self.internal_beats = keep_beats;
            self.smooth = target;
        }
        let up = 1.0 - (-1.0 / (0.025 * sr)).exp();
        let gate_c = 1.0 - (-1.0 / (0.0015 * sr)).exp();

        let tempo = if tr.tempo.is_finite() && tr.tempo > 1.0 { tr.tempo } else { 120.0 };
        let beats_per_sample = tempo / 60.0 / sr as f64;
        let start_beats = match (tr.playing, tr.pos_beats) {
            (true, Some(p)) if p.is_finite() => p,
            _ => self.internal_beats,
        };

        for i in 0..n {
            self.smooth += up * (target - self.smooth);
            let raw = self.smooth;
            let t = shape(raw);
            let dry_l = left[i];
            let dry_r = right[i];
            let mut l = dry_l;
            let mut r = dry_r;

            // CUT : passe-haut 24 dB qui monte, avec un peu de résonance en fin de course
            let cutoff = 20.0 * (fmax / 20.0).powf(t);
            let q = 0.707 + 0.9 * t.powi(4);
            let (g, k1) = Svf::coefs(cutoff, 0.707, sr);
            let (_, k2) = Svf::coefs(cutoff, q, sr);
            let hl = self.hp2[0].tick(self.hp1[0].tick(l, g, k1).2, g, k2).2;
            let hr = self.hp2[1].tick(self.hp1[1].tick(r, g, k1).2, g, k2).2;
            let w = (t * 25.0).min(1.0);
            l += w * (hl - l);
            r += w * (hr - r);

            // ROLL : gate qui accélère, calé sur le tempo
            let roll_target = if roll > 0.0 && t > 0.0 {
                let beat = start_beats + i as f64 * beats_per_sample;
                let open = (beat * roll_steps(t)).rem_euclid(1.0) < 0.5;
                if open {
                    1.0
                } else {
                    1.0 - roll * (t * 1.3).min(1.0)
                }
            } else {
                1.0
            };
            self.roll_gain += gate_c * (roll_target - self.roll_gain);
            l *= self.roll_gain;
            r *= self.roll_gain;

            // SPACE : la reverb monte et sa queue s'allonge ; la stéréo s'ouvre avec elle
            if space > 0.0 && t > 0.0 {
                let wet = 0.6 * space * t;
                let fb = 0.72 + 0.21 * t;
                let s = t.sqrt();
                let (wl, wr) = self.reverb.tick(l * s, r * s, fb);
                l += wet * wl;
                r += wet * wr;
                let side_gain = 1.0 + 0.8 * space * t;
                let mid = 0.5 * (l + r);
                let side = 0.5 * (l - r) * side_gain;
                l = mid + side;
                r = mid - side;
            }

            // Limiteur de sécurité
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
            // À TENSION 0 : sortie strictement identique à l'entrée
            if raw == 0.0 && self.lim_env <= 0.95 {
                l = dry_l;
                r = dry_r;
            }
            left[i] = l.clamp(-0.99, 0.99);
            right[i] = r.clamp(-0.99, 0.99);
        }

        self.internal_beats = start_beats + n as f64 * beats_per_sample;
        self.state = RiseState { shaped: shape(self.smooth) };
    }
}
