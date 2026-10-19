//! Petit générateur pseudo-aléatoire déterministe (SplitMix64).
//! Déterministe = un même seed redonne exactement le même accident.

#[derive(Clone, Copy, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix(self.0)
    }

    /// Uniforme dans [0, 1)
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Entier uniforme dans [0, n)
    pub fn below(&mut self, n: u32) -> u32 {
        ((self.f32() * n as f32) as u32).min(n.saturating_sub(1))
    }

    pub fn chance(&mut self, p: f32) -> bool {
        self.f32() < p
    }

    pub fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len() as u32) as usize]
    }
}

#[inline]
pub fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Valeur pseudo-aléatoire stable dans [0, 1) pour un couple (seed, n).
#[inline]
pub fn hash01(seed: u64, n: u64) -> f32 {
    (mix(seed ^ mix(n.wrapping_add(0x51_7CC1_B727_220A))) >> 40) as f32 / (1u64 << 24) as f32
}
