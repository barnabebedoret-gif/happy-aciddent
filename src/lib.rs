//! Happy Accident Generator - by BRNB
//! Multi-FX aléatoire pour transformer des leads et générer des idées.

use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, RwLock};

pub mod cr;
pub mod dub;
mod editor;
pub mod engine;

use engine::{Engine, History, Macros, Scale, Transport, NUM_MODULES};

#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug)]
pub enum ScaleParam {
    #[name = "Mineur"]
    Minor,
    #[name = "Majeur"]
    Major,
    #[name = "Dorien"]
    Dorian,
    #[name = "Phrygien"]
    Phrygian,
    #[name = "Mineur harm."]
    HarmonicMinor,
    #[name = "Penta mineure"]
    PentaMinor,
}

impl From<ScaleParam> for Scale {
    fn from(s: ScaleParam) -> Scale {
        match s {
            ScaleParam::Minor => Scale::Minor,
            ScaleParam::Major => Scale::Major,
            ScaleParam::Dorian => Scale::Dorian,
            ScaleParam::Phrygian => Scale::Phrygian,
            ScaleParam::HarmonicMinor => Scale::HarmonicMinor,
            ScaleParam::PentaMinor => Scale::PentaMinor,
        }
    }
}

#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug)]
pub enum SpeedParam {
    #[name = "1/4"]
    Quarter,
    #[name = "1/2"]
    Half,
    #[name = "1×"]
    Normal,
    #[name = "2×"]
    Double,
    #[name = "4×"]
    Quad,
}

impl SpeedParam {
    pub fn factor(self) -> f32 {
        match self {
            SpeedParam::Quarter => 0.25,
            SpeedParam::Half => 0.5,
            SpeedParam::Normal => 1.0,
            SpeedParam::Double => 2.0,
            SpeedParam::Quad => 4.0,
        }
    }
}

pub const NOTE_NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

/// Communication interface <-> thread audio (sans verrou côté audio).
pub struct Shared {
    pub accident: AtomicU32,
    pub mutate: AtomicU32,
    pub prev: AtomicU32,
    pub next: AtomicU32,
    /// Copie de l'état pour l'affichage
    pub display: Mutex<History>,
}

#[derive(Params)]
pub struct HaParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    /// Historique des accidents, sauvegardé avec le projet Ableton
    #[persist = "history"]
    pub history: Arc<RwLock<History>>,

    #[id = "chaos"]
    pub chaos: FloatParam,
    #[id = "balance"]
    pub balance: FloatParam,
    #[id = "movement"]
    pub movement: FloatParam,
    #[id = "mix"]
    pub mix: FloatParam,
    #[id = "output"]
    pub output: FloatParam,

    #[id = "key"]
    pub key: IntParam,
    #[id = "scale"]
    pub scale: EnumParam<ScaleParam>,
    #[id = "speed"]
    pub speed: EnumParam<SpeedParam>,

    #[id = "lock0"]
    pub lock0: BoolParam,
    #[id = "lock1"]
    pub lock1: BoolParam,
    #[id = "lock2"]
    pub lock2: BoolParam,
    #[id = "lock3"]
    pub lock3: BoolParam,
    #[id = "lock4"]
    pub lock4: BoolParam,
    #[id = "lock5"]
    pub lock5: BoolParam,

    /// Déclencheurs mappables en MIDI dans Ableton
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

impl Default for HaParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(720, 520),
            history: Arc::new(RwLock::new(History::default())),
            chaos: percent("Chaos", 0.6),
            balance: FloatParam::new("Timbre <> Pattern", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(Arc::new(|v| {
                    if v < 0.34 {
                        format!("Timbre {:.0}%", (1.0 - v) * 100.0)
                    } else if v > 0.66 {
                        format!("Pattern {:.0}%", v * 100.0)
                    } else {
                        "Les deux".to_string()
                    }
                })),
            movement: percent("Movement", 0.4),
            mix: percent("Mix", 1.0),
            output: FloatParam::new(
                "Output",
                util::db_to_gain(0.0),
                FloatRange::Skewed { min: util::db_to_gain(-24.0), max: util::db_to_gain(6.0), factor: FloatRange::gain_skew_factor(-24.0, 6.0) },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(1))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            key: IntParam::new("Tonalité", 9, IntRange::Linear { min: 0, max: 11 })
                .with_value_to_string(Arc::new(|v| NOTE_NAMES[v.clamp(0, 11) as usize].to_string())),
            scale: EnumParam::new("Gamme", ScaleParam::Minor),
            speed: EnumParam::new("Speed", SpeedParam::Normal),
            lock0: BoolParam::new("Lock Gate", false),
            lock1: BoolParam::new("Lock Stutter", false),
            lock2: BoolParam::new("Lock Pitch", false),
            lock3: BoolParam::new("Lock Drive", false),
            lock4: BoolParam::new("Lock Filter", false),
            lock5: BoolParam::new("Lock Space", false),
            accident: BoolParam::new("ACCIDENT", false),
            mutate: BoolParam::new("MUTATE", false),
        }
    }
}

