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

fn stabs(gain: f32) -> (Vec<f32>, Vec<f32>) {
    let (l, r) = render_stabs(SR, 128.0, 2);
    (l.iter().map(|x| x * gain).collect(), r.iter().map(|x| x * gain).collect())
}

fn full() -> RiseMacros {
    RiseMacros { tension: 1.0, cut: 1.0, space: 1.0, roll: 1.0 }
}

#[test]
fn tension_zero_laisse_le_son_intact() {
    let (l, r) = stabs(0.8);
    let mut e = RiseEngine::new(SR);
    let m = RiseMacros { tension: 0.0, ..full() };
    let (ol, or) = run(&mut e, &l, &r, &m, SR);
    assert_eq!(ol, l);
    assert_eq!(or, r);
}

#[test]
fn robuste_tous_reglages() {
    for &sr in &[44100.0f32, 48000.0, 96000.0] {
        let (l, r) = render_stabs(sr, 128.0, 1);
        let mut e = RiseEngine::new(sr);
        for i in 0..40 {
            let m = RiseMacros {
                tension: [0.0, 0.3, 0.7, 1.0][i % 4],
                cut: [0.0, 0.5, 1.0][i % 3],
                space: [1.0, 0.0, 0.5][i % 3],
                roll: [0.0, 1.0][i % 2],
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
fn cut_enleve_les_basses_avec_la_tension() {
    let bass = sine(60.0, 1.0, 0.5);
    let level = |tension: f32, cut: f32| {
        let mut e = RiseEngine::new(SR);
        let m = RiseMacros { tension, cut, space: 0.0, roll: 0.0 };
        let (ol, _) = run(&mut e, &bass, &bass, &m, SR);
        rms(&ol[ol.len() / 2..])
    };
    let dry = level(0.0, 0.5);
    assert!(level(0.6, 0.5) < dry * 0.9);
    assert!(level(1.0, 0.0) < dry * 0.1, "à 100 % le 60 Hz doit avoir quasi disparu");
    assert!(level(1.0, 1.0) < dry * 0.01);
    // Plus de CUT = plus de basses enlevées
    assert!(level(0.8, 1.0) < level(0.8, 0.0));
}

#[test]
fn la_courbe_monte_tard() {
    assert!(shape(0.5) < 0.2);
    assert!(shape(0.9) > 0.75);
    assert_eq!(shape(0.0), 0.0);
    assert_eq!(shape(1.0), 1.0);
    assert!((cut_max_hz(0.0) - 150.0).abs() < 0.1 && (cut_max_hz(1.0) - 1000.0).abs() < 0.5);
}

#[test]
fn drop_retour_immediat_au_son_sec_et_reverb_coupee() {
    let (l, r) = stabs(0.6);
    let mut e = RiseEngine::new(SR);
    let mut m = full();
    run(&mut e, &l, &r, &m, SR);
    m.tension = 0.0;
    let (ol, or) = run(&mut e, &l, &r, &m, SR);
    assert_eq!(ol, l);
    assert_eq!(or, r);
    // Et rien ne traîne dans le silence qui suit
    let silence = vec![0.0f32; SR as usize / 2];
    let (ol, _) = run(&mut e, &silence, &silence, &m, SR);
    assert_eq!(rms(&ol), 0.0);
}

#[test]
fn space_ajoute_reverb_et_largeur() {
    let (l, r) = stabs(0.6);
    let side = |space: f32| {
        let mut e = RiseEngine::new(SR);
        let m = RiseMacros { tension: 1.0, cut: 0.0, space, roll: 0.0 };
        let (ol, or) = run(&mut e, &l, &r, &m, SR);
        let s: Vec<f32> = ol.iter().zip(&or).map(|(a, b)| a - b).collect();
        (rms(&s), rms(&ol))
    };
    let (s0, _) = side(0.0);
    let (s1, _) = side(1.0);
    assert!(s1 > s0 * 1.5, "{s0} {s1}");
    // Avec SPACE, la queue continue dans le silence tant que la tension est là
    let mut e = RiseEngine::new(SR);
    let m = RiseMacros { tension: 1.0, cut: 0.0, space: 1.0, roll: 0.0 };
    run(&mut e, &l, &r, &m, SR);
    let silence = vec![0.0f32; SR as usize / 2];
    let (ol, _) = run(&mut e, &silence, &silence, &m, SR);
    assert!(rms(&ol) > 0.1 * rms(&l));
}

#[test]
fn roll_hache_le_son_et_accelere() {
    let s = sine(440.0, 2.0, 0.5);
    let gated = |tension: f32, roll: f32| {
        let mut e = RiseEngine::new(SR);
        let m = RiseMacros { tension, cut: 0.0, space: 0.0, roll };
        let (ol, _) = run(&mut e, &s, &s, &m, SR);
        ol
    };
    // ROLL à fond : environ la moitié du temps coupée, par rapport à ROLL 0
    let off = gated(1.0, 0.0);
    let on = gated(1.0, 1.0);
    let ratio = rms(&on[on.len() / 2..]) / rms(&off[off.len() / 2..]);
    assert!(ratio > 0.55 && ratio < 0.85, "{ratio}");
    // La division accélère avec la tension
    assert!(roll_steps(0.1) < roll_steps(0.4) && roll_steps(0.4) < roll_steps(0.6) && roll_steps(0.6) < roll_steps(0.9));
    assert_eq!(roll_steps(1.0), 8.0);
}
