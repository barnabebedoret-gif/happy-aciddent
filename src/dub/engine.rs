//! Moteur du Happy Accident DUB : delay synchronisé avec filtre rythmé dans la boucle
//! de feedback, envois "throw" en pattern, saturation et wobble de bande.

use crate::engine::dsp::{sanitize, DcBlock, DelayLine, Svf};
use crate::engine::patch::Mutator;
use crate::engine::rng::Rng;
use crate::engine::{Hist, Transport};
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

/// Temps de delay possibles, en temps (noires)
pub const DUB_TIMES: [f64; 7] = [0.25, 0.375, 0.5, 0.75, 1.0 / 3.0, 1.0, 1.5];
pub const DUB_TIME_NAMES: [&str; 7] = ["1/16", "1/16D", "1/8", "1/8D", "1/8T", "1/4", "1/4D"];

/// Groupes verrouillables
pub const DUB_GROUPS: [&str; 4] = ["TIME", "SEND", "FILTER", "COLOR"];

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DubPatch {
    pub seed: u32,
    // TIME
    pub time_l: u8,
    pub time_r: u8,
    pub pingpong: bool,
    // SEND : quels pas envoient le signal dans le delay
    pub send_all: bool,
    pub send_pattern: u16,
    pub send_div: u8,
    // FILTER (dans la boucle) : 0 = bande dub (HP + LP), 1 = passe-bande, 2 = passe-haut, 3 = passe-bas
    pub filter_mode: u8,
    /// 0 = fixe, 1 = un cutoff différent à chaque écho, 2 = escalier sur 2 mesures
    pub filter_motion: u8,
    pub cutoff: f32,
    pub reso: f32,
    pub cut_steps: [u8; 8],
    // COLOR
    pub drive: f32,
    pub wobble: f32,
    pub feedback_bias: f32,
}

impl Default for DubPatch {
    fn default() -> Self {
        DubPatch {
            seed: 0,
            time_l: 3,
            time_r: 3,
            pingpong: true,
            send_all: true,
            send_pattern: 0b0100_0100_0100_0100,
            send_div: 0,
            filter_mode: 0,
            filter_motion: 1,
            cutoff: 0.5,
            reso: 0.35,
            cut_steps: [10, 6, 12, 4, 9, 14, 5, 8],
            drive: 0.3,
            wobble: 0.3,
            feedback_bias: 0.0,
        }
    }
}

impl DubPatch {
    pub fn generate(prev: &DubPatch, seed: u32, amount: f32, locks: &[bool; 4]) -> DubPatch {
        let mut p = *prev;
        p.seed = seed;
        let mut m = Mutator { rng: Rng::new(seed as u64 * 0x9E37_79B9 + 7), a: amount.clamp(0.0, 1.0) };
        // Temps "dub" favorisés : 1/8D, 1/4, 1/8, 1/4D
        const TIMES: [u8; 12] = [3, 3, 3, 5, 5, 2, 2, 6, 1, 4, 0, 3];

        if !locks[0] {
            if m.reroll() {
                p.time_l = m.rng.pick(&TIMES);
            }
            if m.reroll() {
                p.pingpong = m.rng.chance(0.55);
            }
            if m.reroll() {
                p.time_r = if m.rng.chance(0.6) { p.time_l } else { m.rng.pick(&TIMES) };
            }
        }
        if !locks[1] {
            if m.reroll() {
                p.send_all = m.rng.chance(0.35);
            }
            let density = 0.12 + 0.3 * m.rng.f32();
            for b in 0..16 {
                if m.reroll() {
                    if m.rng.chance(density) {
                        p.send_pattern |= 1 << b;
                    } else {
                        p.send_pattern &= !(1 << b);
                    }
                }
            }
            if p.send_pattern == 0 {
                p.send_pattern = 1 << (m.rng.below(16));
            }
            p.send_div = m.u8(p.send_div, 2);
        }
        if !locks[2] {
            if m.reroll() {
                p.filter_mode = m.rng.pick(&[0u8, 0, 0, 0, 1, 1, 2, 2, 3, 3]);
            }
            if m.reroll() {
                p.filter_motion = m.rng.pick(&[0u8, 1, 1, 2]);
            }
            p.cutoff = m.f(p.cutoff, 0.2, 0.8);
            p.reso = m.f(p.reso, 0.1, 0.65);
            for s in p.cut_steps.iter_mut() {
                if m.reroll() {
                    *s = m.rng.below(16) as u8;
                }
            }
        }
        if !locks[3] {
            p.drive = m.f(p.drive, 0.0, 1.0);
            p.wobble = m.f(p.wobble, 0.0, 1.0);
            p.feedback_bias = m.f(p.feedback_bias, -0.1, 0.12);
        }
        p
    }

