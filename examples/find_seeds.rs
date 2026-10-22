//! Outil interne : trouve les graines qui donnent un pattern précis (pour les presets d'usine).
use happy_accident::cr::engine::{CrSettings, LETTERS, SILENT};

fn show(s: &CrSettings) -> String {
    s.pattern()[..s.len()].iter().map(|&v| if v == SILENT { "." } else { LETTERS[v as usize] }).collect::<Vec<_>>().join("")
}

fn find(voices: u8, mode: u8, len_idx: u8, silence: u8, target: &str) {
    for seed in 1..2_000_000u32 {
        let s = CrSettings { seed, mutations: 0, voices, mode, len_idx, turn_idx: 4, silence };
        if show(&s) == target {
            println!("{target}: seed {seed} (voices {voices} mode {mode} len {len_idx} sil {silence})");
            return;
        }
    }
    println!("{target}: introuvable");
}

fn sample(voices: u8, mode: u8, len_idx: u8, silence: u8, n: usize) {
    print!("v{voices} m{mode} l{len_idx} s{silence}:");
    for seed in [7u32, 42, 99, 1234, 2026, 31337, 65535, 424242].iter().take(n) {
        let s = CrSettings { seed: *seed, mutations: 0, voices, mode, len_idx, turn_idx: 4, silence };
        print!("  {}={}", seed, show(&s));
    }
    println!();
}

fn main() {
    // Patterns précis
    find(2, 0, 0, 0, "ABAB");
    find(2, 2, 0, 0, "AAAB");
    find(2, 2, 1, 0, "AAABAAAB");
    find(2, 2, 1, 0, "AAABAABB");
    find(2, 0, 0, 4, "A.B.");
    find(2, 2, 0, 4, "A.B.");
    find(3, 0, 0, 0, "ABCA");
    find(3, 0, 2, 0, "ABCABCABCABCABCA");
    find(4, 0, 0, 0, "ABCD");
    find(4, 0, 2, 0, "ABCDABCDABCDABCD");
    find(2, 2, 2, 0, "AAAAAAABAAAAAABB");
    find(3, 2, 1, 0, "AABBCCAB");
    // Aperçus pour les modes aléatoires
    sample(2, 1, 2, 0, 8);
    sample(4, 1, 2, 4, 8);
    sample(2, 2, 2, 0, 8);
    sample(3, 1, 2, 10, 8);
    sample(4, 1, 2, 8, 8);
    sample(3, 2, 2, 3, 8);
}
