//! Version autonome (pour tester l'interface sans DAW) : cargo run --release --features standalone --bin standalone
use happy_accident::HappyAccident;
use nih_plug::prelude::*;

fn main() {
    nih_export_standalone::<HappyAccident>();
}
