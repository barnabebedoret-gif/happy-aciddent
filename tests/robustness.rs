use happy_accident::engine::{demo::render_lead, Engine, History, Macros, Patch, Scale, Transport, NUM_MODULES};

fn render(engine: &mut Engine, l: &[f32], r: &[f32], m: &Macros, t: &mut Transport, sr: f32) -> (Vec<f32>, Vec<f32>) {
    let mut ol = l.to_vec();
    let mut or = r.to_vec();
    for (cl, cr) in ol.chunks_mut(256).zip(or.chunks_mut(256)) {
        engine.process(cl, cr, m, t);
        if let Some(p) = t.pos_beats.as_mut() {
            *p += cl.len() as f64 * t.tempo / 60.0 / sr as f64;
        }
    }
    (ol, or)
}

#[test]
fn trois_cents_accidents_sans_crash_ni_saturation() {
    for &sr in &[44100.0f32, 48000.0, 96000.0] {
        let (l, r) = render_lead(sr, 128.0, 1);
        let mut e = Engine::new(sr, 42);
        let mut rng = 1u64;
        let mut next = || {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (rng >> 40) as f32 / (1u64 << 24) as f32
        };
        for i in 0..100 {
            let m = Macros {
                chaos: next(),
                balance: next(),
                movement: next(),
                mix: next(),
                out_gain: 1.0 + next() * 3.0,
                key: (i % 12) as u8,
                scale: [Scale::Minor, Scale::Major, Scale::PentaMinor, Scale::Phrygian][i % 4],
                locks: [false; NUM_MODULES],
            };
            if i % 3 == 0 { e.mutate(&m) } else { e.accident(&m) }
            let tempo = 60.0 + next() as f64 * 140.0;
            let mut t = Transport { tempo, playing: i % 2 == 0, pos_beats: Some(next() as f64 * 32.0) };
            // Entrée parfois très forte, parfois silencieuse
            let gain = [1.0, 6.0, 0.0][i % 3];
            let li: Vec<f32> = l.iter().map(|x| x * gain).collect();
            let ri: Vec<f32> = r.iter().map(|x| x * gain).collect();
            let (ol, or) = render(&mut e, &li, &ri, &m, &mut t, sr);
            for (a, b) in ol.iter().zip(or.iter()) {
                assert!(a.is_finite() && b.is_finite(), "valeur invalide (sr {sr}, accident {i})");
                assert!(a.abs() <= 0.99 && b.abs() <= 0.99, "dépasse le plafond (sr {sr}, accident {i})");
            }
        }
    }
}

#[test]
fn le_son_sort_vraiment() {
    let sr = 48000.0;
    let (l, r) = render_lead(sr, 128.0, 2);
    let mut e = Engine::new(sr, 7);
    let m = Macros::default();
    let mut silent = 0;
    for _ in 0..30 {
        e.accident(&m);
        let mut t = Transport { tempo: 128.0, playing: true, pos_beats: Some(0.0) };
        let (ol, _) = render(&mut e, &l, &r, &m, &mut t, sr);
        let rms = (ol.iter().map(|x| x * x).sum::<f32>() / ol.len() as f32).sqrt();
        if rms < 0.01 {
            silent += 1;
        }
    }
    assert!(silent <= 1, "{silent} accidents quasi silencieux sur 30");
}

#[test]
fn locks_et_historique() {
    let mut e = Engine::new(48000.0, 3);
    let mut m = Macros { chaos: 1.0, ..Macros::default() };
    e.accident(&m);
    let first = *e.patch();
    m.locks = [false, false, false, true, true, false];
    for _ in 0..20 {
        e.accident(&m);
        assert_eq!(e.patch().drive, first.drive, "Drive verrouillé a changé");
        assert_eq!(e.patch().filter, first.filter, "Filter verrouillé a changé");
        assert_eq!(e.patch().enabled[3], first.enabled[3]);
    }
    let last = *e.patch();
    e.history_prev();
    assert_ne!(*e.patch(), last);
    e.history_next();
    assert_eq!(*e.patch(), last);
}

#[test]
fn meme_seed_meme_accident() {
    let a = Patch::generate(&Patch::default(), 1234, 0.8, 0.5, &[false; 6]);
    let b = Patch::generate(&Patch::default(), 1234, 0.8, 0.5, &[false; 6]);
    assert_eq!(a, b);
}

#[test]
fn historique_se_sauvegarde() {
    let mut e = Engine::new(48000.0, 9);
    for _ in 0..20 {
        e.accident(&Macros::default());
    }
    let h = *e.history();
    let json = serde_json::to_string(&h).unwrap();
    let back: History = serde_json::from_str(&json).unwrap();
    assert_eq!(h, back);
    let mut e2 = Engine::new(48000.0, 1);
    e2.load_history(back);
    assert_eq!(e2.patch(), e.patch());
}
