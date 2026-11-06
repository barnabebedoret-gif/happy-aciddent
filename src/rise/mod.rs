//! Happy Accident RISE - by BRNB : un gros bouton TENSION pour la montée avant le drop.

mod editor;
pub mod engine;

use engine::{Amount, Curve, RiseEngine, RiseMacros, Tail, DEFAULT_ON, NUM_INGREDIENTS};
use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use crate::engine::Transport;

#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug)]
pub enum CurveParam {
    Linear,
    Late,
}

#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug)]
pub enum AmountParam {
    Subtle,
    Normal,
    Extreme,
}

#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug)]
pub enum TailParam {
    Cut,
    Keep,
}

/// Ce que le moteur renvoie à l'interface
pub struct RiseShared {
    pub shaped: AtomicU32,
    pub gap: AtomicBool,
}

#[derive(Params)]
pub struct RiseParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    /// LE réglage à automatiser
    #[id = "tension"]
    pub tension: FloatParam,

    #[id = "curve"]
    pub curve: EnumParam<CurveParam>,
    #[id = "amount"]
    pub amount: EnumParam<AmountParam>,
    #[id = "tail"]
    pub tail: EnumParam<TailParam>,
    #[id = "output"]
    pub output: FloatParam,

    #[id = "on_lowcut"]
    pub on_lowcut: BoolParam,
    #[id = "on_space"]
    pub on_space: BoolParam,
    #[id = "on_width"]
    pub on_width: BoolParam,
    #[id = "on_noise"]
    pub on_noise: BoolParam,
    #[id = "on_roll"]
    pub on_roll: BoolParam,
    #[id = "on_drive"]
    pub on_drive: BoolParam,
    #[id = "on_pitch"]
    pub on_pitch: BoolParam,
    #[id = "on_gap"]
    pub on_gap: BoolParam,
}

impl Default for RiseParams {
    fn default() -> Self {
        let b = |name: &str, i: usize| BoolParam::new(name, DEFAULT_ON[i]);
        Self {
            editor_state: EguiState::from_size(600, 600),
            tension: FloatParam::new("Tension", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            curve: EnumParam::new("Curve", CurveParam::Late),
            amount: EnumParam::new("Amount", AmountParam::Normal),
            tail: EnumParam::new("Tail", TailParam::Cut),
            output: FloatParam::new(
                "Output",
                util::db_to_gain(0.0),
                FloatRange::Skewed { min: util::db_to_gain(-24.0), max: util::db_to_gain(6.0), factor: FloatRange::gain_skew_factor(-24.0, 6.0) },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(1))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            on_lowcut: b("Low Cut", 0),
            on_space: b("Space", 1),
            on_width: b("Width", 2),
            on_noise: b("Noise", 3),
            on_roll: b("Roll", 4),
            on_drive: b("Drive", 5),
            on_pitch: b("Pitch", 6),
            on_gap: b("Gap", 7),
        }
    }
}

impl RiseParams {
    pub fn ingredients(&self) -> [&BoolParam; NUM_INGREDIENTS] {
        [&self.on_lowcut, &self.on_space, &self.on_width, &self.on_noise, &self.on_roll, &self.on_drive, &self.on_pitch, &self.on_gap]
    }

    fn macros(&self) -> RiseMacros {
        let ing = self.ingredients();
        let mut on = [false; NUM_INGREDIENTS];
        for (o, p) in on.iter_mut().zip(ing.iter()) {
            *o = p.value();
        }
        RiseMacros {
            tension: self.tension.value(),
            curve: match self.curve.value() {
                CurveParam::Linear => Curve::Linear,
                CurveParam::Late => Curve::Late,
            },
            amount: match self.amount.value() {
                AmountParam::Subtle => Amount::Subtle,
                AmountParam::Normal => Amount::Normal,
                AmountParam::Extreme => Amount::Extreme,
            },
            tail: match self.tail.value() {
                TailParam::Cut => Tail::Cut,
                TailParam::Keep => Tail::Keep,
            },
            on,
            out_gain: self.output.value(),
        }
    }
}

pub struct HappyAccidentRise {
    params: Arc<RiseParams>,
    shared: Arc<RiseShared>,
    engine: Option<RiseEngine>,
}

impl Default for HappyAccidentRise {
    fn default() -> Self {
        Self {
            params: Arc::new(RiseParams::default()),
            shared: Arc::new(RiseShared { shaped: AtomicU32::new(0), gap: AtomicBool::new(false) }),
            engine: None,
        }
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
        self.engine = Some(RiseEngine::new(buffer_config.sample_rate, 0x5153_2026));
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
        let st = engine.state();
        self.shared.shaped.store(st.shaped.to_bits(), Ordering::Relaxed);
        self.shared.gap.store(st.gap_active, Ordering::Relaxed);
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
