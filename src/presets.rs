//! Presets d'usine du Happy Accident Generator : un patch complet (effets allumés, ordre,
//! réglages) + les macros + les LOCK conseillés.

use crate::engine::patch::{ModLfo, Patch};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Category {
    Basics,
    Pushed,
}

pub const CATEGORY_NAMES: [(Category, &str); 2] = [(Category::Basics, "BASICS"), (Category::Pushed, "PUSHED")];

pub struct GenPreset {
    pub name: &'static str,
    pub tip: &'static str,
    pub category: Category,
    pub chaos: f32,
    /// 0 = timbre, 1 = pattern
    pub balance: f32,
    pub movement: f32,
    /// 0 = 1/4, 1 = 1/2, 2 = 1×, 3 = 2×, 4 = 4×
    pub speed: u8,
    /// Modules verrouillés : GATE, STUTTER, PITCH, DRIVE, FILTER, SPACE
    pub locks: [bool; 6],
    pub build: fn() -> Patch,
}

// Index des modules
const GATE: usize = 0;
const STUTTER: usize = 1;
const PITCH: usize = 2;
const DRIVE: usize = 3;
const FILTER: usize = 4;
const SPACE: usize = 5;

/// "x" = pas joué, "." = pas coupé (le premier caractère est le premier pas)
fn bits(s: &str) -> u16 {
    s.bytes().take(16).enumerate().fold(0u16, |acc, (i, c)| if c == b'x' { acc | 1 << i } else { acc })
}

/// "s" = stutter, "r" = reverse, "." = rien
fn events(s: &str) -> [u8; 16] {
    let mut e = [0u8; 16];
    for (i, c) in s.bytes().take(16).enumerate() {
        e[i] = match c {
            b's' => 1,
            b'r' => 2,
            _ => 0,
        };
    }
    e
}

/// Point de départ : tout éteint, ordre donné
fn base(on: &[usize], order: [u8; 6]) -> Patch {
    let mut p = Patch::default();
    p.seed = 0xB27B_0000;
    p.enabled = [false; 6];
    for &i in on {
        p.enabled[i] = true;
    }
    p.order = order;
    p.lfo = [ModLfo { rate: 2, shape: 0, depth: 0.4 }; 6];
    p
}

// ---------------------------------------------------------------- BASICS
fn trance_gate() -> Patch {
    let mut p = base(&[GATE, SPACE], [0, 5, 1, 2, 3, 4]);
    p.gate.pattern = bits("x.xx.xx.x.xx.xx.");
    p.gate.division = 1;
    p.gate.depth = 1.0;
    p.gate.smooth = 0.25;
    p.space.mode = 0;
    p.space.time = 2;
    p.space.feedback = 0.3;
    p.space.mix = 0.2;
    p
}

fn filter_gate() -> Patch {
    let mut p = base(&[FILTER], [4, 0, 1, 2, 3, 5]);
    let f = &mut p.filter;
    f.mode = 0;
    f.slope24 = true;
    f.motion = 0;
    f.pattern = bits("x..xx.x.x..xx.xx");
    f.division = 1;
    f.cutoff = 0.85;
    f.depth = 0.8;
    f.reso = 0.45;
    f.glide = 0.2;
    f.mix = 1.0;
    p
}

fn octave_grit() -> Patch {
    let mut p = base(&[PITCH, DRIVE], [2, 3, 0, 1, 4, 5]);
    p.pitch.steps = [-7, 0, 0, 0, 0, 0, 0, 0];
    p.pitch.patterned = false;
    p.pitch.blend = 0.5;
    p.pitch.grain = 0.4;
    p.drive.mode = 0;
    p.drive.amount = 0.45;
    p.drive.tone = 0.6;
    p
}

fn stutter_fills() -> Patch {
    let mut p = base(&[STUTTER], [1, 0, 2, 3, 4, 5]);
    p.stutter.events = events("............s.ss");
    p.stutter.division = 0;
    p.stutter.repeats = 3;
    p.stutter.decay = 0.85;
    p
}

fn lofi_crush() -> Patch {
    let mut p = base(&[DRIVE, FILTER], [3, 4, 0, 1, 2, 5]);
    p.drive.mode = 2;
    p.drive.amount = 0.55;
    p.drive.tone = 0.55;
    let f = &mut p.filter;
    f.mode = 0;
    f.slope24 = false;
    f.motion = 0;
    f.pattern = 0xFFFF;
    f.cutoff = 0.55;
    f.reso = 0.2;
    f.mix = 1.0;
    p
}

