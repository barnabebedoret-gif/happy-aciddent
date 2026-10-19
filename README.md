# Happy Accident Generator — by BRNB

Multi-FX aléatoire pour transformer des leads et trouver des idées. Format VST3 et CLAP (Windows / macOS), compatible avec Ableton Live 10 et plus récent.

## Obtenir le .vst3 Windows sans rien installer (GitHub)
1. Crée un repo sur github.com, par exemple `happy-accident`, et envoie-y ce dossier (ou glisse les fichiers dans l'interface web « Add file → Upload files », en gardant le dossier `.github`).
2. Ouvre l'onglet **Actions**. La compilation démarre seule, compte environ 10 minutes.
3. Une fois le job **windows** en vert, télécharge l'artefact `HappyAccidentGenerator-windows`.
4. Dézippe-le et copie `Happy Accident Generator.vst3` dans `C:\Program Files\Common Files\VST3\`.
5. Dans Ableton : Préférences → Plug-ins → activer « Use VST3 Plug-in System Folders » → Rescan.

## Compiler soi-même (Windows)
1. Installe Rust : https://rustup.rs (accepte les options par défaut, Visual Studio Build Tools inclus).
2. Dans ce dossier : `cargo xtask bundle happy_accident --release`
3. Le plugin se trouve dans `target/bundled/`.

## Utilisation
- **ACCIDENT** : nouveau tirage. **Chaos** règle à quel point il est radical.
- **MUTATE** : petite variation de l'accident en cours.
- **◀ ▶** : retour aux 16 derniers accidents. L'historique est sauvegardé avec ton projet Ableton.
- **Timbre ↔ Pattern** : oriente le hasard vers le son (drive, résonateur…) ou vers le rythme et la mélodie (gate, stutter, pitch en séquence).
- **Movement** : quantité de modulation synchronisée au tempo.
- **Tonalité / Gamme** : le pitch et le résonateur restent dans la gamme.
- **Lock** sur chaque module : il ne bouge plus aux prochains tirages.
- ACCIDENT et MUTATE sont aussi des paramètres, donc tu peux les mapper sur un pad MIDI (Configure dans Ableton).
- Un limiteur et une compensation de volume protègent la sortie.

Astuce : quand un accident sonne, resample-le (piste audio en entrée Resampling) puis travaille-le comme un nouveau lead.

## Développement
- `cargo test --release` : tests de robustesse (300 accidents, 3 fréquences d'échantillonnage, pas de NaN ni de dépassement).
- `cargo run --release --example render_demos` : génère `demos/happy_accident_demo.wav`.
- Le moteur audio (`src/engine`) ne dépend pas du format plugin.

Licence : GPL-3.0 (imposée par le SDK VST3 utilisé par nih-plug).