    pub fn describe(&self, group: usize) -> String {
        match group {
            0 => {
                let l = DUB_TIME_NAMES[self.time_l as usize % 7];
                let r = DUB_TIME_NAMES[self.time_r as usize % 7];
                let pp = if self.pingpong { " · ping-pong" } else { "" };
                if l == r {
                    format!("{}{}", l, pp)
                } else {
                    format!("L {} / R {}{}", l, r, pp)
                }
            }
            1 => {
                if self.send_all {
                    "Envoi continu".to_string()
                } else {
                    format!("Throws {} · {}/16", ["1/8", "1/16"][self.send_div as usize % 2], self.send_pattern.count_ones())
                }
            }
            2 => {
                let mode = ["Bande dub", "BP", "HP", "LP"][self.filter_mode as usize % 4];
                let motion = ["fixe", "par écho", "escalier"][self.filter_motion as usize % 3];
                format!("{} · {}", mode, motion)
            }
            _ => format!("Drive {:.0}% · Wobble {:.0}%", self.drive * 100.0, self.wobble * 100.0),
        }
    }
}

pub type DubHistory = Hist<DubPatch>;

#[derive(Clone, Copy, Debug)]
pub struct DubMacros {
    pub chaos: f32,
    pub feedback: f32,
    /// Niveau des échos
    pub mix: f32,
    pub movement: f32,
    pub out_gain: f32,
    pub throw: bool,
    pub locks: [bool; 4],
}

impl Default for DubMacros {
    fn default() -> Self {
        DubMacros { chaos: 0.6, feedback: 0.75, mix: 0.6, movement: 0.5, out_gain: 1.0, throw: false, locks: [false; 4] }
    }
}

pub struct DubEngine {
    sr: f32,
    history: DubHistory,
    patch: DubPatch,
    rng: Rng,
    beat: f64,
    dl: DelayLine,
    dr: DelayLine,
    d_l: f32,
    d_r: f32,
    hp: [Svf; 2],
    lp: [Svf; 2],
    cut: f32,
    cut_init: bool,
    send_env: f32,
    fb: f32,
    wob_ph: f32,
    dc: [DcBlock; 2],
    lim_env: f32,
}

impl DubEngine {
    pub fn new(sample_rate: f32, seed: u64) -> Self {
        let sr = sample_rate.max(8000.0);
        let n = (sr * 6.0) as usize;
        DubEngine {
            sr,
            history: DubHistory::default(),
            patch: DubPatch::default(),
            rng: Rng::new(seed),
            beat: 0.0,
            dl: DelayLine::new(n),
            dr: DelayLine::new(n),
            d_l: 0.0,
            d_r: 0.0,
            hp: [Svf::default(); 2],
            lp: [Svf::default(); 2],
            cut: 0.0,
            cut_init: false,
            send_env: 0.0,
            fb: 0.0,
            wob_ph: 0.0,
            dc: [DcBlock::default(); 2],
            lim_env: 0.0,
        }
    }

    pub fn patch(&self) -> &DubPatch {
        &self.patch
    }

    pub fn history(&self) -> &DubHistory {
        &self.history
    }

    pub fn load_history(&mut self, h: DubHistory) {
        if h.is_valid() {
            self.history = h;
            self.patch = h.current();
            self.rng = Rng::new(self.patch.seed as u64 ^ 0xD0B);
        }
    }

