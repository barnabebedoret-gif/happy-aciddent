//! Version autonome du RISE (tests d'interface) : cargo run --release --features standalone --bin standalone_rise
use happy_accident::rise::HappyAccidentRise;
use nih_plug::prelude::*;

fn main() {
    nih_export_standalone::<HappyAccidentRise>();
}
