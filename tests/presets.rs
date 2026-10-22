use happy_accident::engine::{demo::{render_lead, render_sustain_lead}, Engine, Macros, Transport};
use happy_accident::presets::PRESETS;

fn run(e: &mut Engine, l: &[f32], r: &[f32], m: &Macros, sr: f32) -> Vec<f32> {
    let mut ol = l.to_vec();
    let mut or = r.to_vec();
    let mut pos = 0.0;
    for (cl, cr) in ol.chunks_mut(256).zip(or.chunks_mut(256)) {
        let t = Transport { tempo: 128.0, playing: true, pos_beats: Some(pos) };
        e.process(cl, cr, m, &t);
        pos += cl.len() as f64 * 128.0 / 60.0 / sr as f64;
    }
    ol.extend(or);
    ol
}

#[test]
fn chaque_preset_sonne_et_reste_propre() {
    let sr = 48000.0;
    let sources = [render_sustain_lead(sr, 128.0, 4), render_lead(sr, 128.0, 4)];
    let mut names = std::collections::HashSet::new();
    assert!(PRESETS.len() >= 14);
    for p in PRESETS {
        assert!(names.insert(p.name), "nom en double : {}", p.name);
        let patch = (p.build)();
        assert!(patch.enabled.iter().any(|&e| e), "{} : aucun effet allumé", p.name);
        let mut order = patch.order;
        order.sort();
        assert_eq!(order, [0, 1, 2, 3, 4, 5], "{} : ordre invalide", p.name);
        let speed = [0.25, 0.5, 1.0, 2.0, 4.0][p.speed as usize];
        let m = Macros { chaos: p.chaos, balance: p.balance, movement: p.movement, speed, locks: p.locks, ..Macros::default() };
        for (l, r) in &sources {
            let mut e = Engine::new(sr, 1);
            e.load_patch(patch);
            let out = run(&mut e, l, r, &m, sr);
            let tail = &out[out.len() / 4..];
            assert!(tail.iter().all(|x| x.is_finite() && x.abs() <= 0.99), "{} : sortie invalide", p.name);
            let rms = (tail.iter().map(|x| x * x).sum::<f32>() / tail.len() as f32).sqrt();
            assert!(rms > 0.01, "{} : quasi silencieux (rms {rms})", p.name);
            // Les LOCK du preset protègent bien les modules choisis
            let before = *e.patch();
            e.accident(&m);
            for i in 0..6 {
                if p.locks[i] {
                    assert_eq!(e.patch().enabled[i], before.enabled[i], "{} : lock {i} non respecté", p.name);
                }
            }
        }
    }
}
