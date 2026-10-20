# Happy Accident Generator — Installation

## 🇫🇷 Français

### Windows (recommandé)
1. Lance **HappyAccidentGenerator-…-Windows-Setup.exe**.
2. Si Windows affiche « Windows a protégé votre ordinateur » : clique sur **Informations complémentaires**, puis **Exécuter quand même**. Ce message s'affiche parce que le plugin est gratuit et pas signé ; il ne signale pas un problème.
3. Suis l'installeur : le plugin est copié dans `C:\Program Files\Common Files\VST3`.
4. Dans **Ableton** : Préférences → **Plug-ins** → active **Use VST3 Plug-in System Folders** → **Rescan**.
5. Navigateur → **Plug-ins → BRNB → Happy Accident Generator**. Glisse-le sur une piste de lead.

Installation manuelle (zip) : copie le dossier `Happy Accident Generator.vst3` dans `C:\Program Files\Common Files\VST3\`, puis fais l'étape 4.

**Ableton Live 10.1 ou plus récent requis** (le VST3 n'est pas supporté en 10.0).

### macOS (bêta)
1. Copie `Happy Accident Generator.vst3` dans `~/Library/Audio/Plug-Ins/VST3/`.
2. Le plugin n'est pas encore notarisé par Apple. Ouvre le **Terminal** et colle :
   `xattr -dr com.apple.quarantine ~/Library/Audio/Plug-Ins/VST3/"Happy Accident Generator.vst3"`
3. Relance Ableton → Préférences → Plug-ins → Rescan.

### Désinstaller
Windows : Paramètres → Applications → « Happy Accident Generator (BRNB) » → Désinstaller.

---

## 🇬🇧 English

### Windows (recommended)
1. Run **HappyAccidentGenerator-…-Windows-Setup.exe**.
2. If Windows says "Windows protected your PC": click **More info**, then **Run anyway**. You see this because the plugin is free and unsigned; it does not mean anything is wrong.
3. Follow the installer: the plugin is copied to `C:\Program Files\Common Files\VST3`.
4. In **Ableton**: Preferences → **Plug-ins** → enable **Use VST3 Plug-in System Folders** → **Rescan**.
5. Browser → **Plug-ins → BRNB → Happy Accident Generator**. Drop it on a lead track.

Manual install (zip): copy the `Happy Accident Generator.vst3` folder to `C:\Program Files\Common Files\VST3\`, then do step 4.

**Requires Ableton Live 10.1 or later** (VST3 is not supported in 10.0).

### macOS (beta)
1. Copy `Happy Accident Generator.vst3` to `~/Library/Audio/Plug-Ins/VST3/`.
2. The plugin is not notarized by Apple yet. Open **Terminal** and paste:
   `xattr -dr com.apple.quarantine ~/Library/Audio/Plug-Ins/VST3/"Happy Accident Generator.vst3"`
3. Restart Ableton → Preferences → Plug-ins → Rescan.

### Uninstall
Windows: Settings → Apps → "Happy Accident Generator (BRNB)" → Uninstall.
