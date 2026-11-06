use happy_accident::engine::{demo::render_stabs, Transport};
use happy_accident::rise::engine::*;

const SR: f32 = 48000.0;

fn run(e: &mut RiseEngine, l: &[f32], r: &[f32], m: &RiseMacros, sr: f32) -> (Vec<f32>, Vec<f32>) {
    let mut ol = l.to_vec();
    let mut or = r.to_vec();
    let tempo = 128.0;
    let mut pos = 0.0;
    for (cl, cr) in ol.chunks_mut(256).zip(or.chunks_mut(256)) {
        let t = Transport { tempo, playing: true, pos_beats: Some(pos) };
        e.process(cl, cr, m, &t);
        pos += cl.len() as f64 * tempo / 60.0 / sr as f64;
    }
    (ol, or)
}

fn sine(freq: f32, secs: f32, amp: f32) -> Vec<f32> {
    (0..(secs * SR) as usize).map(|i| amp * (2.0 * std::f32::consts::PI * freq * i as f32 / SR).sin()).collect()
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
}

fn all_on() -> [bool; NUM_INGREDIENTS] {
    [true; NUM_INGREDIENTS]
}

#[test]
fn tension_zero_laisse_le_son_intact() {
    let (l, r) = render_stabs(SR, 128.0, 2);
    let l: Vec<f32> = l.iter().map(|x| x * 0.8).collect();
    let r: Vec<f32> = r.iter().map(|x| x * 0.8).collect();
    for curve in [Curve::Linear, Curve::Late] {
        let mut e = RiseEngine::new(SR, 1);
        let m = RiseMacros { tension: 0.0, curve, on: all_on(), ..RiseMacros::default() };
        let (ol, or) = run(&mut e, &l, &r, &m, SR);
        assert_eq!(ol, l);
        assert_eq!(or, r);
    }
}

#[test]
fn robuste_toutes_combinaisons() {
    for &sr in &[44100.0f32, 48000.0, 96000.0] {
        let (l, r) = render_stabs(sr, 128.0, 1);
        let mut e = RiseEngine::new(sr, 7);
        for i in 0..48 {
            let rec = Recipe::roll(i as u64);
            let m = RiseMacros {
                tension: [0.0, 0.3, 0.7, 1.0][i % 4],
                curve: rec.curve,
                amount: [Amount::Subtle, Amount::Normal, Amount::Extreme][i % 3],
                tail: if i % 2 == 0 { Tail::Cut } else { Tail::Keep },
                on: if i % 5 == 0 { all_on() } else { rec.on },
                out_gain: [1.0, 2.0, 0.1][i % 3],
            };
            let gain = [1.0, 10.0, 0.0][i % 3];
            let li: Vec<f32> = l.iter().map(|x| x * gain).collect();
            let ri: Vec<f32> = r.iter().map(|x| x * gain).collect();
            let (ol, or) = run(&mut e, &li, &ri, &m, sr);
            for (a, b) in ol.iter().zip(or.iter()) {
                assert!(a.is_finite() && b.is_finite());
                assert!(a.abs() <= 0.99 && b.abs() <= 0.99);
            }
        }
    }
}

#[test]
fn low_cut_enleve_les_basses_avec_la_tension() {
    let bass = sine(60.0, 1.0, 0.5);
    let mut on = [false; NUM_INGREDIENTS];
    on[LOW_CUT] = true;
    let mut levels = vec![];
    for t in [0.0, 0.5, 1.0] {
        let mut e = RiseEngine::new(SR, 1);
        let m = RiseMacros { tension: t, curve: Curve::Linear, on, ..RiseMacros::default() };
        let (ol, _) = run(&mut e, &bass, &bass, &m, SR);
        levels.push(rms(&ol[ol.len() / 2..]));
    }
    assert!(levels[1] < levels[0] * 0.8, "{levels:?}");
    assert!(levels[2] < levels[0] * 0.02, "à 100 % le 60 Hz doit avoir disparu : {levels:?}");
}

#[test]
fn courbe_late_garde_plus_longtemps_le_son() {
    let bass = sine(60.0, 1.0, 0.5);
    let mut on = [false; NUM_INGREDIENTS];
    on[LOW_CUT] = true;
    let level = |curve| {
        let mut e = RiseEngine::new(SR, 1);
        let m = RiseMacros { tension: 0.5, curve, on, ..RiseMacros::default() };
        let (ol, _) = run(&mut e, &bass, &bass, &m, SR);
        rms(&ol[ol.len() / 2..])
    };
    assert!(level(Curve::Late) > level(Curve::Linear) * 1.5);
}

