//! Interface du Happy Accident RISE.

use super::engine::{Curve, Recipe, INGREDIENT_NAMES, NUM_INGREDIENTS};
use super::{AmountParam, CurveParam, RiseParams, RiseShared, TailParam};
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, Color32, RichText, Vec2};
use nih_plug_egui::{create_egui_editor, widgets};
use std::f32::consts::PI;
use std::sync::atomic::Ordering;
use std::sync::Arc;

const BG: Color32 = Color32::from_rgb(18, 18, 20);
const PANEL: Color32 = Color32::from_rgb(30, 30, 34);
const PANEL_ON: Color32 = Color32::from_rgb(42, 42, 47);
const SAND: Color32 = Color32::from_rgb(222, 218, 208);
const AMBER: Color32 = Color32::from_rgb(208, 168, 110);
const DIM: Color32 = Color32::from_rgb(128, 128, 134);
const TEXT: Color32 = Color32::from_rgb(226, 226, 228);

const TENSION_TIP: &str = "TENSION: the only knob you need. Automate it from 0% to 100% over the 8 or 16 bars before the drop, \
then back to 0% on the drop: everything snaps back at once, so the drop hits full. Drag up/down, double-click = 0%.";

const INGREDIENT_TIPS: [&str; NUM_INGREDIENTS] = [
    "LOW CUT: a high-pass that rises from 20 Hz to about 600 Hz, with a little resonance at the very end. Removes the bass so the drop feels huge.",
    "SPACE: the reverb grows and its tail gets longer as the tension rises.",
    "WIDTH: the stereo image opens up, from normal to very wide.",
    "NOISE: a built-in filtered white-noise riser. Gets louder and brighter with the tension. No sample needed.",
    "ROLL: a rhythmic gate that speeds up, 1/4 → 1/8 → 1/16 → 1/32, like a snare roll applied to your sound. Follows Ableton's tempo.",
    "DRIVE: saturation that builds up, for more aggression.",
    "PITCH: pitches the sound up at the very end (last third): +1 st Subtle, +2 st Normal, +3 st Extreme.",
    "GAP: silence just before the drop. Kicks in when TENSION goes above 97%; on an 8-bar ramp to 100% that is roughly the last beat. The reverb tail keeps ringing.",
];

/// Gros bouton rotatif TENSION
fn tension_knob(ui: &mut egui::Ui, param: &FloatParam, setter: &ParamSetter, shaped: f32, gap: bool) {
    let size = 210.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::click_and_drag());
    let resp = resp.on_hover_text(TENSION_TIP);
    let value = param.unmodulated_normalized_value();

    if resp.drag_started() {
        setter.begin_set_parameter(param);
    }
    if resp.dragged() {
        let fine = if ui.input(|i| i.modifiers.shift) { 0.2 } else { 1.0 };
        let nv = (value - resp.drag_delta().y / 260.0 * fine).clamp(0.0, 1.0);
        setter.set_parameter_normalized(param, nv);
    }
    if resp.drag_stopped() {
        setter.end_set_parameter(param);
    }
    if resp.double_clicked() {
        setter.begin_set_parameter(param);
        setter.set_parameter_normalized(param, 0.0);
        setter.end_set_parameter(param);
    }

    let painter = ui.painter_at(rect.expand(4.0));
    let c = rect.center();
    let radius = size * 0.42;
    let start = PI * 0.75;
    let sweep = PI * 1.5;
    let arc = |from: f32, to: f32, r: f32| -> Vec<egui::Pos2> {
        let steps = 64;
        (0..=steps)
            .map(|k| {
                let a = start + sweep * (from + (to - from) * k as f32 / steps as f32);
                egui::pos2(c.x + r * a.cos(), c.y + r * a.sin())
            })
            .collect()
    };
    painter.circle_filled(c, radius - 14.0, PANEL);
    painter.add(egui::Shape::line(arc(0.0, 1.0, radius), egui::Stroke::new(10.0_f32, PANEL_ON)));
    let v = param.value();
    if v > 0.001 {
        let col = if v >= 0.97 { AMBER } else { SAND };
        painter.add(egui::Shape::line(arc(0.0, v, radius), egui::Stroke::new(10.0_f32, col)));
    }
    // Fine aiguille intérieure : l'intensité réellement appliquée (après la courbe)
    if shaped > 0.001 {
        painter.add(egui::Shape::line(arc(0.0, shaped.min(1.0), radius - 16.0), egui::Stroke::new(3.0_f32, AMBER.gamma_multiply(0.8))));
    }
    painter.text(c - Vec2::new(0.0, 8.0), egui::Align2::CENTER_CENTER, format!("{:.0} %", v * 100.0), egui::FontId::proportional(38.0), TEXT);
    let sub = if gap { "GAP" } else { "TENSION" };
    painter.text(c + Vec2::new(0.0, 26.0), egui::Align2::CENTER_CENTER, sub, egui::FontId::proportional(13.0), if gap { AMBER } else { DIM });
}

/// Rangée de boutons pour un paramètre à choix
fn choice<T: Enum + PartialEq + Copy + 'static>(ui: &mut egui::Ui, setter: &ParamSetter, p: &EnumParam<T>, items: &[(T, &str, &str)]) {
    let cur = p.value();
    for &(v, label, tip) in items {
        let sel = cur == v;
        let b = egui::Button::new(RichText::new(label).size(12.0).color(if sel { BG } else { TEXT })).fill(if sel { SAND } else { PANEL }).min_size(Vec2::new(62.0, 26.0));
        if ui.add(b).on_hover_text(tip).clicked() && !sel {
            setter.begin_set_parameter(p);
            setter.set_parameter(p, v);
            setter.end_set_parameter(p);
        }
    }
}

