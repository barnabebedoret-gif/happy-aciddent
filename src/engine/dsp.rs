//! Briques DSP de base.

use std::f32::consts::PI;

/// Ligne à retard circulaire (taille puissance de 2) avec lecture interpolée.
pub struct DelayLine {
    buf: Vec<f32>,
    mask: usize,
    pos: usize,
}

impl DelayLine {
    pub fn new(min_len: usize) -> Self {
        let len = min_len.max(4).next_power_of_two();
        DelayLine { buf: vec![0.0; len], mask: len - 1, pos: 0 }
    }

    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    #[inline]
    pub fn write(&mut self, x: f32) {
        self.pos = (self.pos + 1) & self.mask;
        self.buf[self.pos] = x;
    }

    /// Lecture `d` échantillons dans le passé (0 = dernier écrit).
    #[inline]
    pub fn read(&self, d: f32) -> f32 {
        let d = d.clamp(0.0, (self.buf.len() - 2) as f32);
        let di = d as usize;
        let frac = d - di as f32;
        let a = self.buf[(self.pos.wrapping_sub(di)) & self.mask];
        let b = self.buf[(self.pos.wrapping_sub(di + 1)) & self.mask];
        a + (b - a) * frac
    }

    pub fn clear(&mut self) {
        self.buf.iter_mut().for_each(|x| *x = 0.0);
    }
}

/// Passe-bas à un pôle.
#[derive(Clone, Copy, Default)]
pub struct OnePole {
    pub z: f32,
}

impl OnePole {
    #[inline]
    pub fn coef(cutoff_hz: f32, sr: f32) -> f32 {
        1.0 - (-2.0 * PI * cutoff_hz.min(sr * 0.45) / sr).exp()
    }
    #[inline]
    pub fn lp(&mut self, x: f32, c: f32) -> f32 {
        self.z += c * (x - self.z);
        self.z
    }
}

/// Filtre à variable d'état (TPT, Cytomic) - sortie passe-bande et passe-bas.
#[derive(Clone, Copy, Default)]
pub struct Svf {
    ic1: f32,
    ic2: f32,
}

impl Svf {
    /// Retourne (passe-bas, passe-bande)
    #[inline]
    pub fn process(&mut self, x: f32, cutoff: f32, q: f32, sr: f32) -> (f32, f32) {
        let g = (PI * cutoff.clamp(20.0, sr * 0.45) / sr).tan();
        let k = 1.0 / q.max(0.1);
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        let v3 = x - self.ic2;
        let v1 = a1 * self.ic1 + a2 * v3;
        let v2 = self.ic2 + a2 * self.ic1 + a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        (v2, v1)
    }
    pub fn reset(&mut self) {
        *self = Svf::default();
    }
}

/// Bloqueur de composante continue.
#[derive(Clone, Copy, Default)]
pub struct DcBlock {
    x1: f32,
    y1: f32,
}

impl DcBlock {
    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = x - self.x1 + 0.995 * self.y1;
        self.x1 = x;
        self.y1 = y;
        y
    }
}

#[inline]
pub fn db_to_gain(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

#[inline]
pub fn midi_to_hz(n: f32) -> f32 {
    440.0 * 2f32.powf((n - 69.0) / 12.0)
}

/// Garde-fou : remplace les valeurs invalides par 0.
#[inline]
pub fn sanitize(x: f32) -> f32 {
    if x.is_finite() {
        x
    } else {
        0.0
    }
}
