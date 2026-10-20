//! Happy Accident DUB - by BRNB : delay dub filtré, aléatoire.

pub mod engine;
mod editor;

use engine::{DubEngine, DubHistory, DubMacros};
use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use crate::engine::Transport;

pub struct DubShared {
    pub accident: AtomicU32,
    pub mutate: AtomicU32,
    pub prev: AtomicU32,
    pub next: AtomicU32,
    pub display: Mutex<DubHistory>,
}

#[derive(Params)]
pub struct DubParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,
    #[persist = "history"]
    pub history: Arc<RwLock<DubHistory>>,

    #[id = "chaos"]
    pub chaos: FloatParam,
    #[id = "feedback"]
    pub feedback: FloatParam,
    #[id = "mix"]
    pub mix: FloatParam,
    #[id = "movement"]
    pub movement: FloatParam,
    #[id = "output"]
    pub output: FloatParam,

    /// Feedback au maximum tant que c'est enfoncé (à mapper sur un pad)
    #[id = "throw"]
    pub throw: BoolParam,

    #[id = "lock_time"]
    pub lock_time: BoolParam,
    #[id = "lock_send"]
    pub lock_send: BoolParam,
    #[id = "lock_filter"]
    pub lock_filter: BoolParam,
    #[id = "lock_color"]
    pub lock_color: BoolParam,

    #[id = "accident"]
    pub accident: BoolParam,
    #[id = "mutate"]
    pub mutate: BoolParam,
}

fn percent(name: &str, default: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min: 0.0, max: 1.0 })
        .with_unit(" %")
        .with_value_to_string(formatters::v2s_f32_percentage(0))
        .with_string_to_value(formatters::s2v_f32_percentage())
}

impl Default for DubParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(720, 470),
            history: Arc::new(RwLock::new(DubHistory::default())),
            chaos: percent("Chaos", 0.6),
            feedback: FloatParam::new("Feedback", 0.75, FloatRange::Linear { min: 0.0, max: 0.97 })
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            mix: percent("Echoes", 0.6),
            movement: percent("Movement", 0.5),
            output: FloatParam::new(
                "Output",
                util::db_to_gain(0.0),
                FloatRange::Skewed { min: util::db_to_gain(-24.0), max: util::db_to_gain(6.0), factor: FloatRange::gain_skew_factor(-24.0, 6.0) },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(1))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            throw: BoolParam::new("THROW", false),
            lock_time: BoolParam::new("Lock Time", false),
            lock_send: BoolParam::new("Lock Send", false),
            lock_filter: BoolParam::new("Lock Filter", false),
            lock_color: BoolParam::new("Lock Color", false),
            accident: BoolParam::new("ACCIDENT", false),
            mutate: BoolParam::new("MUTATE", false),
        }
    }
}

impl DubParams {
    pub fn lock_params(&self) -> [&BoolParam; 4] {
        [&self.lock_time, &self.lock_send, &self.lock_filter, &self.lock_color]
    }

    fn macros(&self) -> DubMacros {
        let l = self.lock_params();
        DubMacros {
            chaos: self.chaos.value(),
            feedback: self.feedback.value(),
            mix: self.mix.value(),
            movement: self.movement.value(),
            out_gain: self.output.value(),
            throw: self.throw.value(),
            locks: [l[0].value(), l[1].value(), l[2].value(), l[3].value()],
        }
    }
}

pub struct HappyAccidentDub {
    params: Arc<DubParams>,
    shared: Arc<DubShared>,
    engine: Option<DubEngine>,
    seen: [u32; 4],
    last_accident: bool,
    last_mutate: bool,
    dirty: bool,
}

impl Default for HappyAccidentDub {
    fn default() -> Self {
        Self {
            params: Arc::new(DubParams::default()),
            shared: Arc::new(DubShared {
                accident: AtomicU32::new(0),
                mutate: AtomicU32::new(0),
                prev: AtomicU32::new(0),
                next: AtomicU32::new(0),
                display: Mutex::new(DubHistory::default()),
            }),
            engine: None,
            seen: [0; 4],
            last_accident: false,
            last_mutate: false,
            dirty: true,
        }
    }
}

impl Plugin for HappyAccidentDub {
    const NAME: &'static str = "Happy Accident DUB";
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
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0xD0B);
        let mut engine = DubEngine::new(buffer_config.sample_rate, seed);
        if let Ok(h) = self.params.history.read() {
            engine.load_history(*h);
        }
        self.engine = Some(engine);
        self.seen = [
            self.shared.accident.load(Ordering::Relaxed),
            self.shared.mutate.load(Ordering::Relaxed),
            self.shared.prev.load(Ordering::Relaxed),
            self.shared.next.load(Ordering::Relaxed),
        ];
        self.dirty = true;
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

        let counters = [&self.shared.accident, &self.shared.mutate, &self.shared.prev, &self.shared.next];
        for (k, c) in counters.iter().enumerate() {
            let v = c.load(Ordering::Relaxed);
            if v != self.seen[k] {
                self.seen[k] = v;
                match k {
                    0 => engine.accident(&macros),
                    1 => engine.mutate(&macros),
                    2 => engine.history_prev(),
                    _ => engine.history_next(),
                }
                self.dirty = true;
            }
        }
        let acc = self.params.accident.value();
        if acc && !self.last_accident {
            engine.accident(&macros);
            self.dirty = true;
        }
        self.last_accident = acc;
        let mt = self.params.mutate.value();
        if mt && !self.last_mutate {
            engine.mutate(&macros);
            self.dirty = true;
        }
        self.last_mutate = mt;

        let tr = context.transport();
        let transport = Transport { tempo: tr.tempo.unwrap_or(120.0), playing: tr.playing, pos_beats: tr.pos_beats() };
        let channels = buffer.as_slice();
        if channels.len() >= 2 {
            let (l, r) = channels.split_at_mut(1);
            engine.process(l[0], r[0], &macros, &transport);
        }

        if self.dirty {
            let h = *engine.history();
            let mut ok = false;
            if let Ok(mut d) = self.shared.display.try_lock() {
                *d = h;
                ok = true;
            }
            if let Ok(mut p) = self.params.history.try_write() {
                *p = h;
            } else {
                ok = false;
            }
            self.dirty = !ok;
        }
        // Les échos continuent après l'arrêt du son
        ProcessStatus::KeepAlive
    }
}

impl ClapPlugin for HappyAccidentDub {
    const CLAP_ID: &'static str = "com.brnb.happy-accident-dub";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Delay dub filtré aléatoire");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo, ClapFeature::Delay, ClapFeature::Filter];
}

impl Vst3Plugin for HappyAccidentDub {
    const VST3_CLASS_ID: [u8; 16] = *b"BRNBHappyAccDub1";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx, Vst3SubCategory::Delay];
}
