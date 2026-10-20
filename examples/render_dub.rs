//! Démo du Happy Accident DUB sur des stabs d'accords.
//! cargo run --release --example render_dub

use happy_accident::dub::engine::{DubEngine, DubMacros};
use happy_accident::engine::{demo::render_stabs, Transport};

fn main() {
    let sr = 44100.0;
    let tempo = 124.0;
    let bars = 4;
    let (l, r) = render_stabs(sr, tempo, bars);
    // 2 mesures de silence après chaque exemple pour entendre la queue des échos
    let tail = (2.0 * 4.0 * 60.0 / tempo * sr as f64) as usize;

    std::fs::create_dir_all("demos").unwrap();
    let spec = hound::WavSpec { channels: 2, sample_rate: sr as u32, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create("demos/happy_accident_dub_demo.wav", spec).unwrap();

    println!("0. Stabs secs (La mineur 9, {tempo} BPM)");
    for (a, b) in l.iter().zip(&r) {
        w.write_sample((a * 32767.0) as i16).unwrap();
        w.write_sample((b * 32767.0) as i16).unwrap();
    }
    for _ in 0..(sr as usize) * 2 {
        w.write_sample(0i16).unwrap();
    }

    let mut e = DubEngine::new(sr, 0xD0B_2026);
    let feedbacks = [0.75, 0.8, 0.85, 0.75, 0.8, 0.85];
    for k in 0..6 {
        let mut m = DubMacros { chaos: 0.9, feedback: feedbacks[k], mix: 0.65, movement: 0.6, ..DubMacros::default() };
        e.accident(&m);
        e.clear();
        let p = *e.patch();
        let desc: Vec<String> = (0..4).map(|g| p.describe(g)).collect();
        let throw = k == 4;
        println!("{}. #{:08X} : {}{}", k + 1, p.seed, desc.join(" | "), if throw { "  + THROW sur la dernière mesure" } else { "" });

        let mut ol = l.clone();
        ol.extend(std::iter::repeat(0.0).take(tail));
        let mut or = r.clone();
        or.extend(std::iter::repeat(0.0).take(tail));
        let bar = 4.0 * 60.0 / tempo * sr as f64;
        let mut pos = 0.0;
        let mut idx = 0usize;
        for (cl, cr) in ol.chunks_mut(256).zip(or.chunks_mut(256)) {
            m.throw = throw && (idx as f64) >= 3.0 * bar && (idx as f64) < 4.0 * bar;
            let t = Transport { tempo, playing: true, pos_beats: Some(pos) };
            e.process(cl, cr, &m, &t);
            pos += cl.len() as f64 * tempo / 60.0 / sr as f64;
            idx += cl.len();
        }
        for (a, b) in ol.iter().zip(&or) {
            w.write_sample((a * 32767.0) as i16).unwrap();
            w.write_sample((b * 32767.0) as i16).unwrap();
        }
    }
    w.finalize().unwrap();
}