pub fn create(params: Arc<RiseParams>, shared: Arc<RiseShared>) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        0u64,
        |ctx, _| {
            let mut style = (*ctx.style()).clone();
            style.visuals = egui::Visuals::dark();
            style.visuals.panel_fill = BG;
            style.visuals.selection.bg_fill = SAND.gamma_multiply(0.55);
            style.visuals.widgets.inactive.bg_fill = PANEL;
            style.spacing.item_spacing = Vec2::new(8.0, 6.0);
            ctx.set_style(style);
        },
        move |ctx, setter, accident_count| {
            let shaped = f32::from_bits(shared.shaped.load(Ordering::Relaxed));
            let gap = shared.gap.load(Ordering::Relaxed);

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("HAPPY ACCIDENT RISE").size(26.0).strong().color(SAND));
                    ui.label(RichText::new(concat!("BUILD-UP TENSION  —  by BRNB  ·  v", env!("CARGO_PKG_VERSION"))).size(12.0).color(DIM));
                });
                ui.add_space(6.0);

                ui.vertical_centered(|ui| {
                    tension_knob(ui, &params.tension, setter, shaped, gap);
                });
                ui.add_space(8.0);

                // ---------------- Ingrédients (clic = on / off)
                let ing = params.ingredients();
                let gap_x = 8.0;
                let w = ((ui.available_width() - 3.0 * gap_x) / 4.0).floor();
                for row in 0..2 {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap_x;
                        for col in 0..4 {
                            let i = row * 4 + col;
                            let on = ing[i].value();
                            let b = egui::Button::new(RichText::new(INGREDIENT_NAMES[i]).size(15.0).strong().color(if on { SAND } else { DIM }))
                                .fill(if on { PANEL_ON } else { PANEL })
                                .stroke(egui::Stroke::new(1.5_f32, if on { SAND.gamma_multiply(0.7) } else { PANEL_ON }))
                                .corner_radius(8)
                                .min_size(Vec2::new(w, 46.0));
                            if ui.add(b).on_hover_text(format!("{}\nClick to switch on / off.", INGREDIENT_TIPS[i])).clicked() {
                                setter.begin_set_parameter(ing[i]);
                                setter.set_parameter(ing[i], !on);
                                setter.end_set_parameter(ing[i]);
                            }
                        }
                    });
                }
                ui.add_space(12.0);

                // ---------------- Réglages secondaires
                let label = |ui: &mut egui::Ui, t: &str, tip: &str| {
                    ui.add_sized(Vec2::new(64.0, 26.0), egui::Label::new(RichText::new(t).size(12.0).color(DIM))).on_hover_text(tip);
                };
                ui.horizontal(|ui| {
                    let tip = "CURVE: how the tension builds. Late = almost nothing at first, everything explodes at the end (usually sounds best). Linear = even build.";
                    label(ui, "CURVE", tip);
                    choice(ui, setter, &params.curve, &[(CurveParam::Late, "Late", tip), (CurveParam::Linear, "Linear", tip)]);
                    ui.add_space(18.0);
                    let tip = "TAIL: what happens to the reverb on the drop. Cut = it stops dead, the drop is clean. Keep = the tail keeps ringing over the drop.";
                    label(ui, "TAIL", tip);
                    choice(ui, setter, &params.tail, &[(TailParam::Cut, "Cut", tip), (TailParam::Keep, "Keep", tip)]);
                });
                ui.horizontal(|ui| {
                    let tip = "AMOUNT: overall intensity of every ingredient at 100% tension. Subtle for a light lift, Extreme for a big festival build-up.";
                    label(ui, "AMOUNT", tip);
                    choice(ui, setter, &params.amount, &[(AmountParam::Subtle, "Subtle", tip), (AmountParam::Normal, "Normal", tip), (AmountParam::Extreme, "Extreme", tip)]);
                });
                ui.horizontal(|ui| {
                    let tip = "OUTPUT: output level, after the safety limiter.";
                    label(ui, "OUTPUT", tip);
                    ui.add(widgets::ParamSlider::for_param(&params.output, setter).with_width(200.0)).on_hover_text(tip);
                });
                ui.add_space(10.0);

                let acc = egui::Button::new(RichText::new("ACCIDENT").size(22.0).strong().color(BG)).fill(SAND);
                if ui.add_sized(Vec2::new(ui.available_width(), 48.0), acc).on_hover_text("Roll a random recipe: which ingredients are on, and the curve. Your TENSION, AMOUNT and TAIL stay as they are.").clicked() {
                    *accident_count += 1;
                    let seed = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos() as u64)
                        .unwrap_or(0)
                        ^ accident_count.wrapping_mul(0x9E37_79B9);
                    let rec = Recipe::roll(seed);
                    for (p, &v) in ing.iter().zip(rec.on.iter()) {
                        setter.begin_set_parameter(*p);
                        setter.set_parameter(*p, v);
                        setter.end_set_parameter(*p);
                    }
                    let c = if rec.curve == Curve::Late { CurveParam::Late } else { CurveParam::Linear };
                    setter.begin_set_parameter(&params.curve);
                    setter.set_parameter(&params.curve, c);
                    setter.end_set_parameter(&params.curve);
                }
            });
            ctx.request_repaint_after(std::time::Duration::from_millis(40));
        },
    )
}
