//! Moteur du Happy Accident CALL & RESPONSE.
//!
//! Chaque instance a une lettre (A, B, C, D). Toutes les instances d'un même groupe partagent
//! les mêmes réglages (graine + paramètres de pattern), stockés dans un seul entier atomique.
//! Le pattern est recalculé de façon déterministe à partir de ces réglages : comme toutes les
//! instances lisent la même position de lecture d'Ableton, elles savent toutes qui a la parole.

use crate::engine::rng::Rng;
use crate::engine::Transport;
use std::sync::atomic::{AtomicU64, Ordering};

pub const NUM_GROUPS: usize = 4;
pub const LETTERS: [&str; 4] = ["A", "B", "C", "D"];
pub const TURN_BEATS: [f64; 6] = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0];
pub const TURN_NAMES: [&str; 6] = ["1/16", "1/8", "1/4", "1/2", "1 mes.", "2 mes."];
pub const LENS: [usize; 3] = [4, 8, 16];
pub const MODE_NAMES: [&str; 3] = ["Alterné", "Aléatoire", "Répétitions"];
pub const SILENT: u8 = 255;

/// Réglages partagés par groupe (0 = jamais initialisé)
pub static GROUPS: [AtomicU64; NUM_GROUPS] = [const { AtomicU64::new(0) }; NUM_GROUPS];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrSettings {
    /// 24 bits
    pub seed: u32,
    pub mutations: u8,
    /// 2..=4
    pub voices: u8,
    pub mode: u8,
    pub len_idx: u8,
    pub turn_idx: u8,
    /// 0..=15
    pub silence: u8,
}

impl Default for CrSettings {
    fn default() -> Self {
        CrSettings { seed: 0x00B2_7B, mutations: 0, voices: 2, mode: 0, len_idx: 1, turn_idx: 4, silence: 0 }
    }
}

const SET_FLAG: u64 = 1 << 63;

impl CrSettings {
    pub fn pack(&self) -> u64 {
        (self.seed as u64 & 0xFF_FFFF)
            | (self.mutations as u64) << 24
            | ((self.voices.clamp(2, 4) - 2) as u64) << 32
            | ((self.mode % 3) as u64) << 34
            | ((self.len_idx % 3) as u64) << 36
            | ((self.turn_idx % 6) as u64) << 38
            | ((self.silence & 15) as u64) << 41
            | SET_FLAG
    }

    pub fn unpack(v: u64) -> Option<CrSettings> {
        if v & SET_FLAG == 0 {
            return None;
        }
        Some(CrSettings {
            seed: (v & 0xFF_FFFF) as u32,
            mutations: (v >> 24) as u8,
            voices: ((v >> 32) & 3) as u8 + 2,
            mode: (((v >> 34) & 3) as u8).min(2),
            len_idx: (((v >> 36) & 3) as u8).min(2),
            turn_idx: (((v >> 38) & 7) as u8).min(5),
            silence: ((v >> 41) & 15) as u8,
        })
    }

    pub fn len(&self) -> usize {
        LENS[self.len_idx as usize % 3]
    }

    pub fn turn_beats(&self) -> f64 {
        TURN_BEATS[self.turn_idx as usize % 6]
    }

    /// Le pattern : pour chaque case, l'index de la voix qui parle (ou SILENT).
    pub fn pattern(&self) -> [u8; 16] {
        let len = self.len();
        let v = self.voices.clamp(2, 4) as u32;
        let mut rng = Rng::new(self.seed as u64 ^ 0xC411_0000);
        // Ordre des voix tiré au sort (pour que "Alterné" ne commence pas toujours par A)
        let mut order = [0u8, 1, 2, 3];
        for i in (1..v as usize).rev() {
            let j = rng.below(i as u32 + 1) as usize;
            order.swap(i, j);
        }
        let mut p = [SILENT; 16];
        for i in 0..len {
            p[i] = match self.mode {
                0 => order[i % v as usize],
                1 => {
                    let mut x = rng.below(v) as u8;
                    if i > 0 && x == p[i - 1] && rng.chance(0.7) {
                        x = ((x as u32 + 1 + rng.below(v - 1)) % v) as u8;
                    }
                    x
                }
                _ => {
                    if i > 0 && rng.chance(0.5) {
                        p[i - 1]
                    } else {
                        rng.below(v) as u8
                    }
                }
            };
        }
        // Silences (jamais sur la première case)
        let p_sil = self.silence as f32 * 0.03;
        for slot in p.iter_mut().take(len).skip(1) {
            if rng.chance(p_sil) {
                *slot = SILENT;
            }
        }
        // Mutations : chaque MUTATE modifie une case
        for j in 0..self.mutations as u64 {
            let mut r = Rng::new(self.seed as u64 * 31 + j * 0x9E37 + 1);
            let idx = r.below(len as u32) as usize;
            p[idx] = if idx > 0 && self.silence > 0 && r.chance(0.25) { SILENT } else { r.below(v) as u8 };
        }
        // Chaque voix doit parler au moins une fois
        for voice in 0..v as u8 {
            if p[..len].contains(&voice) {
                continue;
            }
            let start = rng.below(len as u32) as usize;
            for k in 0..len {
                let idx = (start + k) % len;
                let cur = p[idx];
                let removable = cur == SILENT || p[..len].iter().filter(|&&x| x == cur).count() > 1;
                if removable {
                    p[idx] = voice;
                    break;
                }
            }
        }
        p
    }

