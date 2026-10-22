//! Moteur du Happy Accident CALL & RESPONSE.
//!
//! Chaque instance a une lettre (A, B, C, D). Toutes les instances d'un même groupe partagent
//! les mêmes réglages (graine + paramètres de pattern), stockés dans un seul entier atomique.
//! Le pattern est recalculé de façon déterministe à partir de ces réglages : comme toutes les
//! instances lisent la même position de lecture d'Ableton, elles savent toutes qui a la parole.

use crate::engine::rng::Rng;
use crate::engine::Transport;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};

pub const NUM_GROUPS: usize = 4;
pub const LETTERS: [&str; 4] = ["A", "B", "C", "D"];
pub const TURN_BEATS: [f64; 6] = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0];
pub const TURN_NAMES: [&str; 6] = ["1/16", "1/8", "1/4", "1/2", "1 bar", "2 bars"];
pub const LENS: [usize; 3] = [4, 8, 16];
pub const MODE_NAMES: [&str; 3] = ["Alternate", "Random", "Repeats"];
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

/// Applique des réglages au groupe depuis ACCIDENT, MUTATE ou les réglages.
/// On revient toujours sur LIVE : une variation sauvegardée n'est jamais modifiée par erreur.
pub fn store_group(group: usize, s: &CrSettings) {
    let g = group % NUM_GROUPS;
    let v = s.pack();
    LIVE[g].store(v, Ordering::Release);
    ACTIVE[g].store(VAR_LIVE, Ordering::Release);
    GROUPS[g].store(v, Ordering::Release);
}

// ---------------------------------------------------------------- VARIATIONS
/// LIVE = le pattern de travail ; 1 et 2 = deux patterns sauvegardés, à rappeler en un clic
/// ou avec une seule automation (paramètre Variation).
pub const NUM_SLOTS: usize = 2;
pub const VAR_LIVE: u8 = 0;
pub static LIVE: [AtomicU64; NUM_GROUPS] = [const { AtomicU64::new(0) }; NUM_GROUPS];
pub static SLOTS: [[AtomicU64; NUM_SLOTS]; NUM_GROUPS] = [const { [const { AtomicU64::new(0) }; NUM_SLOTS] }; NUM_GROUPS];
/// 0 = LIVE, 1..=2 = variation
pub static ACTIVE: [AtomicU8; NUM_GROUPS] = [const { AtomicU8::new(0) }; NUM_GROUPS];

pub fn active_variation(group: usize) -> u8 {
    ACTIVE[group % NUM_GROUPS].load(Ordering::Acquire).min(NUM_SLOTS as u8)
}

pub fn slot(group: usize, k: usize) -> Option<CrSettings> {
    CrSettings::unpack(SLOTS[group % NUM_GROUPS][k % NUM_SLOTS].load(Ordering::Acquire))
}

/// Charge un preset d'usine pour tout le groupe : LIVE est remplacé, et les variations 1 et 2
/// seulement si le preset en définit.
pub fn apply_preset(group: usize, p: &super::presets::Preset) {
    let g = group % NUM_GROUPS;
    for (k, v) in [p.var1, p.var2].iter().enumerate() {
        if let Some(v) = v {
            SLOTS[g][k].store(v.pack(), Ordering::Release);
        }
    }
    store_group(g, &p.live);
}

/// Sauvegarde ce qu'on entend en ce moment dans la variation `k` (0 ou 1).
pub fn save_slot(group: usize, k: usize) {
    let g = group % NUM_GROUPS;
    let cur = GROUPS[g].load(Ordering::Acquire);
    if cur != 0 {
        SLOTS[g][k % NUM_SLOTS].store(cur, Ordering::Release);
    }
}

/// Bascule tout le groupe : 0 = LIVE, 1 ou 2 = variation. Une variation vide est ignorée.
/// Retourne vrai si la bascule a eu lieu.
pub fn select_variation(group: usize, v: u8) -> bool {
    let g = group % NUM_GROUPS;
    let target = if v == VAR_LIVE {
        let live = LIVE[g].load(Ordering::Acquire);
        if live == 0 { GROUPS[g].load(Ordering::Acquire) } else { live }
    } else {
        SLOTS[g][(v as usize - 1) % NUM_SLOTS].load(Ordering::Acquire)
    };
    if target == 0 {
        return false;
    }
    // On garde une trace du pattern de travail avant de le quitter
    if active_variation(g) == VAR_LIVE {
        let cur = GROUPS[g].load(Ordering::Acquire);
        if cur != 0 {
            LIVE[g].store(cur, Ordering::Release);
        }
    }
    ACTIVE[g].store(v.min(NUM_SLOTS as u8), Ordering::Release);
    GROUPS[g].store(target, Ordering::Release);
    true
}

/// État à sauvegarder avec le projet : [LIVE, variation 1, variation 2, actif].
pub fn snapshot_vars(group: usize) -> [u64; 4] {
    let g = group % NUM_GROUPS;
    let live = if active_variation(g) == VAR_LIVE { GROUPS[g].load(Ordering::Acquire) } else { LIVE[g].load(Ordering::Acquire) };
    [live, SLOTS[g][0].load(Ordering::Acquire), SLOTS[g][1].load(Ordering::Acquire), active_variation(g) as u64]
}

/// Restaure l'état sauvegardé si le groupe n'a encore rien en mémoire.
pub fn init_vars(group: usize, saved: &[u64; 4]) {
    let g = group % NUM_GROUPS;
    let empty = LIVE[g].load(Ordering::Acquire) == 0 && SLOTS[g].iter().all(|v| v.load(Ordering::Acquire) == 0);
    if !empty || saved[..3].iter().all(|&v| v == 0) {
        return;
    }
    LIVE[g].store(saved[0], Ordering::Release);
    SLOTS[g][0].store(saved[1], Ordering::Release);
    SLOTS[g][1].store(saved[2], Ordering::Release);
    let a = (saved[3] as u8).min(NUM_SLOTS as u8);
    let playing = if a == VAR_LIVE { saved[0] } else { saved[a as usize] };
    if playing != 0 {
        ACTIVE[g].store(a, Ordering::Release);
        GROUPS[g].store(playing, Ordering::Release);
    }
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