    pub fn accident(&mut self, m: &DubMacros) {
        self.generate(m, 0.15 + 0.85 * m.chaos.clamp(0.0, 1.0));
    }

    pub fn mutate(&mut self, m: &DubMacros) {
        self.generate(m, 0.05 + 0.25 * m.chaos.clamp(0.0, 1.0));
    }

    fn generate(&mut self, m: &DubMacros, amount: f32) {
        let seed = (self.rng.next_u64() >> 32) as u32;
        let p = DubPatch::generate(&self.patch, seed, amount, &m.locks);
        self.history.push(p);
        self.patch = p;
    }

    pub fn history_prev(&mut self) {
        if self.history.back() {
            self.patch = self.history.current();
        }
    }

    pub fn history_next(&mut self) {
        if self.history.forward() {
            self.patch = self.history.current();
        }
    }

    /// Vide les échos (utile au rechargement)
    pub fn clear(&mut self) {
        self.dl.clear();
        self.dr.clear();
        self.hp = [Svf::default(); 2];
        self.lp = [Svf::default(); 2];
    }

    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], m: &DubMacros, t: &Transport) {
        let tempo = if t.tempo.is_finite() && t.tempo > 1.0 { t.tempo.clamp(20.0, 999.0) } else { 120.0 };
        let spb = 60.0 / tempo * self.sr as f64;
        if t.playing {
            if let Some(pos) = t.pos_beats {
                if pos.is_finite() {
                    self.beat = pos;
                }
            }
        }
        let p = self.patch;
        let sr = self.sr;
        let max_d = (self.dl.capacity() - 8) as f32;
        let t_l = DUB_TIMES[p.time_l as usize % 7];
        let t_r = DUB_TIMES[p.time_r as usize % 7];
        let target_l = ((t_l * spb) as f32).min(max_d);
        let target_r = ((t_r * spb) as f32).min(max_d);
        if self.d_l <= 0.0 {
            self.d_l = target_l;
            self.d_r = target_r;
        }
        // Glissement façon bande quand le temps change (effet "accident" voulu)
        let glide = 1.0 - (-1.0 / (0.15 * sr)).exp();
        let env_c = 1.0 - (-1.0 / (0.003 * sr)).exp();
        let fb_c = 1.0 - (-1.0 / (0.05 * sr)).exp();
        let cut_c = 1.0 - (-1.0 / (0.002 * sr)).exp();
        let lim_rel = 1.0 - (-1.0 / (0.1 * sr)).exp();
        let fb_target = if m.throw { 1.08 } else { (m.feedback + p.feedback_bias).clamp(0.0, 0.97) };
        let drive_g = 1.0 + p.drive * 4.0;
        let wob_depth = p.wobble * (0.4 + 0.6 * m.movement) * 0.0035 * sr;
        let (lo, hi): (f32, f32) = match p.filter_mode {
            0 => (250.0, 3500.0),
            1 => (300.0, 4500.0),
            2 => (80.0, 2500.0),
            _ => (500.0, 9000.0),
        };
        let range = (hi / lo).log2();
        let q = 0.6 + p.reso * p.reso * 4.0;
        // En THROW on retire la compensation : la boucle dépasse 1 et part en spirale (bornée par la saturation)
        let reso_comp = if m.throw || p.filter_mode == 1 { 1.0 } else { 1.0 / q.max(1.0).powf(0.25) };
        let mix = m.mix.clamp(0.0, 1.0);

        for i in 0..left.len().min(right.len()) {
            let xl = sanitize(left[i]);
            let xr = sanitize(right[i]);

            // ---- Envoi (throws)
            let send_on = p.send_all || {
                let div = if p.send_div == 0 { 0.5 } else { 0.25 };
                let step = (self.beat / div).floor().rem_euclid(16.0) as u32;
                (p.send_pattern >> step) & 1 == 1
            };
            self.send_env += env_c * ((send_on as u8 as f32) - self.send_env);
            let in_l = xl * self.send_env;
            let in_r = xr * self.send_env;

            // ---- Temps de delay + wobble
            self.d_l += glide * (target_l - self.d_l);
            self.d_r += glide * (target_r - self.d_r);
            self.wob_ph = (self.wob_ph + 0.55 / sr).fract();
            let wob = (2.0 * PI * self.wob_ph).sin() * wob_depth;
            let el = self.dl.read((self.d_l + wob).clamp(1.0, max_d) - 1.0);
            let er = self.dr.read((self.d_r - wob).clamp(1.0, max_d) - 1.0);

            // ---- Filtre rythmé (sauts, jamais de wah)
            let c01 = match p.filter_motion {
                1 => {
                    let k = (self.beat / t_l).floor().rem_euclid(8.0) as usize;
                    let st = p.cut_steps[k] as f32 / 15.0;
                    (p.cutoff + (st - 0.5) * (0.4 + 0.6 * m.movement)).clamp(0.0, 1.0)
                }
                2 => {
                    let k = self.beat.floor().rem_euclid(8.0) as f32 / 7.0;
                    let k = if p.cut_steps[0] % 2 == 0 { k } else { 1.0 - k };
                    0.1 + 0.8 * k
                }
                _ => p.cutoff,
            };
            let target = lo.log2() + range * c01;
            if !self.cut_init {
                self.cut = target;
                self.cut_init = true;
            }
            self.cut += cut_c * (target - self.cut);
            let fc = 2f32.powf(self.cut);

            let mut echo = [el, er];
            match p.filter_mode {
                0 => {
                    // Bande dub : passe-haut sous le centre, passe-bas au-dessus
                    let (gh, kh) = Svf::coefs(fc * 0.3, 0.7, sr);
                    let (gl, kl) = Svf::coefs(fc * 2.5, q, sr);
                    for ch in 0..2 {
                        let h = self.hp[ch].tick(echo[ch], gh, kh).2;
                        echo[ch] = self.lp[ch].tick(h, gl, kl).0;
                    }
                }
                mode => {
                    let (g, k) = Svf::coefs(fc, q, sr);
                    for ch in 0..2 {
                        let (lpo, bpo, hpo) = self.lp[ch].tick(echo[ch], g, k);
                        echo[ch] = match mode {
                            1 => bpo * k, // gain unité au centre
                            2 => hpo,
                            _ => lpo,
                        };
                    }
                }
            }
            // Saturation dans la boucle (gain unité à bas niveau) + compensation de la résonance :
            // le feedback ne dépasse jamais 1, sauf en THROW, où la saturation le borne
            for e in echo.iter_mut() {
                *e = (*e * drive_g * reso_comp).tanh() / drive_g;
            }

            // ---- Feedback
            self.fb += fb_c * (fb_target - self.fb);
            let fb = self.fb;
            if p.pingpong {
                let mono = (in_l + in_r) * 0.5;
                self.dl.write(sanitize(mono + fb * echo[1]));
                self.dr.write(sanitize(fb * echo[0]));
            } else {
                self.dl.write(sanitize(in_l + fb * echo[0]));
                self.dr.write(sanitize(in_r + fb * echo[1]));
            }

            // ---- Sortie : son sec intact + échos
            let wl = self.dc[0].process(echo[0]);
            let wr = self.dc[1].process(echo[1]);
            let mut ol = (xl + wl * mix * 2.0) * m.out_gain;
            let mut or = (xr + wr * mix * 2.0) * m.out_gain;

            let peak = ol.abs().max(or.abs());
            if peak > self.lim_env {
                self.lim_env = peak;
            } else {
                self.lim_env += lim_rel * (peak - self.lim_env);
            }
            if self.lim_env > 0.9 {
                let g = 0.9 / self.lim_env;
                ol *= g;
                or *= g;
            }
            ol = ol.clamp(-0.99, 0.99);
            or = or.clamp(-0.99, 0.99);
            if !(ol.is_finite() && or.is_finite() && self.lim_env.is_finite() && self.fb.is_finite()) {
                self.clear();
                self.lim_env = 0.0;
                self.fb = 0.0;
                ol = 0.0;
                or = 0.0;
            }
            left[i] = ol;
            right[i] = or;
            self.beat += 1.0 / spb;
        }
    }
}