fn wide_chorus() -> Patch {
    let mut p = base(&[DRIVE, SPACE], [3, 5, 0, 1, 2, 4]);
    p.drive.mode = 0;
    p.drive.amount = 0.15;
    p.drive.tone = 0.8;
    p.space.mode = 2;
    p.space.mix = 0.55;
    p
}

fn dub_echo() -> Patch {
    let mut p = base(&[FILTER, SPACE], [4, 5, 0, 1, 2, 3]);
    let f = &mut p.filter;
    f.mode = 2;
    f.slope24 = false;
    f.motion = 0;
    f.pattern = 0xFFFF;
    f.cutoff = 0.9;
    f.reso = 0.15;
    f.mix = 1.0;
    p.space.mode = 1;
    p.space.time = 3;
    p.space.feedback = 0.6;
    p.space.mix = 0.45;
    p
}

// ---------------------------------------------------------------- PUSHED
fn acid_steps() -> Patch {
    let mut p = base(&[DRIVE, FILTER, SPACE], [3, 4, 5, 0, 1, 2]);
    p.drive.mode = 0;
    p.drive.amount = 0.7;
    p.drive.tone = 0.8;
    let f = &mut p.filter;
    f.mode = 0;
    f.slope24 = true;
    f.motion = 1;
    f.steps = [12, 3, 9, 15, 2, 7, 13, 4, 10, 1, 14, 6, 11, 3, 15, 8];
    f.division = 1;
    f.cutoff = 0.6;
    f.reso = 0.7;
    f.depth = 1.0;
    f.glide = 0.35;
    f.mix = 1.0;
    p.space.mode = 0;
    p.space.time = 1;
    p.space.feedback = 0.35;
    p.space.mix = 0.2;
    p
}

fn melody_machine() -> Patch {
    let mut p = base(&[PITCH, GATE, SPACE], [2, 0, 5, 1, 3, 4]);
    p.pitch.steps = [0, 2, 4, 7, 4, 2, -3, 0];
    p.pitch.patterned = true;
    p.pitch.division = 0;
    p.pitch.blend = 1.0;
    p.pitch.grain = 0.3;
    p.gate.pattern = bits("xxx.xxx.xxx.xx.x");
    p.gate.division = 1;
    p.gate.depth = 0.9;
    p.gate.smooth = 0.2;
    p.space.mode = 0;
    p.space.time = 2;
    p.space.feedback = 0.35;
    p.space.mix = 0.25;
    p
}

fn reverse_rave() -> Patch {
    let mut p = base(&[STUTTER, FILTER, SPACE], [1, 4, 5, 0, 2, 3]);
    p.stutter.events = events("...r...r..r.r..r");
    p.stutter.division = 0;
    p.stutter.repeats = 2;
    p.stutter.decay = 0.9;
    let f = &mut p.filter;
    f.mode = 2;
    f.slope24 = true;
    f.motion = 0;
    f.pattern = bits("xxxx..xxxx..xx.x");
    f.division = 1;
    f.cutoff = 0.85;
    f.depth = 0.7;
    f.reso = 0.35;
    f.mix = 1.0;
    p.space.mode = 2;
    p.space.mix = 0.4;
    p
}

fn glitch_machine() -> Patch {
    let mut p = base(&[STUTTER, GATE, DRIVE], [1, 0, 3, 2, 4, 5]);
    p.stutter.events = events("s..s.rs...s.rs.s");
    p.stutter.division = 1;
    p.stutter.repeats = 6;
    p.stutter.decay = 0.95;
    p.gate.pattern = bits("xx.x.xxx.x.xx.xx");
    p.gate.division = 2;
    p.gate.depth = 0.8;
    p.gate.smooth = 0.1;
    p.drive.mode = 2;
    p.drive.amount = 0.4;
    p.drive.tone = 0.7;
    p
}

fn stair_riser() -> Patch {
    let mut p = base(&[FILTER, SPACE], [4, 5, 0, 1, 2, 3]);
    let f = &mut p.filter;
    f.mode = 2;
    f.slope24 = true;
    f.motion = 2;
    f.sweep_bars = 2;
    f.sweep_dir = 0;
    f.division = 1;
    f.reso = 0.5;
    f.mix = 1.0;
    p.space.mode = 0;
    p.space.time = 1;
    p.space.feedback = 0.5;
    p.space.mix = 0.3;
    p
}

