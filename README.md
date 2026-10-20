# Happy Accident Generator — by BRNB

Multi-FX aléatoire pour transformer des leads et trouver des idées. Format VST3 et CLAP (Windows / macOS), compatible avec Ableton Live 10 et plus récent.

## Distribuer le plugin
- Chaque push sur GitHub produit l'artefact **HappyAccidentGenerator-windows**, qui contient l'installeur `.exe` et un zip manuel, ainsi qu'un zip macOS bêta.
- Pour publier une version : change `version` dans `Cargo.toml`, puis crée un tag, par exemple `git tag v0.1.0 && git push --tags`. GitHub prépare alors une **Release en brouillon** avec tous les fichiers. Il reste à la relire et à cliquer sur Publish.
- La page d'installation pour les utilisateurs (FR / EN) est dans `INSTALL.md`.
- Licence GPLv3 (voir `LICENSE`) : si tu distribues le plugin, le code source doit être disponible.

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
- **Timbre ↔ Pattern** : oriente le hasard vers le son (filtre, drive…) ou vers le rythme et la mélodie (gate, stutter, pitch en séquence).
- **Movement** : quantité de modulation synchronisée au tempo (sur le filtre : petits sauts aléatoires à chaque pas).
- **Speed** : 1/4, 1/2, 1×, 2× ou 4×. Accélère ou ralentit tous les rythmes (gate, stutter, filtre, mélodie) ; les delays restent calés sur le vrai tempo.
- **Tonalité / Gamme** : le pitch reste dans la gamme.
- **Lock** sur chaque module : il ne bouge plus aux prochains tirages.
- ACCIDENT et MUTATE sont aussi des paramètres, donc tu peux les mapper sur un pad MIDI (Configure dans Ableton).
- Un limiteur et une compensation de volume protègent la sortie.

Astuce : quand un accident sonne, resample-le (piste audio en entrée Resampling) puis travaille-le comme un nouveau lead.

## Développement
- `cargo test --release` : tests de robustesse (300 accidents, 3 fréquences d'échantillonnage, pas de NaN ni de dépassement).
- `cargo run --release --example render_demos` : génère `demos/happy_accident_demo.wav`.
- Le moteur audio (`src/engine`) ne dépend pas du format plugin.

Licence : GPL-3.0 (imposée par le SDK VST3 utilisé par nih-plug).

## Happy Accident DUB (2e plugin, même installeur)
Delay dub filtré et aléatoire. Le son sec passe intact et seuls les échos sont transformés.
- **ACCIDENT / MUTATE / ◀ ▶** : comme dans le Generator.
- **4 groupes verrouillables** :
  - **TIME** : 1/16 à 1/4D, ping-pong, temps gauche et droite différents.
  - **SEND** : envoi continu, ou « throws » où seuls certains pas partent dans le delay.
  - **FILTER** : filtre dans la boucle (bande dub, BP, HP, LP), fixe, différent à chaque écho, ou en escalier.
  - **COLOR** : saturation et wobble de bande.
- **Feedback**, **Echoes** (niveau des échos), **Movement**.
- **THROW** : maintiens le bouton pour pousser le feedback au-delà de 100 % et lancer la spirale dub. La saturation borne le niveau. Il est mappable sur un pad.
- Quand l'accident change le temps de delay, la tête glisse comme une bande : c'est voulu.
