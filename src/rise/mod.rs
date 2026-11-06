//! Happy Accident RISE - by BRNB : un gros bouton TENSION pour la montée avant le drop,
//! plus 3 dosages (CUT, SPACE, ROLL).

mod editor;
pub mod engine;

use engine::{RiseEngine, RiseMacros};
use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use crate::engine::Transport;

/// Ce que le moteur renvoie à l'interface
pub struct RiseShared {
    pub shaped: AtomicU32,
}

#[derive(Params)]
pub struct RiseParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    /// LE réglage à automatiser
    #[id = "tension"]
    pub tension: FloatParam,
    #[id = "cut"]
    pub cut: FloatParam,
    #[id = "space"]
    pub space: FloatParam,
    #[id = "roll"]
    pub roll: FloatParam,
}

fn percent(name: &str, default: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min: 0.0, max: 1.0 })
        .with_unit(" %")
        .with_value_to_string(formatters::v2s_f32_percentage(0))
        .with_string_to_value(formatters::s2v_f32_percentage())
}

impl Default for RiseParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(520, 420),
            tension: percent("Tension", 0.0),
            cut: percent("Cut", 0.5),
            space: percent("Space", 0.5),
            roll: percent("Roll", 0.0),
        }
    }
}

impl RiseParams {
    fn macros(&self) -> RiseMacros {
        RiseMacros { tension: self.tension.value(), cut: self.cut.value(), space: self.space.value(), roll: self.roll.value() }
    }
}

pub struct HappyAccidentRise {
    params: Arc<RiseParams>,
    shared: Arc<RiseShared>,
    engine: Option<RiseEngine>,
}

impl Default for HappyAccidentRise {
    fn default() -> Self {
        Self { params: Arc::new(RiseParams::default()), shared: Arc::new(RiseShared { shaped: AtomicU32::new(0) }), engine: None }
    }
}

impl Plugin for HappyAccidentRise {
    const NAME: &'static str = "Happy Accident RISE";
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
        self.engine = Some(RiseEngine::new(buffer_config.sample_rate));
        true
    }

    fn reset(&mut self) {
        if let Some(e) = self.engine.as_mut() {
            e.clear();
        }
    }

    fn process(&mut self, buffer: &mut Buffer, _aux: &mut AuxiliaryBuffers, context: &mut impl ProcessContext<Self>) -> ProcessStatus {
        let Some(engine) = self.engine.as_mut() else {
            return ProcessStatus::Normal;
        };
        let macros = self.params.macros();
        let tr = context.transport();
        let transport = Transport { tempo: tr.tempo.unwrap_or(120.0), playing: tr.playing, pos_beats: tr.pos_beats() };
        let channels = buffer.as_slice();
        if channels.len() >= 2 {
            let (l, r) = channels.split_at_mut(1);
            engine.process(l[0], r[0], &macros, &transport);
        }
        self.shared.shaped.store(engine.state().shaped.to_bits(), Ordering::Relaxed);
        // La queue de reverb continue après l'arrêt du son
        ProcessStatus::KeepAlive
    }
}

impl ClapPlugin for HappyAccidentRise {
    const CLAP_ID: &'static str = "com.brnb.happy-accident-rise";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("One-knob build-up tension before the drop");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo, ClapFeature::Filter, ClapFeature::Reverb];
}

impl Vst3Plugin for HappyAccidentRise {
    const VST3_CLASS_ID: [u8; 16] = *b"BRNBHappyAccRise";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx, Vst3SubCategory::Filter];
}
