//! Le "Patch" = un accident : quels modules sont actifs, dans quel ordre, et leurs réglages.
//! Tout est généré de façon déterministe à partir d'un seed.

use super::rng::Rng;
use serde::{Deserialize, Serialize};

pub const NUM_MODULES: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleKind {
    Gate,
    Stutter,
    Pitch,
    Drive,
    Filter,
    Space,
}

pub const KINDS: [ModuleKind; NUM_MODULES] = [
    ModuleKind::Gate,
    ModuleKind::Stutter,
    ModuleKind::Pitch,
    ModuleKind::Drive,
    ModuleKind::Filter,
    ModuleKind::Space,
];

impl ModuleKind {
    pub fn name(self) -> &'static str {
        match self {
            ModuleKind::Gate => "GATE",
            ModuleKind::Stutter => "STUTTER",
            ModuleKind::Pitch => "PITCH",
            ModuleKind::Drive => "DRIVE",
            ModuleKind::Filter => "FILTER",
            ModuleKind::Space => "SPACE",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Scale {
    #[default]
    Minor,
    Major,
    Dorian,
    Phrygian,
    HarmonicMinor,
    PentaMinor,
}

impl Scale {
    pub fn intervals(self) -> &'static [i32] {
        match self {
            Scale::Minor => &[0, 2, 3, 5, 7, 8, 10],
            Scale::Major => &[0, 2, 4, 5, 7, 9, 11],
            Scale::Dorian => &[0, 2, 3, 5, 7, 9, 10],
            Scale::Phrygian => &[0, 1, 3, 5, 7, 8, 10],
            Scale::HarmonicMinor => &[0, 2, 3, 5, 7, 8, 11],
            Scale::PentaMinor => &[0, 3, 5, 7, 10],
        }
    }

    /// Degré de gamme (peut être négatif / dépasser l'octave) -> demi-tons.
    pub fn degree_to_semis(self, degree: i32) -> i32 {
        let iv = self.intervals();
        let n = iv.len() as i32;
        let oct = degree.div_euclid(n);
        let idx = degree.rem_euclid(n) as usize;
        oct * 12 + iv[idx]
    }
}

/// Divisions rythmiques en temps (noires).
pub const DIVS: [f64; 4] = [0.5, 0.25, 0.125, 1.0 / 6.0];
pub const DIV_NAMES: [&str; 4] = ["1/8", "1/16", "1/32", "1/16T"];

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GateP {
    pub pattern: u16,
    pub division: u8,
    pub depth: f32,
    pub smooth: f32,
}

/// events: 0 = rien, 1 = stutter, 2 = reverse
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StutterP {
    pub events: [u8; 16],
    pub division: u8,
    pub repeats: u8,
    pub decay: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PitchP {
    /// Décalages en degrés de gamme
    pub steps: [i8; 8],
    pub patterned: bool,
    pub division: u8,
    pub blend: f32,
    pub grain: f32,
}

/// mode: 0 = saturation, 1 = wavefold, 2 = bitcrush
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DriveP {
    pub mode: u8,
    pub amount: f32,
    pub tone: f32,
}

/// Filtre rythmé (pas de wah : le cutoff saute d'un pas à l'autre).
/// mode : 0 = passe-bas, 1 = passe-bande, 2 = passe-haut, 3 = notch
/// motion : 0 = gate (ouvert / fermé selon un pattern), 1 = steps (un cutoff par pas),
///          2 = escalier (montée / descente par paliers sur 1/2/4 mesures)
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FilterP {
    pub mode: u8,
    pub slope24: bool,
    pub motion: u8,
    /// Niveau du filtre "ouvert" (0..1)
    pub cutoff: f32,
    /// Résonance bridée : jamais d'auto-oscillation
    pub reso: f32,
    /// Écart entre ouvert et fermé
    pub depth: f32,
    /// Pattern ouvert/fermé (mode gate)
    pub pattern: u16,
    /// Cutoff de chaque pas (mode steps, 0..15)
    pub steps: [u8; 16],
    pub division: u8,
    pub sweep_bars: u8,
    /// 0 = montée, 1 = descente, 2 = aller-retour
    pub sweep_dir: u8,
    /// Adoucissement des sauts (anti-clic, reste court)
    pub glide: f32,
    pub mix: f32,
}

/// mode: 0 = delay ping-pong, 1 = delay filtré (dub), 2 = chorus
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpaceP {
    pub mode: u8,
    pub time: u8,
    pub feedback: f32,
    pub mix: f32,
}

pub const LFO_BEATS: [f64; 5] = [0.25, 0.5, 1.0, 2.0, 4.0];

/// shape: 0 sinus, 1 triangle, 2 dent de scie, 3 sample & hold
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModLfo {
    pub rate: u8,
    pub shape: u8,
    pub depth: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Patch {
    pub seed: u32,
    pub enabled: [bool; NUM_MODULES],
    pub order: [u8; NUM_MODULES],
    pub gate: GateP,
    pub stutter: StutterP,
    pub pitch: PitchP,
    pub drive: DriveP,
    pub filter: FilterP,
    pub space: SpaceP,
    pub lfo: [ModLfo; NUM_MODULES],
}

impl Default for Patch {
    /// Point de départ neutre : un peu de drive + espace.
    fn default() -> Self {
        Patch {
            seed: 0,
            enabled: [false, false, false, false, true, true],
            order: [0, 1, 2, 3, 4, 5],
            gate: GateP { pattern: 0b1011_0110_1101_1011, division: 1, depth: 0.8, smooth: 0.3 },
            stutter: StutterP { events: [0; 16], division: 1, repeats: 2, decay: 0.9 },
            pitch: PitchP { steps: [0; 8], patterned: false, division: 0, blend: 0.5, grain: 0.5 },
            drive: DriveP { mode: 0, amount: 0.3, tone: 0.7 },
            filter: FilterP {
                mode: 0,
                slope24: true,
                motion: 0,
                cutoff: 0.55,
                reso: 0.35,
                depth: 0.6,
                pattern: 0b0110_1011_0101_1101,
                steps: [12, 4, 8, 2, 14, 6, 10, 3, 12, 5, 9, 1, 15, 7, 11, 4],
                division: 1,
                sweep_bars: 1,
                sweep_dir: 0,
                glide: 0.3,
                mix: 1.0,
            },
            space: SpaceP { mode: 0, time: 1, feedback: 0.35, mix: 0.25 },
            lfo: [ModLfo { rate: 2, shape: 0, depth: 0.5 }; NUM_MODULES],
        }
    }
}

/// Mélange "valeur actuelle" -> "valeur aléatoire" selon l'intensité.
struct Mutator {
    rng: Rng,
    a: f32,
}

impl Mutator {
    fn f(&mut self, cur: f32, lo: f32, hi: f32) -> f32 {
        let r = lo + (hi - lo) * self.rng.f32();
        (cur + (r - cur) * self.a).clamp(lo.min(hi), lo.max(hi))
    }
    fn reroll(&mut self) -> bool {
        self.rng.chance(self.a)
    }
    fn u8(&mut self, cur: u8, n: u32) -> u8 {
        if self.reroll() {
            self.rng.below(n) as u8
        } else {
            cur
        }
    }
}

const PATTERN_DEGREES: [i8; 12] = [-7, -5, -3, -2, 0, 0, 0, 2, 3, 4, 5, 7];
const STATIC_DEGREES: [i8; 8] = [-7, 7, -7, 4, -3, 2, 5, 14];

impl Patch {
    /// Génère un nouvel accident.
    /// - `amount` : 0 = rien ne bouge, 1 = complètement aléatoire
    /// - `balance` : 0 = timbre, 1 = pattern (rythme / mélodie)
    /// - `locks` : modules à ne pas toucher
    pub fn generate(prev: &Patch, seed: u32, amount: f32, balance: f32, locks: &[bool; NUM_MODULES]) -> Patch {
        let mut p = *prev;
        p.seed = seed;
        let mut m = Mutator { rng: Rng::new(seed as u64 * 0x2545_F491_4F6C_DD1D + 1), a: amount.clamp(0.0, 1.0) };
        let tp = balance.clamp(0.0, 1.0);

        // Probabilité d'activation de chaque module selon Timbre <-> Pattern
        let enable_p = [
            0.15 + 0.70 * tp, // Gate
            0.10 + 0.65 * tp, // Stutter
            0.45,             // Pitch
            0.70 - 0.45 * tp, // Drive
            0.90,             // Filter : quasi toujours là
            0.45,             // Space
        ];
        for i in 0..NUM_MODULES {
            if !locks[i] && m.reroll() {
                p.enabled[i] = m.rng.chance(enable_p[i]);
            }
        }
        if !p.enabled.iter().any(|&e| e) {
            // Au moins un module actif : on choisit selon la balance
            let i = if m.rng.chance(tp) { m.rng.pick(&[0usize, 1, 2]) } else { m.rng.pick(&[3usize, 4, 4]) };
            p.enabled[i] = true;
        }

        // Ordre de la chaîne
        for i in (1..NUM_MODULES).rev() {
            if m.reroll() {
                let j = m.rng.below(i as u32 + 1) as usize;
                p.order.swap(i, j);
            }
        }

        // --- Gate
        if !locks[0] {
            let g = &mut p.gate;
            let density = 0.75 - 0.3 * m.rng.f32();
            let mut bits = g.pattern;
            for b in 0..16 {
                if m.reroll() {
                    if m.rng.chance(density) {
                        bits |= 1 << b;
                    } else {
                        bits &= !(1 << b);
                    }
                }
            }
            bits |= 1; // le premier step joue toujours
            g.pattern = bits;
            g.division = m.u8(g.division, 4);
            g.depth = m.f(g.depth, 0.5, 1.0);
            g.smooth = m.f(g.smooth, 0.0, 1.0);
        }

        // --- Stutter / reverse
        if !locks[1] {
            let s = &mut p.stutter;
            let p_event = 0.12 + 0.30 * tp;
            for e in s.events.iter_mut() {
                if m.reroll() {
                    *e = if m.rng.chance(p_event) {
                        if m.rng.chance(0.65) { 1 } else { 2 }
                    } else {
                        0
                    };
                }
            }
            if s.events.iter().all(|&e| e == 0) {
                let k = m.rng.below(16) as usize;
                s.events[k] = 1;
            }
            s.division = m.u8(s.division, 2);
            if m.reroll() {
                s.repeats = m.rng.pick(&[2u8, 3, 4, 6]);
            }
            s.decay = m.f(s.decay, 0.6, 1.0);
        }

        // --- Pitch
        if !locks[2] {
            let pt = &mut p.pitch;
            if m.reroll() {
                pt.patterned = m.rng.chance(0.2 + 0.7 * tp);
            }
            for (k, st) in pt.steps.iter_mut().enumerate() {
                if m.reroll() {
                    *st = if pt.patterned {
                        if k == 0 { 0 } else { m.rng.pick(&PATTERN_DEGREES) }
                    } else {
                        m.rng.pick(&STATIC_DEGREES)
                    };
                }
            }
            if !pt.patterned && pt.steps[0] == 0 {
                pt.steps[0] = -7;
            }
            pt.division = m.u8(pt.division, 2);
            pt.blend = m.f(pt.blend, 0.35, 1.0);
            pt.grain = m.f(pt.grain, 0.0, 1.0);
        }

        // --- Drive
        if !locks[3] {
            let d = &mut p.drive;
            d.mode = m.u8(d.mode, 3);
            d.amount = m.f(d.amount, 0.15, 1.0);
            d.tone = m.f(d.tone, 0.2, 1.0);
        }

        // --- Filter
        if !locks[4] {
            let f = &mut p.filter;
            if m.reroll() {
                f.mode = m.rng.pick(&[0u8, 0, 0, 1, 1, 2, 2, 3]); // passe-bas favorisé
            }
            if m.reroll() {
                f.slope24 = m.rng.chance(0.6);
            }
            if m.reroll() {
                let r = m.rng.f32();
                let p_steps = 0.25 + 0.15 * tp;
                f.motion = if r < p_steps {
                    1
                } else if r < p_steps + 0.15 {
                    2
                } else {
                    0
                };
            }
            f.cutoff = m.f(f.cutoff, 0.4, 0.95);
            f.reso = m.f(f.reso, 0.05, 0.7);
            f.depth = m.f(f.depth, 0.3, 1.0);
            let density = 0.4 + 0.35 * m.rng.f32();
            for b in 1..16 {
                if m.reroll() {
                    if m.rng.chance(density) {
                        f.pattern |= 1 << b;
                    } else {
                        f.pattern &= !(1 << b);
                    }
                }
            }
            f.pattern |= 1;
            for st in f.steps.iter_mut() {
                if m.reroll() {
                    *st = m.rng.below(16) as u8;
                }
            }
            f.division = m.u8(f.division, 4);
            f.sweep_bars = m.u8(f.sweep_bars, 3);
            f.sweep_dir = m.u8(f.sweep_dir, 3);
            f.glide = m.f(f.glide, 0.0, 1.0);
            f.mix = m.f(f.mix, 0.85, 1.0);
        }

        // --- Space
        if !locks[5] {
            let s = &mut p.space;
            s.mode = m.u8(s.mode, 3);
            s.time = m.u8(s.time, 4);
            s.feedback = m.f(s.feedback, 0.15, 0.75);
            s.mix = m.f(s.mix, 0.15, 0.6);
        }

        // --- LFO de mouvement
        for i in 0..NUM_MODULES {
            if locks[i] {
                continue;
            }
            let l = &mut p.lfo[i];
            l.rate = m.u8(l.rate, LFO_BEATS.len() as u32);
            l.shape = m.u8(l.shape, 4);
            l.depth = m.f(l.depth, 0.0, 1.0);
        }

        p
    }

    /// Courte description d'un module pour l'interface.
    pub fn describe(&self, kind: ModuleKind) -> String {
        match kind {
            ModuleKind::Gate => {
                let steps = self.gate.pattern.count_ones();
                format!("{} · {}/16 steps", DIV_NAMES[self.gate.division as usize % 4], steps)
            }
            ModuleKind::Stutter => {
                let st = self.stutter.events.iter().filter(|&&e| e == 1).count();
                let rv = self.stutter.events.iter().filter(|&&e| e == 2).count();
                format!("{} · {}× rep · {}× rev", DIV_NAMES[self.stutter.division as usize % 4], st, rv)
            }
            ModuleKind::Pitch => {
                if self.pitch.patterned {
                    format!("Mélodie 8 pas · {}", DIV_NAMES[self.pitch.division as usize % 4])
                } else {
                    format!("{:+} degrés", self.pitch.steps[0])
                }
            }
            ModuleKind::Drive => {
                let mode = ["Saturation", "Wavefold", "Bitcrush"][self.drive.mode as usize % 3];
                format!("{} {:.0}%", mode, self.drive.amount * 100.0)
            }
            ModuleKind::Filter => {
                let f = &self.filter;
                let mode = ["LP", "BP", "HP", "Notch"][f.mode as usize % 4];
                let slope = if f.slope24 { "24" } else { "12" };
                let div = DIV_NAMES[f.division as usize % 4];
                let motion = match f.motion {
                    1 => format!("Steps {}", div),
                    2 => format!("Escalier {} mes. {}", [1, 2, 4][f.sweep_bars as usize % 3], ["montée", "descente", "aller-retour"][f.sweep_dir as usize % 3]),
                    _ => format!("Gate {} · {}/16", div, f.pattern.count_ones()),
                };
                format!("{}{} · {}", mode, slope, motion)
            }
            ModuleKind::Space => {
                let mode = ["Ping-pong", "Dub filtré", "Chorus"][self.space.mode as usize % 3];
                format!("{} · {:.0}%", mode, self.space.mix * 100.0)
            }
        }
    }
}
