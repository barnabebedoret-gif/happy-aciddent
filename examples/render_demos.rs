//! Rend un fichier de démo : le lead sec, puis plusieurs accidents.
//! cargo run --release --example render_demos

use happy_accident::engine::{demo::{render_lead, render_sustain_lead}, Engine, Macros, Transport, KINDS};

fn main() {
    let sr = 44100.0;
    let tempo = 128.0;
    let bars = 4;
    let sustain = std::env::args().any(|a| a == "sustain");
    let (l, r) = if sustain { render_sustain_lead(sr, tempo, bars) } else { render_lead(sr, tempo, bars) };
    let out = if sustain { "demos/happy_accident_demo_sustain.wav" } else { "demos/happy_accident_demo.wav" };
    let gap = (sr * 0.6) as usize;

    std::fs::create_dir_all("demos").unwrap();
    let spec = hound::WavSpec { channels: 2, sample_rate: sr as u32, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(out, spec).unwrap();
    let mut write = |a: &[f32], b: &[f32]| {
        for (x, y) in a.iter().zip(b) {
            w.write_sample((x * 32767.0) as i16).unwrap();
            w.write_sample((y * 32767.0) as i16).unwrap();
        }
        for _ in 0..gap * 2 {
            w.write_sample(0i16).unwrap();
        }
    };

    println!("0. Lead sec (La mineur, 128 BPM{})", if sustain { ", sustain" } else { "" });
    write(&l, &r);

    let mut e = Engine::new(sr, 0xB27B_2026);
    // (balance, chaos, label)
    let runs = [(0.1, 0.9, "Timbre"), (0.15, 1.0, "Timbre"), (0.5, 0.9, "Les deux"), (0.5, 1.0, "Les deux"), (0.9, 0.9, "Pattern"), (0.95, 1.0, "Pattern")];
    for (k, (balance, chaos, label)) in runs.iter().enumerate() {
        let speed = [1.0, 2.0, 1.0, 0.5, 1.0, 2.0][k];
        let m = Macros { balance: *balance, chaos: *chaos, movement: 0.3, speed, ..Macros::default() };
        // On tire jusqu'à ~5 fois pour éviter les accidents trop sages dans la démo
        e.accident(&m);
        // Démo : on montre chaque type de mouvement du filtre
        let want = [0u8, 0, 1, 0, 2, 1][k];
        for _ in 0..40 {
            let p = e.patch();
            if p.enabled[4] && p.filter.motion == want {
                break;
            }
            e.accident(&m);
        }
        let p = *e.patch();
        let chain: Vec<String> = p
            .order
            .iter()
            .map(|&i| i as usize)
            .filter(|&i| p.enabled[i])
            .map(|i| format!("{} ({})", KINDS[i].name(), p.describe(KINDS[i])))
            .collect();
        println!("{}. [{}] Speed {}× #{:08X} : {}", k + 1, label, speed, p.seed, chain.join(" → "));

        // Petite pré-chauffe pour que le fondu de changement soit passé
        let mut t = Transport { tempo, playing: true, pos_beats: Some(-0.25) };
        let mut wl = vec![0.0f32; (sr * 0.1) as usize];
        let mut wr = wl.clone();
        e.process(&mut wl, &mut wr, &m, &t);
        t.pos_beats = Some(0.0);

        let mut ol = l.clone();
        let mut or = r.clone();
        let mut pos = 0.0;
        for (cl, cr) in ol.chunks_mut(512).zip(or.chunks_mut(512)) {
            t.pos_beats = Some(pos);
            e.process(cl, cr, &m, &t);
            pos += cl.len() as f64 * tempo / 60.0 / sr as f64;
        }
        write(&ol, &or);
    }
    w.finalize().unwrap();
}
