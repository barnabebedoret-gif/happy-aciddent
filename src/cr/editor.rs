//! Interface du Happy Accident CALL & RESPONSE.

use super::engine::{CrSettings, LETTERS, LENS, MODE_NAMES, SILENT, TURN_NAMES};
use super::{CrParams, CrShared, Letter};
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, Color32, RichText, Vec2};
use nih_plug_egui::{create_egui_editor, widgets};
use std::sync::atomic::Ordering;
use std::sync::Arc;

const BG: Color32 = Color32::from_rgb(18, 18, 20);
const PANEL: Color32 = Color32::from_rgb(30, 30, 34);
const DIM: Color32 = Color32::from_rgb(128, 128, 134);
const TEXT: Color32 = Color32::from_rgb(226, 226, 228);
pub const LETTER_COLORS: [Color32; 4] = [
    Color32::from_rgb(222, 214, 196),
    Color32::from_rgb(150, 176, 200),
    Color32::from_rgb(184, 166, 204),
    Color32::from_rgb(206, 162, 140),
];

fn chip(ui: &mut egui::Ui, text: &str, selected: bool, color: Color32, w: f32) -> bool {
    let b = egui::Button::new(RichText::new(text).size(13.0).strong().color(if selected { BG } else { TEXT }))
        .fill(if selected { color } else { PANEL })
        .min_size(Vec2::new(w, 26.0));
    ui.add(b).clicked()
}

