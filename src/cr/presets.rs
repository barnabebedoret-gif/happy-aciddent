//! Presets d'usine du Happy Accident CALL & RESPONSE.
//! Les graines ont été choisies (examples/find_seeds.rs) pour donner exactement les patterns voulus ;
//! le test `presets_donnent_les_bons_patterns` vérifie qu'ils ne changent pas.

use super::engine::CrSettings;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Category {
    Basics,
    Pushed,
    Variations,
}

pub const CATEGORY_NAMES: [(Category, &str); 3] = [(Category::Basics, "BASICS"), (Category::Pushed, "PUSHED"), (Category::Variations, "WITH VARIATIONS")];

pub struct Preset {
    pub name: &'static str,
    pub tip: &'static str,
    pub category: Category,
    pub live: CrSettings,
    /// Variations 1 et 2 remplies par le preset (None = la case de l'utilisateur n'est pas touchée)
    pub var1: Option<CrSettings>,
    pub var2: Option<CrSettings>,
    /// Pattern attendu pour LIVE (vérifié par les tests)
    pub expect: &'static str,
}

// Index de TURN : 0 = 1/16, 1 = 1/8, 2 = 1/4, 3 = 1/2, 4 = 1 bar, 5 = 2 bars
// Index de STEPS : 0 = 4, 1 = 8, 2 = 16. MODE : 0 = Alternate, 1 = Random, 2 = Repeats.
const fn s(voices: u8, mode: u8, len_idx: u8, turn_idx: u8, silence: u8, seed: u32, mutations: u8) -> CrSettings {
    CrSettings { seed, mutations, voices, mode, len_idx, turn_idx, silence }
}

