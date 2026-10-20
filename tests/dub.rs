use happy_accident::dub::engine::{DubEngine, DubHistory, DubMacros, DubPatch};
use happy_accident::engine::{demo::render_stabs, Transport};

fn run(e: &mut DubEngine, l: &[f32], r: &[f32], m: &DubMacros, tempo: f64, sr: f32) -> (Vec<f32>, Vec<f32>) {
    let mut ol = l.to_vec();
    let mut or = r.to_vec();
    let mut pos = 0.0;
    for (cl, cr) in ol.chunks_mut(256).zip(or.chunks_mut(256)) {
        let t = Transport { tempo, playing: true, pos_beats: Some(pos) };
        e.process(cl, cr, m, &t);
        pos += cl.len() as f64 * tempo / 60.0 / sr as f64;
    }
    (ol, or)
}

#[test]
fn dub_robuste_meme_en_throw() {
    for &sr in &[44100.0f32, 48000.0, 96000.0] {
        let (l, r) = render_stabs(sr, 128.0, 1);
        let mut e = DubEngine::new(sr, 5);
        for i in 0..60 {
            let m = DubMacros {
                chaos: (i % 10) as f32 / 9.0,
                feedback: [0.0, 0.5, 0.95][i % 3],
                mix: 1.0,
                movement: 1.0,
                out_gain: 2.0,
                throw: i % 4 == 0,
                locks: [false; 4],
            };
            e.accident(&m);
            let gain = [1.0, 8.0, 0.0][i % 3];
            let li: Vec<f32> = l.iter().map(|x| x * gain).collect();
            let ri: Vec<f32> = r.iter().map(|x| x * gain).collect();
            let tempo = [70.0, 128.0, 175.0][i % 3];
            let (ol, or) = run(&mut e, &li, &ri, &m, tempo, sr);
            for (a, b) in ol.iter().zip(or.iter()) {
                assert!(a.is_finite() && b.is_finite());
                assert!(a.abs() <= 0.99 && b.abs() <= 0.99);
            }
        }
    }
}

#[test]
fn les_echos_continuent_apres_le_son() {
    let sr = 48000.0;
    let (l, r) = render_stabs(sr, 128.0, 1);
    let mut e = DubEngine::new(sr, 1);
    let m = DubMacros { feedback: 0.7, ..DubMacros::default() };
    run(&mut e, &l, &r, &m, 128.0, sr);
    let silence = vec![0.0f32; sr as usize];
    let (ol, _) = run(&mut e, &silence, &silence, &m, 128.0, sr);
    let rms = (ol.iter().map(|x| x * x).sum::<f32>() / ol.len() as f32).sqrt();
    assert!(rms > 0.003, "pas d'échos après l'arrêt (rms {rms})");
}

#[test]
fn dub_locks_et_historique() {
    let mut e = DubEngine::new(48000.0, 3);
    let mut m = DubMacros { chaos: 1.0, ..DubMacros::default() };
    e.accident(&m);
    let first = *e.patch();
    m.locks = [true, false, true, false];
    for _ in 0..20 {
        e.accident(&m);
        assert_eq!((e.patch().time_l, e.patch().time_r, e.patch().pingpong), (first.time_l, first.time_r, first.pingpong));
        assert_eq!((e.patch().filter_mode, e.patch().cut_steps), (first.filter_mode, first.cut_steps));
    }
    let h = *e.history();
    let back: DubHistory = serde_json::from_str(&serde_json::to_string(&h).unwrap()).unwrap();
    assert_eq!(h, back);
    let a = DubPatch::generate(&DubPatch::default(), 42, 0.8, &[false; 4]);
    let b = DubPatch::generate(&DubPatch::default(), 42, 0.8, &[false; 4]);
    assert_eq!(a, b);
}
