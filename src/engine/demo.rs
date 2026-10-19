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