pub const PRESETS: &[Preset] = &[
    // ---------------------------------------------------------------- BASICS
    Preset {
        name: "Ping-Pong · 1 bar",
        tip: "The classic: A plays a bar, B answers a bar. Put two different leads (or a lead and a vocal) on A and B.",
        category: Category::Basics,
        live: s(2, 0, 0, 4, 0, 3, 0),
        var1: None,
        var2: None,
        expect: "ABAB",
    },
    Preset {
        name: "Question / Answer · 2 bars",
        tip: "Long phrases: 2 bars of A, then 2 bars of B. Great for melodic leads and breakdowns.",
        category: Category::Basics,
        live: s(2, 0, 0, 5, 0, 3, 0),
        var1: None,
        var2: None,
        expect: "ABAB",
    },
    Preset {
        name: "Half-Bar Trade",
        tip: "A and B swap every half bar. Tighter, more groove-oriented exchange.",
        category: Category::Basics,
        live: s(2, 0, 0, 3, 0, 3, 0),
        var1: None,
        var2: None,
        expect: "ABAB",
    },
    Preset {
        name: "Three Calls, One Answer",
        tip: "A plays 3 bars, B answers on the 4th. The most common phrasing in electronic music.",
        category: Category::Basics,
        live: s(2, 2, 0, 4, 0, 5, 0),
        var1: None,
        var2: None,
        expect: "AAAB",
    },
    Preset {
        name: "Answer Twice",
        tip: "A A A B, then A A B B over 4 bars (half-bar turns): the answer grows at the end of the phrase.",
        category: Category::Basics,
        live: s(2, 2, 1, 3, 0, 38, 0),
        var1: None,
        var2: None,
        expect: "AAABAABB",
    },
    Preset {
        name: "Call & Breathe",
        tip: "A, silence, B, silence. Space between the calls: perfect for intros and breakdowns.",
        category: Category::Basics,
        live: s(2, 0, 0, 4, 4, 397, 0),
        var1: None,
        var2: None,
        expect: "A.B.",
    },
    Preset {
        name: "Trio Rotation",
        tip: "Three tracks take turns every half bar (A B C). The 16-step loop shifts the phrasing over 8 bars.",
        category: Category::Basics,
        live: s(3, 0, 2, 3, 0, 11, 0),
        var1: None,
        var2: None,
        expect: "ABCABCABCABCABCA",
    },
    Preset {
        name: "Four-Way Round",
        tip: "Four tracks, one bar each: A B C D. Use it with 4 different samples or 4 layers of the same lead.",
        category: Category::Basics,
        live: s(4, 0, 0, 4, 0, 27, 0),
        var1: None,
        var2: None,
        expect: "ABCD",
    },
    // ---------------------------------------------------------------- PUSHED
    Preset {
        name: "Chatter 1/16",
        tip: "Two tracks fighting on every 1/16: it turns two leads into one new hybrid riff. Tip: set FADE to 1-2 ms for tight cuts.",
        category: Category::Pushed,
        live: s(2, 1, 2, 0, 0, 42, 0),
        var1: None,
        var2: None,
        expect: "ABABABABBABAABBA",
    },
    Preset {
        name: "Stutter Duel 1/8",
        tip: "Repeats mode on 1/8: each track grabs the mic for a few steps, like a stutter edit between two sounds.",
        category: Category::Pushed,
        live: s(2, 2, 2, 1, 0, 31337, 0),
        var1: None,
        var2: None,
        expect: "BBAAABAAAAAABBBA",
    },
    Preset {
        name: "Glitch Swarm",
        tip: "Four tracks, random 1/16 cuts with a few holes. Instant glitch texture: try it on 4 one-shots or 4 chopped vocals.",
        category: Category::Pushed,
        live: s(4, 1, 2, 0, 4, 1234, 0),
        var1: None,
        var2: None,
        expect: "ADADCDCBABCA.BC.",
    },
    Preset {
        name: "Ghost Notes",
        tip: "Three tracks on 1/8 with lots of silence: sparse, nervous fragments. Great with DEPTH around 80% so the others stay in the background.",
        category: Category::Pushed,
        live: s(3, 1, 2, 1, 10, 7, 0),
        var1: None,
        var2: None,
        expect: "CB.C...AB...B..A",
    },
    Preset {
        name: "Polyrhythm 3 over 16",
        tip: "A B C rotating on 1/16 over a 16-step loop: a 3-against-4 cross-rhythm that never lands where you expect.",
        category: Category::Pushed,
        live: s(3, 0, 2, 0, 0, 11, 0),
        var1: None,
        var2: None,
        expect: "ABCABCABCABCABCA",
    },
    Preset {
        name: "Solo & Interrupt",
        tip: "A holds the stage for almost 2 bars on 1/4 steps, B only interrupts at the end. Turns a second sound into punctuation.",
        category: Category::Pushed,
        live: s(2, 2, 2, 2, 0, 108, 0),
        var1: None,
        var2: None,
        expect: "AAAAAAABAAAAAABB",
    },
    // ---------------------------------------------------------------- WITH VARIATIONS
    Preset {
        name: "Build-Up Ramp",
        tip: "LIVE = 1-bar ping-pong, variation 1 = 1/4 trades, variation 2 = 1/16 chatter. Automate Variation Live → 1 → 2 to build tension before a drop. Replaces your variations 1 and 2.",
        category: Category::Variations,
        live: s(2, 0, 0, 4, 0, 3, 0),
        var1: Some(s(2, 0, 0, 2, 0, 3, 0)),
        var2: Some(s(2, 1, 2, 0, 0, 42, 0)),
        expect: "ABAB",
    },
    Preset {
        name: "Verse / Drop",
        tip: "LIVE = 2-bar question/answer between A and B (C and D stay silent), variation 1 = three calls one answer, variation 2 = all four tracks in a glitch swarm for the drop. Replaces your variations 1 and 2.",
        category: Category::Variations,
        live: s(2, 0, 0, 5, 0, 3, 0),
        var1: Some(s(2, 2, 0, 4, 0, 5, 0)),
        var2: Some(s(4, 1, 2, 0, 4, 1234, 0)),
        expect: "ABAB",
    },
    Preset {
        name: "Total Chaos",
        tip: "Everything pushed: 4 tracks, random 1/16 with silences and mutations. Variation 1 slows it to 1/8, variation 2 switches to 3 tracks in repeats mode. Hit MUTATE while it plays. Replaces your variations 1 and 2.",
        category: Category::Variations,
        live: s(4, 1, 2, 0, 8, 65535, 3),
        var1: Some(s(4, 1, 2, 1, 4, 2026, 0)),
        var2: Some(s(3, 2, 2, 0, 3, 99, 0)),
        expect: "",
    },
];
