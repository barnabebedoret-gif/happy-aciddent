//! Démo du Happy Accident CALL & RESPONSE : 3 pistes (A stabs, B arpège, C lead tenu).
//! cargo run --release --example render_cr

use happy_accident::cr::engine::{CrGate, CrSettings, LETTERS, SILENT, TURN_NAMES, MODE_NAMES};
use happy_accident::engine::{demo::{render_lead, render_stabs, render_sustain_lead}, Transport};

fn main() {
    let sr = 44100.0;
    let tempo = 124.0;
    let bars = 4;
    let src = [render_stabs(sr, tempo, bars), render_lead(sr, tempo, bars), render_sustain_lead(sr, tempo, bars)];
    let gains = [1.4f32, 0.8, 1.0];
    let n = src[0].0.len();

    std::fs::create_dir_all("demos").unwrap();
    let spec = hound::WavSpec { channels: 2, sample_rate: sr as u32, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create("demos/happy_accident_cr_demo.wav", spec).unwrap();
    let mut write = |l: &[f32], r: &[f32]| {
        for (a, b) in l.iter().zip(r) {
            w.write_sample(((a * 0.8).clamp(-1.0, 1.0) * 32767.0) as i16).unwrap();
            w.write_sample(((b * 0.8).clamp(-1.0, 1.0) * 32767.0) as i16).unwrap();
        }
        for _ in 0..(sr as usize) {
            w.write_sample(0i16).unwrap();
            w.write_sample(0i16).unwrap();
        }
    };

    // 0. Les 3 pistes en même temps, sans le plugin : le bazar
    println!("0. Les 3 pistes ensemble, sans le plugin");
    let mut ml = vec![0.0f32; n];
    let mut mr = vec![0.0f32; n];
    for (k, (l, r)) in src.iter().enumerate() {
        for i in 0..n {
            ml[i] += l[i] * gains[k];
            mr[i] += r[i] * gains[k];
        }
    }
    write(&ml, &mr);

    let mut s = CrSettings { voices: 3, mode: 0, turn_idx: 4, len_idx: 1, ..CrSettings::default() };
    let seeds = [0u32, 0xBEEF, 0x5EED, 0xACC1];
    for (round, seed) in seeds.iter().enumerate() {
        if round > 0 {
            s = s.accident(*seed, 0.9);
        }
        // Pour la démo, on force un peu de variété entre les exemples
        match round {
            1 => { s.mode = 1; s.turn_idx = 2; s.len_idx = 2; }
            2 => { s.mode = 2; s.turn_idx = 3; s.len_idx = 1; s.silence = 4; }
            3 => { s.mode = 1; s.turn_idx = 1; s.len_idx = 2; s.silence = 0; s = s.mutate(); }
            _ => {}
        }
        let p = s.pattern();
        let pat: Vec<&str> = p[..s.len()].iter().map(|&v| if v == SILENT { "·" } else { LETTERS[v as usize] }).collect();
        println!("{}. {} · tour {} · {} : {}", round + 1, MODE_NAMES[s.mode as usize], TURN_NAMES[s.turn_idx as usize], if round == 3 { "+ MUTATE" } else { "" }, pat.join(" "));

        let mut ml = vec![0.0f32; n];
        let mut mr = vec![0.0f32; n];
        for (k, (l, r)) in src.iter().enumerate() {
            let mut gate = CrGate::new(sr);
            let mut cl = l.clone();
            let mut cr = r.clone();
            let mut pos = 0.0;
            for (bl, br) in cl.chunks_mut(512).zip(cr.chunks_mut(512)) {
                let t = Transport { tempo, playing: true, pos_beats: Some(pos) };
                gate.process(bl, br, &s, k as u8, 1.0, 6.0, &t);
                pos += bl.len() as f64 * tempo / 60.0 / sr as f64;
            }
            for i in 0..n {
                ml[i] += cl[i] * gains[k];
                mr[i] += cr[i] * gains[k];
            }
        }
        write(&ml, &mr);
    }
    w.finalize().unwrap();
}