fn wavefold_monster() -> Patch {
    let mut p = base(&[PITCH, DRIVE, FILTER], [2, 3, 4, 0, 1, 5]);
    p.pitch.steps = [-7, 0, 0, 0, 0, 0, 0, 0];
    p.pitch.patterned = false;
    p.pitch.blend = 0.4;
    p.pitch.grain = 0.5;
    p.drive.mode = 1;
    p.drive.amount = 0.85;
    p.drive.tone = 0.7;
    let f = &mut p.filter;
    f.mode = 3;
    f.slope24 = false;
    f.motion = 1;
    f.steps = [3, 10, 6, 14, 1, 9, 12, 5, 8, 15, 2, 11, 7, 13, 4, 0];
    f.division = 1;
    f.reso = 0.3;
    f.glide = 0.2;
    f.mix = 1.0;
    p
}

fn half_speed_haze() -> Patch {
    let mut p = base(&[PITCH, FILTER, SPACE], [2, 4, 5, 0, 1, 3]);
    p.pitch.steps = [7, 0, 0, 0, 0, 0, 0, 0];
    p.pitch.patterned = false;
    p.pitch.blend = 0.3;
    p.pitch.grain = 0.8;
    let f = &mut p.filter;
    f.mode = 0;
    f.slope24 = false;
    f.motion = 0;
    f.pattern = bits("x.x.xx.x.xx.x.x.");
    f.division = 0;
    f.cutoff = 0.75;
    f.depth = 0.6;
    f.reso = 0.3;
    f.glide = 0.6;
    f.mix = 1.0;
    p.space.mode = 2;
    p.space.mix = 0.5;
    p
}

fn everything() -> Patch {
    let mut p = base(&[GATE, STUTTER, PITCH, DRIVE, FILTER, SPACE], [3, 4, 0, 1, 2, 5]);
    p.drive.mode = 0;
    p.drive.amount = 0.5;
    p.drive.tone = 0.7;
    let f = &mut p.filter;
    f.mode = 0;
    f.slope24 = true;
    f.motion = 1;
    f.steps = [10, 4, 13, 7, 15, 2, 9, 12, 5, 14, 3, 8, 11, 6, 15, 1];
    f.division = 1;
    f.reso = 0.55;
    f.glide = 0.25;
    f.mix = 1.0;
    p.gate.pattern = bits("xx.xxx.xx.xxx.x.");
    p.gate.division = 1;
    p.gate.depth = 0.85;
    p.gate.smooth = 0.2;
    p.stutter.events = events("......s....r..ss");
    p.stutter.division = 1;
    p.stutter.repeats = 4;
    p.stutter.decay = 0.9;
    p.pitch.steps = [0, 0, 7, 0, 3, 0, 5, -2];
    p.pitch.patterned = true;
    p.pitch.division = 0;
    p.pitch.blend = 0.6;
    p.pitch.grain = 0.4;
    p.space.mode = 0;
    p.space.time = 2;
    p.space.feedback = 0.4;
    p.space.mix = 0.25;
    p
}

const NO_LOCK: [bool; 6] = [false; 6];
const fn lock(i: usize) -> [bool; 6] {
    let mut l = [false; 6];
    l[i] = true;
    l
}

