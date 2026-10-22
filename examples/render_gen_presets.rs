//! Démo des presets d'usine du Generator sur le lead tenu.
use happy_accident::engine::{demo::render_sustain_lead, Engine, Macros, Transport};
use happy_accident::presets::PRESETS;

fn main() {
    let sr = 44100.0;
    let tempo = 128.0;
    let (l, r) = render_sustain_lead(sr, tempo, 4);
    let gap = (sr * 0.8) as usize;
    std::fs::create_dir_all("demos").unwrap();
    let spec = hound::WavSpec { channels: 2, sample_rate: sr as u32, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create("demos/happy_accident_gen_presets.wav", spec).unwrap();
    let mut t_sec = 0.0f64;
    let mut write = |a: &[f32], b: &[f32], t: &mut f64| {
        for (x, y) in a.iter().zip(b) {
            w.write_sample((x * 32767.0) as i16).unwrap();
            w.write_sample((y * 32767.0) as i16).unwrap();
        }
        for _ in 0..gap * 2 {
            w.write_sample(0i16).unwrap();
        }
        *t += (a.len() + gap) as f64 / sr as f64;
    };
    let stamp = |t: f64| format!("{}:{:02}", (t / 60.0) as u32, (t % 60.0) as u32);
    println!("{}  Lead sec", stamp(t_sec));
    write(&l, &r, &mut t_sec);
    for p in PRESETS {
        println!("{}  {}", stamp(t_sec), p.name);
        let speed = [0.25, 0.5, 1.0, 2.0, 4.0][p.speed as usize];
        let m = Macros { chaos: p.chaos, balance: p.balance, movement: p.movement, speed, locks: p.locks, ..Macros::default() };
        let mut e = Engine::new(sr, 3);
        e.load_patch((p.build)());
        // pré-chauffe (fondu de changement)
        let mut wl = vec![0.0f32; (sr * 0.05) as usize];
        let mut wr = wl.clone();
        e.process(&mut wl, &mut wr, &m, &Transport { tempo, playing: true, pos_beats: Some(-0.2) });
        let mut ol = l.clone();
        let mut or = r.clone();
        let mut pos = 0.0;
        for (cl, cr) in ol.chunks_mut(512).zip(or.chunks_mut(512)) {
            e.process(cl, cr, &m, &Transport { tempo, playing: true, pos_beats: Some(pos) });
            pos += cl.len() as f64 * tempo / 60.0 / sr as f64;
        }
        write(&ol, &or, &mut t_sec);
    }
    w.finalize().unwrap();
}