impl HaParams {
    pub fn lock_params(&self) -> [&BoolParam; NUM_MODULES] {
        [&self.lock0, &self.lock1, &self.lock2, &self.lock3, &self.lock4, &self.lock5]
    }

    fn macros(&self) -> Macros {
        let l = self.lock_params();
        Macros {
            chaos: self.chaos.value(),
            balance: self.balance.value(),
            movement: self.movement.value(),
            mix: self.mix.value(),
            out_gain: self.output.value(),
            key: self.key.value() as u8,
            scale: self.scale.value().into(),
            speed: self.speed.value().factor(),
            locks: [l[0].value(), l[1].value(), l[2].value(), l[3].value(), l[4].value(), l[5].value()],
        }
    }
}

pub struct HappyAccident {
    params: Arc<HaParams>,
    shared: Arc<Shared>,
    engine: Option<Engine>,
    seen: [u32; 4],
    last_accident_param: bool,
    last_mutate_param: bool,
    dirty: bool,
}

impl Default for HappyAccident {
    fn default() -> Self {
        Self {
            params: Arc::new(HaParams::default()),
            shared: Arc::new(Shared {
                accident: AtomicU32::new(0),
                mutate: AtomicU32::new(0),
                prev: AtomicU32::new(0),
                next: AtomicU32::new(0),
                display: Mutex::new(History::default()),
            }),
            engine: None,
            seen: [0; 4],
            last_accident_param: false,
            last_mutate_param: false,
            dirty: true,
        }
    }
}

impl Plugin for HappyAccident {
    const NAME: &'static str = "Happy Accident Generator";
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
            .unwrap_or(0xB27B);
        let mut engine = Engine::new(buffer_config.sample_rate, seed);
        if let Ok(h) = self.params.history.read() {
            engine.load_history(*h);
        }
        self.engine = Some(engine);
        // On ignore les clics faits avant l'initialisation
        self.seen = [
            self.shared.accident.load(Ordering::Relaxed),
            self.shared.mutate.load(Ordering::Relaxed),
            self.shared.prev.load(Ordering::Relaxed),
            self.shared.next.load(Ordering::Relaxed),
        ];
        self.dirty = true;
        true
    }

    fn process(&mut self, buffer: &mut Buffer, _aux: &mut AuxiliaryBuffers, context: &mut impl ProcessContext<Self>) -> ProcessStatus {
        let Some(engine) = self.engine.as_mut() else {
            return ProcessStatus::Normal;
        };
        let macros = self.params.macros();

        // Commandes venant de l'interface
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

        // Déclencheurs automatisables / mappables (front montant)
        let acc = self.params.accident.value();
        if acc && !self.last_accident_param {
            engine.accident(&macros);
            self.dirty = true;
        }
        self.last_accident_param = acc;
        let mt = self.params.mutate.value();
        if mt && !self.last_mutate_param {
            engine.mutate(&macros);
            self.dirty = true;
        }
        self.last_mutate_param = mt;

        let tr = context.transport();
        let transport = Transport { tempo: tr.tempo.unwrap_or(120.0), playing: tr.playing, pos_beats: tr.pos_beats() };

        let channels = buffer.as_slice();
        if channels.len() >= 2 {
            let (l, r) = channels.split_at_mut(1);
            engine.process(l[0], r[0], &macros, &transport);
        }

        // Publication de l'état (sans jamais bloquer le thread audio)
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

        ProcessStatus::Normal
    }
}

impl ClapPlugin for HappyAccident {
    const CLAP_ID: &'static str = "com.brnb.happy-accident-generator";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Multi-FX aléatoire pour générer des idées de leads");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo, ClapFeature::Distortion, ClapFeature::Glitch];
}

impl Vst3Plugin for HappyAccident {
    const VST3_CLASS_ID: [u8; 16] = *b"BRNBHappyAccdnt1";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx, Vst3SubCategory::Distortion];
}

nih_export_clap!(HappyAccident, dub::HappyAccidentDub, cr::HappyAccidentCr);
nih_export_vst3!(HappyAccident, dub::HappyAccidentDub, cr::HappyAccidentCr);