pub const PRESETS: &[GenPreset] = &[
    GenPreset {
        name: "Trance Gate 1/16",
        tip: "Classic 1/16 trance gate with a light dotted echo. The gate is locked: ACCIDENT only changes the rest.",
        category: Category::Basics,
        chaos: 0.3,
        balance: 0.7,
        movement: 0.2,
        speed: 2,
        locks: lock(GATE),
        build: trance_gate,
    },
    GenPreset {
        name: "Filter Gate",
        tip: "A 24 dB low-pass that opens and closes on a 1/16 pattern. Great on pads and long leads. The filter is locked.",
        category: Category::Basics,
        chaos: 0.4,
        balance: 0.3,
        movement: 0.2,
        speed: 2,
        locks: lock(FILTER),
        build: filter_gate,
    },
    GenPreset {
        name: "Octave Down Grit",
        tip: "Adds an octave below plus warm saturation: instant weight for a thin lead.",
        category: Category::Basics,
        chaos: 0.3,
        balance: 0.2,
        movement: 0.1,
        speed: 2,
        locks: NO_LOCK,
        build: octave_grit,
    },
    GenPreset {
        name: "Stutter Fills",
        tip: "Plays your lead straight, then stutters the end of every 2 bars: automatic fills.",
        category: Category::Basics,
        chaos: 0.4,
        balance: 0.8,
        movement: 0.0,
        speed: 2,
        locks: NO_LOCK,
        build: stutter_fills,
    },
    GenPreset {
        name: "Lo-Fi Crush",
        tip: "Bitcrush into a soft low-pass: dusty, retro tone.",
        category: Category::Basics,
        chaos: 0.3,
        balance: 0.1,
        movement: 0.1,
        speed: 2,
        locks: NO_LOCK,
        build: lofi_crush,
    },
    GenPreset {
        name: "Wide Chorus",
        tip: "A touch of saturation and a wide chorus: makes a mono lead big and stereo.",
        category: Category::Basics,
        chaos: 0.2,
        balance: 0.1,
        movement: 0.2,
        speed: 2,
        locks: NO_LOCK,
        build: wide_chorus,
    },
    GenPreset {
        name: "Dub Echo",
        tip: "Thin high-pass into a filtered dub delay on the beat. Space without mud.",
        category: Category::Basics,
        chaos: 0.3,
        balance: 0.3,
        movement: 0.3,
        speed: 2,
        locks: NO_LOCK,
        build: dub_echo,
    },
    GenPreset {
        name: "Acid Steps",
        tip: "Driven sound into a resonant 24 dB filter that jumps on every 1/16. Turns any lead into an acid line. Push MOVEMENT for more jumps.",
        category: Category::Pushed,
        chaos: 0.5,
        balance: 0.3,
        movement: 0.5,
        speed: 2,
        locks: lock(FILTER),
        build: acid_steps,
    },
    GenPreset {
        name: "Melody Machine",
        tip: "Rewrites a long sustained note into an 8-step melody in your KEY and SCALE, gated on 1/16. Set the key first! Pitch is locked so ACCIDENT keeps the melody.",
        category: Category::Pushed,
        chaos: 0.5,
        balance: 0.9,
        movement: 0.2,
        speed: 2,
        locks: lock(PITCH),
        build: melody_machine,
    },
    GenPreset {
        name: "Reverse Rave",
        tip: "Reversed fragments thrown into the groove, a gated high-pass and a wide chorus. Rave energy from any lead.",
        category: Category::Pushed,
        chaos: 0.6,
        balance: 0.8,
        movement: 0.3,
        speed: 2,
        locks: NO_LOCK,
        build: reverse_rave,
    },
    GenPreset {
        name: "Glitch Machine",
        tip: "Stutters, reverses, a 1/32 gate and bitcrush, at double speed. Hit MUTATE again and again.",
        category: Category::Pushed,
        chaos: 0.9,
        balance: 0.9,
        movement: 0.6,
        speed: 3,
        locks: NO_LOCK,
        build: glitch_machine,
    },
    GenPreset {
        name: "Stair Riser",
        tip: "A high-pass that climbs in steps over 4 bars. Start it 4 bars before a drop for an instant build-up.",
        category: Category::Pushed,
        chaos: 0.3,
        balance: 0.5,
        movement: 0.0,
        speed: 2,
        locks: lock(FILTER),
        build: stair_riser,
    },
    GenPreset {
        name: "Wavefold Monster",
        tip: "Octave down, heavy wavefolding and a notch filter jumping on 1/16. Aggressive, for hard techno.",
        category: Category::Pushed,
        chaos: 0.7,
        balance: 0.3,
        movement: 0.7,
        speed: 2,
        locks: NO_LOCK,
        build: wavefold_monster,
    },
    GenPreset {
        name: "Half-Speed Haze",
        tip: "Everything at half speed: an octave-up shimmer layer, a slow filter gate and a wide chorus. Dreamy breakdown tool.",
        category: Category::Pushed,
        chaos: 0.4,
        balance: 0.4,
        movement: 0.3,
        speed: 1,
        locks: NO_LOCK,
        build: half_speed_haze,
    },
    GenPreset {
        name: "Everything At Once",
        tip: "All six effects on together: drive, stepping filter, gate, stutter, pitch melody and echo. The deep end: MUTATE it and resample what you like.",
        category: Category::Pushed,
        chaos: 1.0,
        balance: 0.5,
        movement: 0.8,
        speed: 2,
        locks: NO_LOCK,
        build: everything,
    },
];