    /// Nouvel accident. Chaos règle si la structure (tour, longueur, mode, silences) change aussi.
    pub fn accident(&self, new_seed: u32, chaos: f32) -> CrSettings {
        let mut s = *self;
        let mut rng = Rng::new(new_seed as u64 ^ 0xACC1);
        s.seed = new_seed & 0xFF_FFFF;
        s.mutations = 0;
        let c = chaos.clamp(0.0, 1.0);
        if rng.chance(c) {
            s.turn_idx = rng.pick(&[1u8, 2, 2, 3, 3, 4, 4, 5]);
        }
        if rng.chance(c * 0.7) {
            s.len_idx = rng.pick(&[0u8, 1, 1, 2]);
        }
        if rng.chance(c * 0.6) {
            s.mode = rng.below(3) as u8;
        }
        if rng.chance(c * 0.5) {
            s.silence = rng.pick(&[0u8, 0, 2, 4, 6]);
        }
        s
    }

    pub fn mutate(&self) -> CrSettings {
        let mut s = *self;
        s.mutations = s.mutations.saturating_add(1);
        s
    }
}

pub fn load_group(group: usize) -> Option<CrSettings> {
    CrSettings::unpack(GROUPS[group % NUM_GROUPS].load(Ordering::Acquire))
}

pub fn store_group(group: usize, s: &CrSettings) {
    GROUPS[group % NUM_GROUPS].store(s.pack(), Ordering::Release);
}

/// Initialise le groupe avec ces réglages s'il est encore vide ; retourne les réglages en vigueur.
pub fn init_group(group: usize, s: &CrSettings) -> CrSettings {
    let g = &GROUPS[group % NUM_GROUPS];
    match g.compare_exchange(0, s.pack(), Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) => *s,
        Err(cur) => CrSettings::unpack(cur).unwrap_or(*s),
    }
}

/// Partie audio : ouvre ou ferme la piste selon la lettre active.
pub struct CrGate {
    sr: f32,
    beat: f64,
    gain: f32,
    cached: u64,
    pattern: [u8; 16],
    /// Case en cours (pour l'affichage)
    pub slot: usize,
}

impl CrGate {
    pub fn new(sr: f32) -> Self {
        CrGate { sr: sr.max(8000.0), beat: 0.0, gain: 1.0, cached: 0, pattern: [0; 16], slot: 0 }
    }

    pub fn reset(&mut self) {
        self.gain = 1.0;
    }

    /// `letter` : 0..3 ; `depth` : 1 = les autres lettres sont coupées, 0.5 = juste baissées ;
    /// `fade_ms` : durée des fondus entre les tours.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], s: &CrSettings, letter: u8, depth: f32, fade_ms: f32, t: &Transport) {
        let packed = s.pack();
        if packed != self.cached {
            self.pattern = s.pattern();
            self.cached = packed;
        }
        let tempo = if t.tempo.is_finite() && t.tempo > 1.0 { t.tempo.clamp(20.0, 999.0) } else { 120.0 };
        let spb = 60.0 / tempo * self.sr as f64;
        if t.playing {
            if let Some(pos) = t.pos_beats {
                if pos.is_finite() {
                    self.beat = pos;
                }
            }
        }
        let len = s.len() as f64;
        let turn = s.turn_beats();
        let low = 1.0 - depth.clamp(0.0, 1.0);
        let coef = 1.0 - (-1.0 / (fade_ms.max(0.5) * 0.001 * self.sr)).exp();
        for i in 0..left.len().min(right.len()) {
            let slot = (self.beat / turn).floor().rem_euclid(len) as usize;
            self.slot = slot;
            let target = if self.pattern[slot] == letter { 1.0 } else { low };
            self.gain += coef * (target - self.gain);
            let l = if left[i].is_finite() { left[i] } else { 0.0 };
            let r = if right[i].is_finite() { right[i] } else { 0.0 };
            left[i] = l * self.gain;
            right[i] = r * self.gain;
            self.beat += 1.0 / spb;
        }
    }
}
