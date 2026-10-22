//! Moteur audio du Happy Accident Generator, indépendant du format plugin
//! (utilisable tel quel dans les tests et le rendu de démos).

pub mod demo;
pub mod dsp;
pub mod modules;
pub mod patch;
pub mod rng;

use dsp::{sanitize, DcBlock};
use modules::*;
pub use patch::{ModuleKind, Patch, Scale, KINDS, NUM_MODULES};
use rng::Rng;
use serde::{Deserialize, Serialize};

/// Les réglages de surface (macros) lus à chaque bloc.
#[derive(Clone, Copy, Debug)]
pub struct Macros {
    pub chaos: f32,
    /// 0 = timbre, 1 = pattern
    pub balance: f32,
    pub movement: f32,
    pub mix: f32,
    pub out_gain: f32,
    pub key: u8,
    pub scale: Scale,
    /// Vitesse des rythmes (0.25 = 4× plus lent, 4 = 4× plus rapide)
    pub speed: f32,
    pub locks: [bool; NUM_MODULES],
}

impl Default for Macros {
    fn default() -> Self {
        Macros { chaos: 0.6, balance: 0.5, movement: 0.4, mix: 1.0, out_gain: 1.0, key: 9, scale: Scale::Minor, speed: 1.0, locks: [false; NUM_MODULES] }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Transport {
    pub tempo: f64,
    pub playing: bool,
    pub pos_beats: Option<f64>,
}

impl Default for Transport {
    fn default() -> Self {
        Transport { tempo: 128.0, playing: false, pos_beats: None }
    }
}

pub const HISTORY_LEN: usize = 16;

/// Historique des accidents (taille fixe : pas d'allocation dans le thread audio).
/// Générique pour être partagé entre les plugins (Generator, Dub…).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: Deserialize<'de>"))]
pub struct Hist<T> {
    pub patches: [T; HISTORY_LEN],
    pub len: u8,
    pub pos: u8,
}

pub type History = Hist<Patch>;

impl<T: Copy + Default> Default for Hist<T> {
    fn default() -> Self {
        Hist { patches: [T::default(); HISTORY_LEN], len: 1, pos: 0 }
    }
}

impl<T: Copy> Hist<T> {
    pub fn current(&self) -> T {
        self.patches[self.pos as usize]
    }

    pub fn is_valid(&self) -> bool {
        self.len > 0 && self.pos < self.len && self.len as usize <= HISTORY_LEN
    }

    pub fn push(&mut self, p: T) {
        // On coupe le "futur" si on était revenu en arrière
        let mut len = self.pos as usize + 1;
        if len == HISTORY_LEN {
            self.patches.copy_within(1.., 0);
            len -= 1;
        }
        self.patches[len] = p;
        self.len = (len + 1) as u8;
        self.pos = len as u8;
    }

    pub fn set_current(&mut self, p: T) {
        self.patches[self.pos as usize] = p;
    }

    pub fn back(&mut self) -> bool {
        if self.pos > 0 {
            self.pos -= 1;
            true
        } else {
            false
        }
    }

    pub fn forward(&mut self) -> bool {
        if self.pos + 1 < self.len {
            self.pos += 1;
            true
        } else {
            false
        }
    }
}

pub struct Engine {
    sr: f32,
    history: History,
    patch: Patch,
    pending: Option<Patch>,
    fade: f32,
    rng: Rng,
    beat: f64,

    gate: Gate,
    stutter: Stutter,
    pitch: Pitch,
    drive: Drive,
    filter: Filter,
    space: Space,

    pw_dry: f32,
    pw_wet: f32,
    agc: f32,
    dc_l: DcBlock,
    dc_r: DcBlock,
    lim_env: f32,
}

impl Engine {
    pub fn new(sample_rate: f32, seed: u64) -> Self {
        let sr = sample_rate.max(8000.0);
        let mut e = Engine {
            sr,
            history: History::default(),
            patch: Patch::default(),
            pending: None,
            fade: 1.0,
            rng: Rng::new(seed),
            beat: 0.0,
            gate: Gate::default(),
            stutter: Stutter::new(sr),
            pitch: Pitch::new(sr),
            drive: Drive::default(),
            filter: Filter::default(),
            space: Space::new(sr),
            pw_dry: 0.0,
            pw_wet: 0.0,
            agc: 1.0,
            dc_l: DcBlock::default(),
            dc_r: DcBlock::default(),
            lim_env: 0.0,
        };
        e.reset_modules();
        e
    }

    pub fn patch(&self) -> &Patch {
        self.pending.as_ref().unwrap_or(&self.patch)
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    /// Restaure un historique sauvegardé (ex: rechargement du projet Ableton).
    pub fn load_history(&mut self, h: History) {
        if h.len == 0 || h.pos >= h.len || h.len as usize > HISTORY_LEN {
            return;
        }
        self.history = h;
        self.patch = h.current();
        self.pending = None;
        self.fade = 1.0;
        self.rng = Rng::new(self.patch.seed as u64 ^ 0xB27B);
        self.reset_modules();
    }

    fn new_seed(&mut self) -> u32 {
        (self.rng.next_u64() >> 32) as u32
    }

    /// ACCIDENT : nouveau tirage, plus ou moins radical selon Chaos.
    pub fn accident(&mut self, m: &Macros) {
        let amount = 0.15 + 0.85 * m.chaos.clamp(0.0, 1.0);
        self.generate(m, amount);
    }

    /// MUTATE : petite dérive autour de l'accident actuel.
    pub fn mutate(&mut self, m: &Macros) {
        let amount = 0.05 + 0.25 * m.chaos.clamp(0.0, 1.0);
        self.generate(m, amount);
    }

    fn generate(&mut self, m: &Macros, amount: f32) {
        let seed = self.new_seed();
        let base = *self.patch();
        let p = Patch::generate(&base, seed, amount, m.balance, &m.locks);
        self.history.push(p);
        self.switch_to(p);
    }

    pub fn history_prev(&mut self) {
        if self.history.pos > 0 {
            self.history.pos -= 1;
            self.switch_to(self.history.current());
        }
    }

    /// Allume ou éteint un module dans l'accident en cours (sans créer de nouvel accident).
    /// Charge un patch complet (preset d'usine) comme nouvel accident.
    pub fn load_patch(&mut self, p: Patch) {
        self.history.push(p);
        self.switch_to(p);
    }

    pub fn toggle_module(&mut self, i: usize) {
        if i >= NUM_MODULES {
            return;
        }
        let mut p = *self.patch();
        p.enabled[i] = !p.enabled[i];
        self.history.set_current(p);
        self.switch_to(p);
    }

    pub fn history_next(&mut self) {
        if self.history.pos + 1 < self.history.len {
            self.history.pos += 1;
            self.switch_to(self.history.current());
        }
    }

    fn switch_to(&mut self, p: Patch) {
        self.pending = Some(p);
    }

    fn reset_modules(&mut self) {
        self.gate.reset();
        self.stutter.reset();
        self.pitch.reset();
        self.drive.reset();
        self.filter.reset();
        self.space.reset();
    }

    /// Traitement d'un bloc stéréo, sur place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], m: &Macros, t: &Transport) {
        let tempo = if t.tempo.is_finite() && t.tempo > 1.0 { t.tempo.clamp(20.0, 999.0) } else { 120.0 };
        let spb = 60.0 / tempo * self.sr as f64;
        if t.playing {
            if let Some(pos) = t.pos_beats {
                if pos.is_finite() {
                    self.beat = pos;
                }
            }
        }

        let fade_step = 1.0 / (0.008 * self.sr);
        let pw_c = 1.0 - (-1.0 / (0.3 * self.sr)).exp();
        let agc_c = 1.0 - (-1.0 / (0.5 * self.sr)).exp();
        let lim_rel = 1.0 - (-1.0 / (0.1 * self.sr)).exp();
        let mix = m.mix.clamp(0.0, 1.0);

        for i in 0..left.len().min(right.len()) {
            let dl = sanitize(left[i]);
            let dr = sanitize(right[i]);

            // Fondu lors d'un changement d'accident (évite les clics)
            if self.pending.is_some() {
                self.fade -= fade_step;
                if self.fade <= 0.0 {
                    self.fade = 0.0;
                    self.patch = self.pending.take().unwrap();
                    self.reset_modules();
                }
            } else if self.fade < 1.0 {
                self.fade = (self.fade + fade_step).min(1.0);
            }

            // Horloge accélérée / ralentie pour les modules rythmiques ; Space reste au vrai tempo
            let speed = m.speed.clamp(0.0625, 16.0) as f64;
            let c = Ctx { sr: self.sr, beat: self.beat * speed, spb: spb / speed, key: m.key as i32, scale: m.scale, seed: self.patch.seed as u64 };
            let c_space = Ctx { beat: self.beat, spb, ..c };
            let p = self.patch;
            let (mut x, mut y) = (dl, dr);
            for &mi in p.order.iter() {
                let mi = mi as usize % NUM_MODULES;
                if !p.enabled[mi] {
                    continue;
                }
                let mv = lfo_value(&p.lfo[mi], c.beat, c.seed, mi as u64) * p.lfo[mi].depth * m.movement;
                (x, y) = match KINDS[mi] {
                    ModuleKind::Gate => self.gate.process(x, y, &p.gate, &c, mv),
                    ModuleKind::Stutter => self.stutter.process(x, y, &p.stutter, &c),
                    ModuleKind::Pitch => self.pitch.process(x, y, &p.pitch, &c, mv),
                    ModuleKind::Drive => self.drive.process(x, y, &p.drive, &c, mv),
                    ModuleKind::Filter => self.filter.process(x, y, &p.filter, &c, m.movement),
                    ModuleKind::Space => self.space.process(x, y, &p.space, &c_space, mv),
                };
            }
            let x = self.dc_l.process(x);
            let y = self.dc_r.process(y);

            // Pendant un changement : on retombe sur le son sec, pas sur du silence
            let wl = dl + (x - dl) * self.fade;
            let wr = dr + (y - dr) * self.fade;

            // Compensation de volume : le wet garde à peu près le niveau du dry
            self.pw_dry += pw_c * ((dl * dl + dr * dr) - self.pw_dry);
            self.pw_wet += pw_c * ((wl * wl + wr * wr) - self.pw_wet);
            if self.pw_dry > 1e-7 {
                let target = ((self.pw_dry + 1e-9) / (self.pw_wet + 1e-9)).sqrt().clamp(0.25, 2.0);
                self.agc += agc_c * (target - self.agc);
            }

            let mut ol = (dl * (1.0 - mix) + wl * self.agc * mix) * m.out_gain;
            let mut or = (dr * (1.0 - mix) + wr * self.agc * mix) * m.out_gain;

            // Limiteur de sécurité (plafond ~ -0.9 dBFS)
            let peak = ol.abs().max(or.abs());
            if peak > self.lim_env {
                self.lim_env = peak;
            } else {
                self.lim_env += lim_rel * (peak - self.lim_env);
            }
            let ceiling = 0.9;
            if self.lim_env > ceiling {
                let g = ceiling / self.lim_env;
                ol *= g;
                or *= g;
            }
            ol = ol.clamp(-0.99, 0.99);
            or = or.clamp(-0.99, 0.99);

            if !(ol.is_finite() && or.is_finite() && self.agc.is_finite() && self.lim_env.is_finite()) {
                self.reset_modules();
                self.agc = 1.0;
                self.lim_env = 0.0;
                self.pw_dry = 0.0;
                self.pw_wet = 0.0;
                ol = 0.0;
                or = 0.0;
            }

            left[i] = ol;
            right[i] = or;
            self.beat += 1.0 / spb;
        }
    }
}
