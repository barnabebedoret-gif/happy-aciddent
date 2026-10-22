//! Démo des presets d'usine du CALL & RESPONSE (4 pistes : A stabs, B arpège, C lead tenu, D stabs décalés).
use happy_accident::cr::engine::{CrGate, CrSettings};
use happy_accident::cr::presets::PRESETS;
use happy_accident::engine::{demo::{render_lead, render_stabs, render_sustain_lead}, Transport};

fn main() {
    let sr = 44100.0;
    let tempo = 124.0;
    let bars = 4;
    let spb = 60.0 / tempo * sr as f64;
    let mut src = vec![render_stabs(sr, tempo, bars * 2), render_lead(sr, tempo, bars * 2), render_sustain_lead(sr, tempo, bars * 2)];
    // D : les stabs décalés d'un quart de temps et plus aigus (simple variante pour la démo)
    let (sl, sr2) = render_stabs(sr, tempo * 2.0, bars * 4);
    let n = src[0].0.len();
    let shift = (spb * 0.25) as usize;
    let d_l: Vec<f32> = (0..n).map(|i| sl[(i + shift) % sl.len()] * 0.8).collect();
    let d_r: Vec<f32> = (0..n).map(|i| sr2[(i + shift) % sr2.len()] * 0.8).collect();
    src.push((d_l, d_r));
    let gains = [1.4f32, 0.8, 1.0, 1.2];

    std::fs::create_dir_all("demos").unwrap();
    let spec = hound::WavSpec { channels: 2, sample_rate: sr as u32, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create("demos/happy_accident_cr_presets.wav", spec).unwrap();

    // (index du preset, nombre de mesures, variations : (mesure de départ, settings))
    let picks = ["Ping-Pong · 1 bar", "Three Calls, One Answer", "Trio Rotation", "Chatter 1/16", "Glitch Swarm", "Ghost Notes", "Build-Up Ramp"];
    for name in picks {
        let p = PRESETS.iter().find(|p| p.name == name).unwrap();
        let len_bars = if p.var1.is_some() { 8 } else { 4 };
        let len = (len_bars as f64 * 4.0 * spb) as usize;
        // Pour un preset à variations : 4 mesures LIVE, 2 mesures var 1, 2 mesures var 2
        let schedule: Vec<(usize, CrSettings)> = if let (Some(a), Some(b)) = (p.var1, p.var2) {
            vec![(0, p.live), ((4.0 * 4.0 * spb) as usize, a), ((6.0 * 4.0 * spb) as usize, b)]
        } else {
            vec![(0, p.live)]
        };
        println!("{} ({} mesures)", p.name, len_bars);
        let mut ml = vec![0.0f32; len];
        let mut mr = vec![0.0f32; len];
        for (k, (l, r)) in src.iter().enumerate() {
            let mut gate = CrGate::new(sr);
            let mut cl: Vec<f32> = l[..len].to_vec();
            let mut cr: Vec<f32> = r[..len].to_vec();
            let mut pos = 0.0;
            let mut idx = 0;
            for (bl, br) in cl.chunks_mut(256).zip(cr.chunks_mut(256)) {
                let s = schedule.iter().rev().find(|(start, _)| idx >= *start).unwrap().1;
                let t = Transport { tempo, playing: true, pos_beats: Some(pos) };
                gate.process(bl, br, &s, k as u8, 1.0, if s.turn_idx == 0 { 2.0 } else { 6.0 }, &t);
                pos += bl.len() as f64 * tempo / 60.0 / sr as f64;
                idx += bl.len();
            }
            for i in 0..len {
                ml[i] += cl[i] * gains[k];
                mr[i] += cr[i] * gains[k];
            }
        }
        for (a, b) in ml.iter().zip(&mr) {
            w.write_sample(((a * 0.8).clamp(-1.0, 1.0) * 32767.0) as i16).unwrap();
            w.write_sample(((b * 0.8).clamp(-1.0, 1.0) * 32767.0) as i16).unwrap();
        }
        for _ in 0..(sr as usize) * 2 {
            w.write_sample(0i16).unwrap();
        }
    }
    w.finalize().unwrap();
}
