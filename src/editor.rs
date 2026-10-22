//! Interface du plugin (egui).

use crate::engine::{History, KINDS, NUM_MODULES};
use crate::{HaParams, Shared};
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, Color32, RichText, Vec2};
use nih_plug_egui::{create_egui_editor, widgets};
use std::sync::atomic::Ordering;
use std::sync::Arc;

const BG: Color32 = Color32::from_rgb(18, 18, 20);
const PANEL: Color32 = Color32::from_rgb(30, 30, 34);
const PANEL_ON: Color32 = Color32::from_rgb(42, 42, 47);
const ACID: Color32 = Color32::from_rgb(222, 218, 208);
const PINK: Color32 = Color32::from_rgb(208, 168, 110);
const DIM: Color32 = Color32::from_rgb(128, 128, 134);
const TEXT: Color32 = Color32::from_rgb(226, 226, 228);

pub fn create(params: Arc<HaParams>, shared: Arc<Shared>) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        None::<usize>,
        |ctx, _| {
            let mut style = (*ctx.style()).clone();
            style.visuals = egui::Visuals::dark();
            style.visuals.panel_fill = BG;
            style.visuals.selection.bg_fill = ACID.gamma_multiply(0.6);
            style.visuals.widgets.inactive.bg_fill = PANEL;
            style.spacing.item_spacing = Vec2::new(8.0, 6.0);
            ctx.set_style(style);
        },
        move |ctx, setter, chosen| {
            let hist: History = shared.display.lock().map(|h| *h).unwrap_or_default();
            let patch = hist.current();

            egui::CentralPanel::default().show(ctx, |ui| {
                // ---------------- En-tête
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("HAPPY ACCIDENT").size(26.0).strong().color(ACID));
                        ui.label(RichText::new(concat!("GENERATOR  —  by BRNB  ·  v", env!("CARGO_PKG_VERSION"))).size(12.0).color(DIM));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_enabled(hist.pos + 1 < hist.len, egui::Button::new("▶")).on_hover_text("Next accident in the history").clicked() {
                            shared.next.fetch_add(1, Ordering::Relaxed);
                        }
                        ui.label(RichText::new(format!("{}/{}", hist.pos + 1, hist.len)).color(TEXT));
                        if ui.add_enabled(hist.pos > 0, egui::Button::new("◀")).on_hover_text("Previous accident. The last 16 are kept and saved with your Live set.").clicked() {
                            shared.prev.fetch_add(1, Ordering::Relaxed);
                        }
                        ui.label(RichText::new(format!("#{:08X}", patch.seed)).monospace().color(DIM));
                    });
                });
                ui.add_space(8.0);

                // ---------------- Presets d'usine
                ui.horizontal(|ui| {
                    ui.label(RichText::new("PRESET").size(12.0).color(DIM)).on_hover_text(
                        "Factory presets: a full set of effects plus the CHAOS, TIMBRE / PATTERN, MOVEMENT and SPEED settings \
                         and the recommended LOCKs. Load one, then use ACCIDENT and MUTATE to explore around it.",
                    );
                    let current = chosen.map(|i| crate::presets::PRESETS[i].name).unwrap_or("Choose a preset…");
                    egui::ComboBox::from_id_salt("gen_preset").width(280.0).selected_text(current).show_ui(ui, |ui| {
                        for (cat, title) in crate::presets::CATEGORY_NAMES {
                            ui.label(RichText::new(title).size(11.0).strong().color(DIM));
                            for (i, p) in crate::presets::PRESETS.iter().enumerate().filter(|(_, p)| p.category == cat) {
                                if ui.selectable_label(*chosen == Some(i), p.name).on_hover_text(p.tip).clicked() {
                                    let set = |param: &FloatParam, v: f32| {
                                        setter.begin_set_parameter(param);
                                        setter.set_parameter(param, v);
                                        setter.end_set_parameter(param);
                                    };
                                    set(&params.chaos, p.chaos);
                                    set(&params.balance, p.balance);
                                    set(&params.movement, p.movement);
                                    set(&params.mix, 1.0);
                                    let speed = [crate::SpeedParam::Quarter, crate::SpeedParam::Half, crate::SpeedParam::Normal, crate::SpeedParam::Double, crate::SpeedParam::Quad][(p.speed as usize).min(4)];
                                    setter.begin_set_parameter(&params.speed);
                                    setter.set_parameter(&params.speed, speed);
                                    setter.end_set_parameter(&params.speed);
                                    for (k, lp) in params.lock_params().iter().enumerate() {
                                        setter.begin_set_parameter(*lp);
                                        setter.set_parameter(*lp, p.locks[k]);
                                        setter.end_set_parameter(*lp);
                                    }
                                    shared.preset_idx.store(i as u32, Ordering::Relaxed);
                                    shared.preset_cmd.fetch_add(1, Ordering::Relaxed);
                                    *chosen = Some(i);
                                }
                            }
                            ui.add_space(4.0);
                        }
                    });
                    if let Some(i) = *chosen {
                        ui.label(RichText::new("hover for tips").size(11.0).color(DIM)).on_hover_text(crate::presets::PRESETS[i].tip);
                    }
                });
                ui.add_space(6.0);

                // ---------------- Boutons principaux
                ui.horizontal(|ui| {
                    let acc = egui::Button::new(RichText::new("ACCIDENT").size(30.0).strong().color(BG))
                        .fill(ACID)
                        .min_size(Vec2::new(430.0, 64.0));
                    if ui.add(acc).on_hover_text("Roll a brand-new accident: new effects, order and settings. CHAOS sets how wild it gets. Locked modules are kept.").clicked() {
                        shared.accident.fetch_add(1, Ordering::Relaxed);
                    }
                    let mt = egui::Button::new(RichText::new("MUTATE").size(22.0).strong().color(TEXT))
                        .fill(PANEL_ON)
                        .min_size(Vec2::new(250.0, 64.0));
                    if ui.add(mt).on_hover_text("Small variation of the current accident. Great to explore around a sound you like.").clicked() {
                        shared.mutate.fetch_add(1, Ordering::Relaxed);
                    }
                });
                ui.add_space(10.0);

                // ---------------- Macros
                let label = |ui: &mut egui::Ui, t: &str, tip: &str| {
                    ui.label(RichText::new(t).size(12.0).color(DIM)).on_hover_text(tip);
                };
                egui::Grid::new("macros").num_columns(4).min_col_width(60.0).spacing(Vec2::new(12.0, 8.0)).show(ui, |ui| {
                    label(ui, "CHAOS", "How radical ACCIDENT and MUTATE are. Low = small changes around the current sound. High = anything goes.");
                    ui.add(widgets::ParamSlider::for_param(&params.chaos, setter).with_width(160.0)).on_hover_text("How radical ACCIDENT and MUTATE are. Low = small changes around the current sound. High = anything goes.");
                    label(ui, "TIMBRE / PATTERN", "Steers the randomness. Left: tone changes (filter, drive). Right: rhythm and melody changes (gate, stutter, pitch sequences). Middle: both.");
                    ui.add(widgets::ParamSlider::for_param(&params.balance, setter).with_width(160.0)).on_hover_text("Steers the randomness. Left: tone changes (filter, drive). Right: rhythm and melody changes (gate, stutter, pitch sequences). Middle: both.");
                    ui.end_row();
                    label(ui, "MOVEMENT", "Amount of tempo-synced modulation inside the effects. On the filter it adds random jumps on each step (never a wah).");
                    ui.add(widgets::ParamSlider::for_param(&params.movement, setter).with_width(160.0)).on_hover_text("Amount of tempo-synced modulation inside the effects. On the filter it adds random jumps on each step (never a wah).");
                    label(ui, "SPEED", "Plays every rhythm (gate, stutter, filter steps, pitch melody) from 1/4 to 4x speed. Delays stay locked to the real tempo.");
                    ui.add(widgets::ParamSlider::for_param(&params.speed, setter).with_width(160.0)).on_hover_text("Plays every rhythm (gate, stutter, filter steps, pitch melody) from 1/4 to 4x speed. Delays stay locked to the real tempo.");
                    ui.end_row();
                    label(ui, "KEY", "Key and scale. Pitch melodies always stay in this scale.");
                    ui.horizontal(|ui| {
                        ui.add(widgets::ParamSlider::for_param(&params.key, setter).with_width(50.0)).on_hover_text("Key and scale. Pitch melodies always stay in this scale.");
                        ui.add(widgets::ParamSlider::for_param(&params.scale, setter).with_width(60.0)).on_hover_text("Key and scale. Pitch melodies always stay in this scale.");
                    });
                    label(ui, "MIX", "Dry / wet balance. 100% = fully processed.");
                    ui.add(widgets::ParamSlider::for_param(&params.mix, setter).with_width(160.0)).on_hover_text("Dry / wet balance. 100% = fully processed.");
                    ui.end_row();
                    ui.label("");
                    ui.label("");
                    label(ui, "OUTPUT", "Output level, after the automatic gain compensation and the safety limiter.");
                    ui.add(widgets::ParamSlider::for_param(&params.output, setter).with_width(160.0)).on_hover_text("Output level, after the automatic gain compensation and the safety limiter.");
                    ui.end_row();
                });
                ui.add_space(12.0);

                // ---------------- Modules
                let locks = params.lock_params();
                let positions = {
                    let mut pos = [0usize; NUM_MODULES];
                    let mut n = 0;
                    for &mi in patch.order.iter() {
                        let mi = mi as usize % NUM_MODULES;
                        if patch.enabled[mi] {
                            n += 1;
                            pos[mi] = n;
                        }
                    }
                    pos
                };
                let gap = 10.0;
                let tile_w = ((ui.available_width() - 2.0 * gap) / 3.0).floor();
                for row in 0..2 {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for col in 0..3 {
                            let i = row * 3 + col;
                            let kind = KINDS[i];
                            let on = patch.enabled[i];
                            let locked = locks[i].value();
                            ui.allocate_ui_with_layout(Vec2::new(tile_w, 76.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                                egui::Frame::new()
                                    .fill(if on { PANEL_ON } else { PANEL })
                                    .stroke(egui::Stroke::new(1.0_f32, if locked { PINK } else if on { ACID.gamma_multiply(0.5) } else { PANEL }))
                                    .corner_radius(8)
                                    .inner_margin(egui::Margin::same(10))
                                    .show(ui, |ui| {
                                        let inner = tile_w - 22.0;
                                        ui.set_min_size(Vec2::new(inner, 52.0));
                                        ui.set_max_width(inner);
                                        ui.horizontal(|ui| {
                                            let name = egui::Button::new(RichText::new(kind.name()).size(15.0).strong().color(if on { ACID } else { DIM })).frame(false);
                                            if ui
                                                .add(name)
                                                .on_hover_text("Click to turn this effect on or off. Turn it off and LOCK it to keep it out of future accidents.")
                                                .clicked()
                                            {
                                                shared.toggle[i].fetch_add(1, Ordering::Relaxed);
                                            }
                                            if on {
                                                ui.label(RichText::new(format!("#{}", positions[i])).size(11.0).color(DIM));
                                            }
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                let mut v = locked;
                                                let txt = RichText::new("LOCK").size(11.0).color(if locked { PINK } else { DIM });
                                                if ui.toggle_value(&mut v, txt).on_hover_text("Keep this module exactly as it is (on/off and settings) on the next ACCIDENT or MUTATE.").changed() {
                                                    setter.begin_set_parameter(locks[i]);
                                                    setter.set_parameter(locks[i], v);
                                                    setter.end_set_parameter(locks[i]);
                                                }
                                            });
                                        });
                                        ui.add_space(2.0);
                                        let desc = if on { patch.describe(kind) } else { "off  (click the name to turn on)".to_string() };
                                        ui.label(RichText::new(desc).size(12.0).color(if on { TEXT } else { DIM }));
                                    });
                            });
                        }
                    });
                    ui.add_space(gap - ui.spacing().item_spacing.y);
                }
                ui.add_space(4.0);

                // ---------------- Chaîne
                let chain: Vec<&str> = patch
                    .order
                    .iter()
                    .map(|&m| m as usize % NUM_MODULES)
                    .filter(|&m| patch.enabled[m])
                    .map(|m| KINDS[m].name())
                    .collect();
                ui.label(RichText::new(format!("CHAIN :  IN  >  {}  >  OUT", chain.join("  >  "))).size(12.0).color(DIM));
            });
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        },
    )
}