pub fn create(params: Arc<CrParams>, shared: Arc<CrShared>) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        0u64,
        |ctx, _| {
            let mut style = (*ctx.style()).clone();
            style.visuals = egui::Visuals::dark();
            style.visuals.panel_fill = BG;
            style.visuals.widgets.inactive.bg_fill = PANEL;
            style.visuals.selection.bg_fill = LETTER_COLORS[0].gamma_multiply(0.6);
            style.spacing.item_spacing = Vec2::new(8.0, 6.0);
            ctx.set_style(style);
        },
        move |ctx, setter, seed_state| {
            let s = params.current();
            let pattern = s.pattern();
            let len = s.len();
            let letter = params.letter.value().index();
            let my_color = LETTER_COLORS[letter as usize];
            let slot = shared.slot.load(Ordering::Relaxed) as usize;
            let (hpos, hlen) = params.history.read().map(|h| (h.pos, h.len)).unwrap_or((0, 1));

            egui::CentralPanel::default().show(ctx, |ui| {
                // ---------------- En-tête
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("HAPPY ACCIDENT").size(24.0).strong().color(my_color));
                        ui.label(RichText::new(concat!("CALL & RESPONSE  —  by BRNB  ·  v", env!("CARGO_PKG_VERSION"))).size(12.0).color(DIM));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        for g in (1..=4).rev() {
                            if chip(ui, &g.to_string(), params.group.value() == g, TEXT, 28.0) {
                                setter.begin_set_parameter(&params.group);
                                setter.set_parameter(&params.group, g);
                                setter.end_set_parameter(&params.group);
                            }
                        }
                        ui.label(RichText::new("GROUP").size(12.0).color(DIM)).on_hover_text("Instances in the same group share one pattern. Use another group for an independent call & response in the same set.");
                    });
                });
                ui.add_space(8.0);

                // ---------------- Lettre de cette piste
                ui.horizontal(|ui| {
                    ui.label(RichText::new("THIS TRACK").size(12.0).color(DIM)).on_hover_text("The letter of this track. It only plays when its letter has the turn.");
                    for i in 0..4u8 {
                        let active = i < s.voices;
                        let sel = letter == i;
                        let c = LETTER_COLORS[i as usize];
                        let b = egui::Button::new(RichText::new(LETTERS[i as usize]).size(26.0).strong().color(if sel { BG } else if active { c } else { DIM }))
                            .fill(if sel { c } else { PANEL })
                            .stroke(egui::Stroke::new(1.0_f32, if active { c.gamma_multiply(0.6) } else { PANEL }))
                            .min_size(Vec2::new(70.0, 52.0));
                        if ui.add(b).clicked() {
                            setter.begin_set_parameter(&params.letter);
                            setter.set_parameter(&params.letter, Letter::from_index(i));
                            setter.end_set_parameter(&params.letter);
                        }
                    }
                    if letter >= s.voices {
                        ui.label(RichText::new("letter outside the active voices: this track stays muted").size(11.0).color(LETTER_COLORS[1]));
                    }
                });
                ui.add_space(8.0);

                // ---------------- Accident
                ui.horizontal(|ui| {
                    let acc = egui::Button::new(RichText::new("ACCIDENT").size(24.0).strong().color(BG)).fill(my_color).min_size(Vec2::new(300.0, 52.0));
                    if ui.add(acc).on_hover_text("New pattern for the whole group. With a high CHAOS, the turn length, steps and mode can change too.").clicked() {
                        *seed_state = crate::engine::rng::mix(
                            *seed_state ^ std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1),
                        );
                        params.commit(s.accident((*seed_state >> 40) as u32, params.chaos.value()));
                    }
                    let mt = egui::Button::new(RichText::new("MUTATE").size(18.0).strong().color(TEXT)).fill(PANEL).min_size(Vec2::new(150.0, 52.0));
                    if ui.add(mt).on_hover_text("Changes one step of the pattern.").clicked() {
                        params.commit(s.mutate());
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_enabled(hpos + 1 < hlen, egui::Button::new("▶")).on_hover_text("Next pattern in the history").clicked() {
                            if let Ok(mut h) = params.history.write() {
                                if h.forward() {
                                    if let Some(n) = CrSettings::unpack(h.current()) {
                                        super::engine::store_group(params.group_index(), &n);
                                    }
                                }
                            }
                        }
                        ui.label(RichText::new(format!("{}/{}", hpos + 1, hlen)).color(TEXT));
                        if ui.add_enabled(hpos > 0, egui::Button::new("◀")).on_hover_text("Previous pattern. Saved with your Live set.").clicked() {
                            if let Ok(mut h) = params.history.write() {
                                if h.back() {
                                    if let Some(n) = CrSettings::unpack(h.current()) {
                                        super::engine::store_group(params.group_index(), &n);
                                    }
                                }
                            }
                        }
                    });
                });
                ui.add_space(10.0);

                // ---------------- Variations
                let active_var = super::engine::active_variation(params.group_index());
                ui.horizontal(|ui| {
                    ui.label(RichText::new("VARIATION").size(12.0).color(DIM)).on_hover_text(
                        "Each group stores 4 variations of the pattern. Click to switch the whole group. \
                         To switch during your arrangement, automate the Variation parameter on ONE track only.",
                    );
                    for v in 0..super::engine::NUM_VARS {
                        let b = egui::Button::new(RichText::new((v + 1).to_string()).size(16.0).strong().color(if v == active_var { BG } else { TEXT }))
                            .fill(if v == active_var { LETTER_COLORS[0] } else { PANEL })
                            .min_size(Vec2::new(44.0, 30.0));
                        let tip = "Switch the whole group to this variation. An empty variation starts as a copy of the current one: then change it with ACCIDENT, MUTATE or the settings.";
                        if ui.add(b).on_hover_text(tip).clicked() {
                            setter.begin_set_parameter(&params.variation);
                            setter.set_parameter(&params.variation, super::Variation::from_index(v));
                            setter.end_set_parameter(&params.variation);
                            // Si ce paramètre avait déjà cette valeur, on bascule directement
                            if params.variation.value().index() == v {
                                super::engine::select_variation(params.group_index(), v);
                            }
                        }
                    }
                });
                ui.add_space(6.0);

                // ---------------- Timeline du pattern
                let avail = ui.available_width();
                let cell = ((avail - 4.0 * (len as f32 - 1.0)) / len as f32).floor().min(80.0);
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(avail, 54.0), egui::Sense::hover());
                resp.on_hover_text("The pattern. The outlined step is playing now. Bright steps = this track.");
                let painter = ui.painter_at(rect);
                for i in 0..len {
                    let x = rect.left() + i as f32 * (cell + 4.0);
                    let r = egui::Rect::from_min_size(egui::pos2(x, rect.top()), Vec2::new(cell, 46.0));
                    let v = pattern[i];
                    let (fill, txt) = if v == SILENT {
                        (PANEL, "·")
                    } else {
                        let c = LETTER_COLORS[v as usize];
                        (if v == letter { c } else { c.gamma_multiply(0.35) }, LETTERS[v as usize])
                    };
                    painter.rect_filled(r, 6.0, fill);
                    if i == slot {
                        painter.rect_stroke(r.expand(2.0), 7.0, egui::Stroke::new(2.0_f32, TEXT), egui::StrokeKind::Outside);
                    }
                    painter.text(r.center(), egui::Align2::CENTER_CENTER, txt, egui::FontId::proportional(if cell > 30.0 { 18.0 } else { 12.0 }), if v == letter { BG } else { TEXT });
                }
                ui.add_space(6.0);

                // ---------------- Réglages du groupe
                let mut next = s;
                egui::Grid::new("cr_settings").num_columns(2).spacing(Vec2::new(12.0, 8.0)).show(ui, |ui| {
                    ui.label(RichText::new("VOICES").size(12.0).color(DIM)).on_hover_text("How many tracks take part: A/B, A/B/C or A/B/C/D.");
                    ui.horizontal(|ui| {
                        for v in 2..=4u8 {
                            if chip(ui, &v.to_string(), s.voices == v, TEXT, 34.0) {
                                next.voices = v;
                            }
                        }
                    });
                    ui.end_row();
                    ui.label(RichText::new("MODE").size(12.0).color(DIM)).on_hover_text("Alternate: strict rotation. Random: random order with few repeats. Repeats: a track often keeps the turn for several steps.");
                    ui.horizontal(|ui| {
                        for (m, name) in MODE_NAMES.iter().enumerate() {
                            if chip(ui, name, s.mode as usize == m, TEXT, 90.0) {
                                next.mode = m as u8;
                            }
                        }
                    });
                    ui.end_row();
                    ui.label(RichText::new("TURN").size(12.0).color(DIM)).on_hover_text("How long each turn lasts, from 1/16 to 2 bars.");
                    ui.horizontal(|ui| {
                        for (t, name) in TURN_NAMES.iter().enumerate() {
                            if chip(ui, name, s.turn_idx as usize == t, TEXT, 56.0) {
                                next.turn_idx = t as u8;
                            }
                        }
                    });
                    ui.end_row();
                    ui.label(RichText::new("STEPS").size(12.0).color(DIM)).on_hover_text("Pattern length (number of turns) before it loops.");
                    ui.horizontal(|ui| {
                        for (k, n) in LENS.iter().enumerate() {
                            if chip(ui, &n.to_string(), s.len_idx as usize == k, TEXT, 34.0) {
                                next.len_idx = k as u8;
                            }
                        }
                        ui.add_space(20.0);
                        ui.label(RichText::new("SILENCES").size(12.0).color(DIM)).on_hover_text("Adds steps where nobody plays. Turn it up for more space and tension.");
                        let mut sil = s.silence as f32;
                        if ui.add(egui::Slider::new(&mut sil, 0.0..=15.0).show_value(false)).changed() {
                            next.silence = sil.round() as u8;
                        }
                    });
                    ui.end_row();
                });
                if next != s {
                    params.commit(next);
                }
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("CHAOS").size(12.0).color(DIM)).on_hover_text("How much ACCIDENT changes. Low = only a new order. High = turn length, steps and mode too.");
                    ui.add(widgets::ParamSlider::for_param(&params.chaos, setter).with_width(110.0)).on_hover_text("How much ACCIDENT changes. Low = only a new order. High = turn length, steps and mode too.");
                    ui.label(RichText::new("DEPTH").size(12.0).color(DIM)).on_hover_text("100% = the other tracks are fully muted. Lower = they are only ducked.");
                    ui.add(widgets::ParamSlider::for_param(&params.depth, setter).with_width(110.0)).on_hover_text("100% = the other tracks are fully muted. Lower = they are only ducked.");
                    ui.label(RichText::new("FADE").size(12.0).color(DIM)).on_hover_text("Fade time between turns. Short = tight cuts. Long = smooth handovers.");
                    ui.add(widgets::ParamSlider::for_param(&params.fade, setter).with_width(110.0)).on_hover_text("Fade time between turns. Short = tight cuts. Long = smooth handovers.");
                });
            });
            ctx.request_repaint_after(std::time::Duration::from_millis(40));
        },
    )
}
