use happy_accident::cr::engine::{init_group, load_group, store_group, CrGate, CrSettings, SILENT};
use happy_accident::engine::Transport;

fn all_settings() -> Vec<CrSettings> {
    let mut v = Vec::new();
    let mut s = CrSettings::default();
    for seed in 0..40u32 {
        for voices in 2..=4u8 {
            s = s.accident(seed * 7919 + 13, 1.0);
            s.voices = voices;
            s.silence = (seed % 16) as u8;
            if seed % 3 == 0 {
                s = s.mutate().mutate();
            }
            v.push(s);
        }
    }
    v
}

#[test]
fn pack_unpack_et_determinisme() {
    for s in all_settings() {
        assert_eq!(CrSettings::unpack(s.pack()), Some(s));
        assert_eq!(s.pattern(), s.pattern());
    }
    assert_eq!(CrSettings::unpack(0), None);
}

#[test]
fn chaque_voix_parle_et_rien_hors_limites() {
    for s in all_settings() {
        let p = s.pattern();
        let len = s.len();
        for v in 0..s.voices {
            assert!(p[..len].contains(&v), "voix {v} absente : {:?} ({:?})", &p[..len], s);
        }
        assert!(p[..len].iter().all(|&x| x == SILENT || x < s.voices));
        assert_ne!(p[0], SILENT);
    }
}

#[test]
fn une_seule_lettre_a_la_fois() {
    let sr = 48000.0;
    for s in all_settings().into_iter().step_by(7) {
        let voices = s.voices as usize;
        let mut gates: Vec<CrGate> = (0..voices).map(|_| CrGate::new(sr)).collect();
        let n = (sr as f64 * 60.0 / 128.0 * s.turn_beats() * s.len() as f64) as usize + 1000;
        let mut outs = vec![vec![0.0f32; n]; voices];
        for (k, g) in gates.iter_mut().enumerate() {
            let mut l = vec![1.0f32; n];
            let mut r = vec![1.0f32; n];
            let mut pos = 0.0;
            for (cl, cr) in l.chunks_mut(512).zip(r.chunks_mut(512)) {
                let t = Transport { tempo: 128.0, playing: true, pos_beats: Some(pos) };
                g.process(cl, cr, &s, k as u8, 1.0, 2.0, &t);
                pos += cl.len() as f64 * 128.0 / 60.0 / sr as f64;
            }
            outs[k] = l;
        }
        // Après la mise en place initiale, la somme des pistes ne dépasse jamais 1
        for i in 2000..n {
            let sum: f32 = (0..voices).map(|k| outs[k][i]).sum();
            assert!(sum <= 1.01, "chevauchement {sum} à l'échantillon {i} ({:?})", s);
        }
    }
}

#[test]
fn les_instances_partagent_le_groupe() {
    let a = CrSettings::default().accident(1234, 1.0);
    let got = init_group(3, &a);
    store_group(3, &got);
    let b = got.mutate();
    store_group(3, &b);
    assert_eq!(load_group(3), Some(b));
    // Un groupe déjà initialisé n'est pas écrasé par une autre instance qui arrive
    let c = CrSettings::default().accident(999, 1.0);
    assert_eq!(init_group(3, &c), b);
}

#[test]
fn variations_live_1_2() {
    use happy_accident::cr::engine::{active_variation, save_slot, select_variation, slot, snapshot_vars, VAR_LIVE};
    let g = 2;
    let a = CrSettings::default().accident(111, 1.0);
    store_group(g, &a);
    assert_eq!(active_variation(g), VAR_LIVE);
    // Une variation vide ne fait rien
    assert!(!select_variation(g, 2));
    assert_eq!(load_group(g), Some(a));
    // Sauvegarde dans 1, nouveau pattern, sauvegarde dans 2
    save_slot(g, 0);
    let b = a.accident(222, 1.0);
    store_group(g, &b);
    save_slot(g, 1);
    assert_eq!(slot(g, 0), Some(a));
    assert_eq!(slot(g, 1), Some(b));
    // Rappel de 1 puis 2 : tout le groupe suit
    assert!(select_variation(g, 1));
    assert_eq!(load_group(g), Some(a));
    assert!(select_variation(g, 2));
    assert_eq!(load_group(g), Some(b));
    // ACCIDENT pendant qu'on joue la variation 2 : on repasse en LIVE, la variation 2 est intacte
    let c = b.accident(333, 1.0);
    store_group(g, &c);
    assert_eq!(active_variation(g), VAR_LIVE);
    assert_eq!(slot(g, 1), Some(b));
    // Retour en LIVE après un rappel : on retrouve le pattern de travail
    assert!(select_variation(g, 1));
    assert!(select_variation(g, VAR_LIVE));
    assert_eq!(load_group(g), Some(c));
    // L'état complet est sauvegardable
    let snap = snapshot_vars(g);
    assert_eq!(snap, [c.pack(), a.pack(), b.pack(), 0]);
}

#[test]
fn presets_donnent_les_bons_patterns() {
    use happy_accident::cr::engine::{apply_preset, slot, LETTERS};
    use happy_accident::cr::presets::PRESETS;
    let show = |s: &CrSettings| s.pattern()[..s.len()].iter().map(|&v| if v == SILENT { "." } else { LETTERS[v as usize] }).collect::<String>();
    assert!(PRESETS.len() >= 15);
    for p in PRESETS {
        if !p.expect.is_empty() {
            assert_eq!(show(&p.live), p.expect, "preset {}", p.name);
        }
        for v in [Some(p.live), p.var1, p.var2].into_iter().flatten() {
            assert_eq!(CrSettings::unpack(v.pack()), Some(v), "preset {} non encodable", p.name);
        }
    }
    // Charger un preset avec variations remplit 1 et 2 ; un preset simple ne touche pas les variations
    let g = 1;
    let with = PRESETS.iter().find(|p| p.var1.is_some()).unwrap();
    apply_preset(g, with);
    assert_eq!(load_group(g), Some(with.live));
    assert_eq!(slot(g, 0), with.var1);
    let plain = PRESETS.iter().find(|p| p.var1.is_none()).unwrap();
    apply_preset(g, plain);
    assert_eq!(load_group(g), Some(plain.live));
    assert_eq!(slot(g, 0), with.var1);
}

#[test]
fn etat_recharge_sapplique_au_groupe() {
    use happy_accident::cr::engine::{force_vars, init_vars, snapshot_vars};
    use happy_accident::cr::Watched;
    use nih_plug::params::persist::PersistentField;
    let g = 0;
    let running = CrSettings::default().accident(500, 1.0);
    store_group(g, &running);
    let other = running.accident(600, 1.0);
    let saved = [other.pack(), 0, 0, 0];
    // Nouvelle instance / réinitialisation simple : le groupe qui tourne n'est pas écrasé
    init_vars(g, &saved);
    assert_eq!(load_group(g), Some(running));
    // État rechargé par Ableton : il s'applique à tout le groupe
    force_vars(g, &saved);
    assert_eq!(load_group(g), Some(other));
    assert_eq!(snapshot_vars(g)[0], other.pack());
    // Un état vide ne doit jamais rien effacer
    force_vars(g, &[0, 0, 0, 0]);
    assert_eq!(load_group(g), Some(other));
    // Le marqueur "rechargé" s'allume quand Ableton restaure la valeur
    let w = Watched::new([0u64; 4]);
    assert!(!w.loaded.load(std::sync::atomic::Ordering::Acquire));
    w.set(saved);
    assert!(w.loaded.load(std::sync::atomic::Ordering::Acquire));
    assert_eq!(w.map(|v| *v), saved);
}
