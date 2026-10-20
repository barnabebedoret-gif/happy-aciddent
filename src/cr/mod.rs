//! Happy Accident CALL & RESPONSE - by BRNB.
//! Une instance par piste ; une seule lettre parle à la fois.

mod editor;
pub mod engine;

use crate::engine::{Hist, Transport};
use engine::{init_group, load_group, store_group, CrGate, CrSettings};
use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock};

#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Letter {
    A,
    B,
    C,
    D,
}

impl Letter {
    pub fn index(self) -> u8 {
        match self {
            Letter::A => 0,
            Letter::B => 1,
            Letter::C => 2,
            Letter::D => 3,
        }
    }
    pub fn from_index(i: u8) -> Letter {
        [Letter::A, Letter::B, Letter::C, Letter::D][i as usize % 4]
    }
}

pub type CrHistory = Hist<u64>;

#[derive(Params)]
pub struct CrParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,
    /// Historique des réglages (le courant est sauvegardé avec le projet)
    #[persist = "cr-history"]
    pub history: Arc<RwLock<CrHistory>>,

    #[id = "letter"]
    pub letter: EnumParam<Letter>,
    #[id = "group"]
    pub group: IntParam,
    #[id = "depth"]
    pub depth: FloatParam,
    #[id = "fade"]
    pub fade: FloatParam,
    #[id = "chaos"]
    pub chaos: FloatParam,

    #[id = "accident"]
    pub accident: BoolParam,
    #[id = "mutate"]
    pub mutate: BoolParam,
}

impl Default for CrParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(720, 500),
            history: Arc::new(RwLock::new(CrHistory::default())),
            letter: EnumParam::new("Letter", Letter::A),
            group: IntParam::new("Group", 1, IntRange::Linear { min: 1, max: 4 }),
            depth: FloatParam::new("Depth", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            fade: FloatParam::new("Fade", 6.0, FloatRange::Skewed { min: 1.0, max: 80.0, factor: FloatRange::skew_factor(-1.5) })
                .with_unit(" ms")
                .with_value_to_string(formatters::v2s_f32_rounded(1)),
            chaos: FloatParam::new("Chaos", 0.6, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            accident: BoolParam::new("ACCIDENT", false),
            mutate: BoolParam::new("MUTATE", false),
        }
    }
}

impl CrParams {
    pub fn group_index(&self) -> usize {
        (self.group.value() - 1).clamp(0, 3) as usize
    }

    /// Réglages en vigueur pour le groupe de cette instance.
    pub fn current(&self) -> CrSettings {
        let g = self.group_index();
        load_group(g).unwrap_or_else(|| {
            let own = self.history.read().ok().and_then(|h| CrSettings::unpack(h.current())).unwrap_or_default();
            init_group(g, &own)
        })
    }

    /// Applique de nouveaux réglages à tout le groupe et les ajoute à l'historique.
    pub fn commit(&self, s: CrSettings) {
        store_group(self.group_index(), &s);
        if let Ok(mut h) = self.history.write() {
            h.push(s.pack());
        }
    }
}

pub struct CrShared {
    /// Case jouée en ce moment (pour la tête de lecture de l'interface)
    pub slot: AtomicU32,
}

pub struct HappyAccidentCr {
    params: Arc<CrParams>,
    shared: Arc<CrShared>,
    gate: Option<CrGate>,
    rng_state: u64,
    last_accident: bool,
    last_mutate: bool,
}

impl Default for HappyAccidentCr {
    fn default() -> Self {
        Self {
            params: Arc::new(CrParams::default()),
            shared: Arc::new(CrShared { slot: AtomicU32::new(0) }),
            gate: None,
            rng_state: 0,
            last_accident: false,
            last_mutate: false,
        }
    }
}

impl Plugin for HappyAccidentCr {
    const NAME: &'static str = "Happy Accident CALL & RESPONSE";
    const VENDOR: &'static str = "BRNB";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.shared.clone())
    }

    fn initialize(&mut self, _layout: &AudioIOLayout, buffer_config: &BufferConfig, _context: &mut impl InitContext<Self>) -> bool {
        self.gate = Some(CrGate::new(buffer_config.sample_rate));
        self.rng_state = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0xC411);
        // Si le groupe est vide, on y met les réglages sauvegardés de cette instance
        let eff = self.params.current();
        if let Ok(mut h) = self.params.history.write() {
            if h.current() == 0 {
                let pos = h.pos as usize;
                h.patches[pos] = eff.pack();
            }
        }
        true
    }

    fn reset(&mut self) {
        if let Some(g) = self.gate.as_mut() {
            g.reset();
        }
    }

    fn process(&mut self, buffer: &mut Buffer, _aux: &mut AuxiliaryBuffers, context: &mut impl ProcessContext<Self>) -> ProcessStatus {
        let Some(gate) = self.gate.as_mut() else {
            return ProcessStatus::Normal;
        };
        let group = self.params.group_index();
        let mut s = match load_group(group) {
            Some(s) => s,
            None => {
                let own = self.params.history.try_read().ok().and_then(|h| CrSettings::unpack(h.current())).unwrap_or_default();
                init_group(group, &own)
            }
        };

        // Déclencheurs mappables (front montant)
        let acc = self.params.accident.value();
        let mt = self.params.mutate.value();
        let mut changed = None;
        if acc && !self.last_accident {
            self.rng_state = crate::engine::rng::mix(self.rng_state.wrapping_add(0x9E37_79B9_7F4A_7C15));
            changed = Some(s.accident((self.rng_state >> 40) as u32, self.params.chaos.value()));
        } else if mt && !self.last_mutate {
            changed = Some(s.mutate());
        }
        self.last_accident = acc;
        self.last_mutate = mt;
        if let Some(n) = changed {
            store_group(group, &n);
            s = n;
        }

        // L'historique de cette instance suit les changements faits depuis n'importe quelle instance
        let packed = s.pack();
        let stale = self.params.history.try_read().map(|h| h.current() != packed).unwrap_or(false);
        if stale {
            if let Ok(mut h) = self.params.history.try_write() {
                if h.current() != packed {
                    h.push(packed);
                }
            }
        }

        let tr = context.transport();
        let transport = Transport { tempo: tr.tempo.unwrap_or(120.0), playing: tr.playing, pos_beats: tr.pos_beats() };
        let letter = self.params.letter.value().index();
        let channels = buffer.as_slice();
        if channels.len() >= 2 {
            let (l, r) = channels.split_at_mut(1);
            gate.process(l[0], r[0], &s, letter, self.params.depth.value(), self.params.fade.value(), &transport);
        }
        self.shared.slot.store(gate.slot as u32, Ordering::Relaxed);
        ProcessStatus::Normal
    }
}

impl ClapPlugin for HappyAccidentCr {
    const CLAP_ID: &'static str = "com.brnb.happy-accident-call-response";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Call & response between several tracks");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo, ClapFeature::Utility];
}

impl Vst3Plugin for HappyAccidentCr {
    const VST3_CLASS_ID: [u8; 16] = *b"BRNBHappyAccCR01";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}