#[test]
fn drop_cut_retour_immediat_au_son_sec() {
    let (l, r) = render_stabs(SR, 128.0, 2);
    let l: Vec<f32> = l.iter().map(|x| x * 0.6).collect();
    let r: Vec<f32> = r.iter().map(|x| x * 0.6).collect();
    let mut e = RiseEngine::new(SR, 1);
    let mut m = RiseMacros { tension: 1.0, tail: Tail::Cut, on: all_on(), ..RiseMacros::default() };
    run(&mut e, &l, &r, &m, SR);
    m.tension = 0.0;
    let (ol, or) = run(&mut e, &l, &r, &m, SR);
    // Dès le premier échantillon après le drop, le son est intact
    assert_eq!(ol, l);
    assert_eq!(or, r);
}

#[test]
fn drop_keep_la_queue_continue() {
    let (l, r) = render_stabs(SR, 128.0, 2);
    let l: Vec<f32> = l.iter().map(|x| x * 0.6).collect();
    let r: Vec<f32> = r.iter().map(|x| x * 0.6).collect();
    let mut on = [false; NUM_INGREDIENTS];
    on[SPACE] = true;
    let mut e = RiseEngine::new(SR, 1);
    let mut m = RiseMacros { tension: 1.0, tail: Tail::Keep, on, ..RiseMacros::default() };
    run(&mut e, &l, &r, &m, SR);
    m.tension = 0.0;
    let silence = vec![0.0f32; SR as usize];
    let (ol, _) = run(&mut e, &silence, &silence, &m, SR);
    let tail = rms(&ol[..(SR * 0.5) as usize]);
    // Queue audible : au moins 10 % du niveau du son sec
    assert!(tail > 0.1 * rms(&l), "pas de queue en mode Keep ({tail})");
    // En mode Cut, rien
    let mut e = RiseEngine::new(SR, 1);
    m.tail = Tail::Cut;
    m.tension = 1.0;
    run(&mut e, &l, &r, &m, SR);
    m.tension = 0.0;
    let (ol, _) = run(&mut e, &silence, &silence, &m, SR);
    assert_eq!(rms(&ol), 0.0);
}

#[test]
fn gap_coupe_le_son_en_haut_de_course() {
    let s = sine(440.0, 1.0, 0.5);
    let mut on = [false; NUM_INGREDIENTS];
    on[GAP] = true;
    let mut e = RiseEngine::new(SR, 1);
    let m = RiseMacros { tension: 1.0, on, ..RiseMacros::default() };
    let (ol, _) = run(&mut e, &s, &s, &m, SR);
    assert!(rms(&ol[ol.len() / 2..]) < 0.001);
    assert!(e.state().gap_active);
    // Sous le seuil : pas de trou
    let mut e = RiseEngine::new(SR, 1);
    let m = RiseMacros { tension: 0.9, on, ..RiseMacros::default() };
    let (ol, _) = run(&mut e, &s, &s, &m, SR);
    assert!(rms(&ol[ol.len() / 2..]) > 0.3);
}

#[test]
fn roll_et_noise_font_quelque_chose() {
    let s = sine(440.0, 1.0, 0.5);
    let mut on = [false; NUM_INGREDIENTS];
    on[ROLL] = true;
    let mut e = RiseEngine::new(SR, 1);
    let m = RiseMacros { tension: 1.0, on, ..RiseMacros::default() };
    let (ol, _) = run(&mut e, &s, &s, &m, SR);
    // Gate à moitié fermé : environ -3 dB
    let ratio = rms(&ol[ol.len() / 2..]) / rms(&s);
    assert!(ratio > 0.55 && ratio < 0.85, "{ratio}");

    let silence = vec![0.0f32; SR as usize];
    let mut on = [false; NUM_INGREDIENTS];
    on[NOISE] = true;
    let mut e = RiseEngine::new(SR, 1);
    let m = RiseMacros { tension: 1.0, on, ..RiseMacros::default() };
    let (ol, _) = run(&mut e, &silence, &silence, &m, SR);
    assert!(rms(&ol) > 0.005);
}

#[test]
fn width_elargit_la_stereo() {
    let (l, r) = render_stabs(SR, 128.0, 1);
    let mut on = [false; NUM_INGREDIENTS];
    on[WIDTH] = true;
    let mut e = RiseEngine::new(SR, 1);
    let m = RiseMacros { tension: 1.0, on, ..RiseMacros::default() };
    let (ol, or) = run(&mut e, &l, &r, &m, SR);
    let side_in: Vec<f32> = l.iter().zip(&r).map(|(a, b)| a - b).collect();
    let side_out: Vec<f32> = ol.iter().zip(&or).map(|(a, b)| a - b).collect();
    assert!(rms(&side_out) > rms(&side_in) * 1.5);
}

#[test]
fn accident_deterministe_et_raisonnable() {
    assert_eq!(Recipe::roll(42), Recipe::roll(42));
    let mut differents = std::collections::HashSet::new();
    for seed in 0..500u64 {
        let r = Recipe::roll(seed);
        let n = r.on.iter().filter(|&&b| b).count();
        assert!((3..=6).contains(&n));
        assert!(r.on[LOW_CUT] || r.on[SPACE]);
        differents.insert(r.on);
    }
    assert!(differents.len() > 40);
}
