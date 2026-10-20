//! Version autonome du Call & Response (tests d'interface) : cargo run --release --features standalone --bin standalone_cr
use happy_accident::cr::HappyAccidentCr;
use nih_plug::prelude::*;

fn main() {
    nih_export_standalone::<HappyAccidentCr>();
}
