//! Démo du Happy Accident RISE : 8 mesures de montée (TENSION 0 → 100 %), puis 4 mesures de drop.
//! cargo run --release --example render_rise

use happy_accident::engine::{demo::render_stabs, demo::render_sustain_lead, Transport};
use happy_accident::rise::engine::*;
use std::f32::consts::PI;

/// Petite boucle techno : kick, basse sur les contretemps, stabs et lead.
fn loop_techno(sr: f32, tempo: f64, bars: usize) -> (Vec<f32>, Vec<f32>) {
    let beat = (60.0 / tempo * sr as f64) as usize;
    let n = beat * 4 * bars;
    let (sl, sr_) = render_stabs(sr, tempo, bars);
    let (ll, lr) = render_sustain_lead(sr, tempo, bars);
    let mut l = vec![0.0f32; n];
    let mut r = vec![0.0f32; n];
    for i in 0..n {
        let in_beat = i % beat;
        let t = in_beat as f32 / sr;
        // Kick
        let f = 48.0 + 110.0 * (-t * 35.0).exp();
        let phase = 2.0 * PI * (48.0 * t + 110.0 / 35.0 * (1.0 - (-t * 35.0).exp()));
        let kick = 0.85 * phase.sin() * (-t * 7.0).exp() * (f > 0.0) as u8 as f32;
        // Basse sur le contretemps
        let off = (in_beat as i64 - beat as i64 / 2).max(0) as f32 / sr;
        let bass = if in_beat >= beat / 2 { 0.35 * (2.0 * PI * 55.0 * off).sin().signum() * 0.4 * (-off * 9.0).exp() + 0.3 * (2.0 * PI * 55.0 * off).sin() * (-off * 9.0).exp() } else { 0.0 };
        let mono = kick + bass;
        l[i] = mono + 1.6 * sl.get(i).copied().unwrap_or(0.0) + 0.5 * ll.get(i).copied().unwrap_or(0.0);
        r[i] = mono + 1.6 * sr_.get(i).copied().unwrap_or(0.0) + 0.5 * lr.get(i).copied().unwrap_or(0.0);
    }
    let peak = l.iter().chain(r.iter()).fold(0.0f32, |m, x| m.max(x.abs()));
    let g = 0.8 / peak.max(1e-6);
    l.iter_mut().chain(r.iter_mut()).for_each(|x| *x *= g);
    (l, r)
}

fn main() {
    let sr = 44100.0;
    let tempo = 128.0;
    let build_bars = 8;
    let drop_bars = 4;
    let (l, r) = loop_techno(sr, tempo, build_bars + drop_bars);
    let bar = (4.0 * 60.0 / tempo * sr as f64) as usize;
    let build_len = bar * build_bars;

    std::fs::create_dir_all("demos").unwrap();
    let spec = hound::WavSpec { channels: 2, sample_rate: sr as u32, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create("demos/happy_accident_rise_demo.wav", spec).unwrap();

    let examples: [(&str, f32, f32, f32); 4] = [
        ("Réglages par défaut : CUT 50 %, SPACE 50 %, ROLL 0 %", 0.5, 0.5, 0.0),
        ("Avec ROLL : CUT 50 %, SPACE 50 %, ROLL 100 %", 0.5, 0.5, 1.0),
        ("Radical : CUT 100 %, SPACE 100 %, ROLL 70 %", 1.0, 1.0, 0.7),
        ("Léger : CUT 0 %, SPACE 30 %, ROLL 0 %", 0.0, 0.3, 0.0),
    ];

    for (k, (name, cut, space, roll)) in examples.iter().enumerate() {
        println!("{}. {}  (8 mesures de montée, puis drop de 4 mesures)", k + 1, name);
        let mut e = RiseEngine::new(sr);
        let mut ol = l.clone();
        let mut or = r.clone();
        let mut pos = 0.0;
        let mut idx = 0usize;
        for (cl, cr) in ol.chunks_mut(256).zip(or.chunks_mut(256)) {
            // Automation : rampe linéaire 0 → 100 % sur 8 mesures, puis 0 sur le drop
            let tension = if idx < build_len { idx as f32 / build_len as f32 } else { 0.0 };
            let m = RiseMacros { tension, cut: *cut, space: *space, roll: *roll };
            let t = Transport { tempo, playing: true, pos_beats: Some(pos) };
            e.process(cl, cr, &m, &t);
            pos += cl.len() as f64 * tempo / 60.0 / sr as f64;
            idx += cl.len();
        }
        for (a, b) in ol.iter().zip(&or) {
            w.write_sample((a * 32767.0) as i16).unwrap();
            w.write_sample((b * 32767.0) as i16).unwrap();
        }
        for _ in 0..sr as usize {
            w.write_sample(0i16).unwrap();
            w.write_sample(0i16).unwrap();
        }
    }
    w.finalize().unwrap();
    println!("→ demos/happy_accident_rise_demo.wav");
}
