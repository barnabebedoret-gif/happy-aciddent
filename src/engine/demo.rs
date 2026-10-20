//! Générateur de lead de test (pour les tests et les démos audio).

use super::dsp::{midi_to_hz, OnePole};

/// Lead "saw" en arpège, La mineur, 1/16. Retourne (gauche, droite).
pub fn render_lead(sr: f32, tempo: f64, bars: usize) -> (Vec<f32>, Vec<f32>) {
    const NOTES: [i32; 16] = [57, 60, 64, 69, 67, 64, 62, 60, 57, 60, 64, 72, 71, 67, 64, 62];
    let spb = 60.0 / tempo * sr as f64;
    let step_len = spb / 4.0;
    let total = (bars as f64 * 4.0 * spb) as usize;
    let mut l = Vec::with_capacity(total);
    let mut r = Vec::with_capacity(total);
    let (mut ph1, mut ph2) = (0.0f32, 0.0f32);
    let mut lp = OnePole::default();
    let mut env = OnePole::default();
    for i in 0..total {
        let step = (i as f64 / step_len) as usize;
        let pos = (i as f64 - step as f64 * step_len) / step_len;
        let f = midi_to_hz(NOTES[step % 16] as f32);
        ph1 = (ph1 + f / sr).fract();
        ph2 = (ph2 + f * 1.007 / sr).fract();
        let saw = (2.0 * ph1 - 1.0) + (2.0 * ph2 - 1.0);
        let gate = if pos < 0.7 { 1.0 } else { 0.0 };
        let e = env.lp(gate, OnePole::coef(200.0, sr));
        let cut = 900.0 + 3500.0 * (1.0 - pos as f32).powi(2);
        let y = lp.lp(saw, OnePole::coef(cut, sr)) * e * 0.3;
        l.push(y);
        r.push(y);
    }
    (l, r)
}

/// Lead long "supersaw" stéréo, notes tenues (gros sustain), La mineur.
pub fn render_sustain_lead(sr: f32, tempo: f64, bars: usize) -> (Vec<f32>, Vec<f32>) {
    // (note MIDI, durée en temps) — enchaînées legato
    const PHRASE: [(i32, f64); 6] = [(69, 4.0), (67, 2.0), (64, 2.0), (65, 4.0), (64, 3.0), (72, 1.0)];
    const DETUNE: [f32; 5] = [-0.018, -0.008, 0.0, 0.009, 0.017];
    let spb = 60.0 / tempo * sr as f64;
    let total = (bars as f64 * 4.0 * spb) as usize;
    let phrase_len: f64 = PHRASE.iter().map(|p| p.1).sum();
    let mut l = Vec::with_capacity(total);
    let mut r = Vec::with_capacity(total);
    let mut ph = [0.0f32; 5];
    let mut glide = OnePole { z: PHRASE[0].0 as f32 };
    let (mut lpl, mut lpr) = (OnePole::default(), OnePole::default());
    let mut amp = OnePole::default();
    for i in 0..total {
        let beat = i as f64 / spb;
        let mut b = beat % phrase_len;
        let mut note = PHRASE[0].0;
        for &(n, d) in PHRASE.iter() {
            if b < d {
                note = n;
                break;
            }
            b -= d;
        }
        let vib = (2.0 * std::f32::consts::PI * 5.0 * i as f32 / sr).sin() * 0.08 * ((beat % 4.0) as f32 / 4.0);
        let n = glide.lp(note as f32, OnePole::coef(25.0, sr)) + vib;
        let f = midi_to_hz(n);
        let (mut sl, mut sr_) = (0.0, 0.0);
        for (k, d) in DETUNE.iter().enumerate() {
            ph[k] = (ph[k] + f * (1.0 + d) / sr).fract();
            let s = 2.0 * ph[k] - 1.0;
            let pan = k as f32 / 4.0;
            sl += s * (1.0 - pan * 0.7);
            sr_ += s * (0.3 + pan * 0.7);
        }
        let a = amp.lp(1.0, OnePole::coef(4.0, sr));
        let k = OnePole::coef(3200.0, sr);
        l.push(lpl.lp(sl, k) * a * 0.12);
        r.push(lpr.lp(sr_, k) * a * 0.12);
    }
    (l, r)
}
