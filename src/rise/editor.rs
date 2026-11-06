//! Interface du Happy Accident RISE : un gros bouton, trois petits.

use super::{RiseParams, RiseShared};
use nih_plug::prelude::*;
use nih_plug_egui::create_egui_editor;
use nih_plug_egui::egui::{self, Color32, RichText, Vec2};
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

const TENSION_TIP: &str = "TENSION: the only knob to automate. Ramp it from 0% to 100% over the 8 or 16 bars before the drop, \
then straight back to 0% on the drop: everything snaps back at once and the reverb is cut, so the drop hits full. \
At 0% the sound is untouched. The thin amber arc shows how much is really applied (it builds late, on purpose). \
Drag up/down (Shift = fine), double-click = 0%.";
const CUT_TIP: &str = "CUT: how much bass is removed at 100% tension. 0% = up to 150 Hz (light), 100% = up to 1 kHz (radical). Drag up/down, double-click = default.";
const SPACE_TIP: &str = "SPACE: how much reverb (and stereo width) at 100% tension. 0% = none. Drag up/down, double-click = default.";
const ROLL_TIP: &str = "ROLL: a gate that speeds up with the tension, 1/4 → 1/8 → 1/16 → 1/32, like a snare roll applied to your sound. Locked to Ableton's tempo. 0% = off. Drag up/down, double-click = default.";

/// Bouton rotatif dessiné à la main
fn knob(ui: &mut egui::Ui, param: &FloatParam, setter: &ParamSetter, size: f32, label: &str, tip: &str, inner: Option<f32>) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::click_and_drag());
    let resp = resp.on_hover_text(tip);
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
        setter.set_parameter_normalized(param, param.default_normalized_value());
        setter.end_set_parameter(param);
    }

    let big = size > 150.0;
    let painter = ui.painter_at(rect.expand(4.0));
    let c = rect.center();
    let radius = size * 0.42;
    let thick = if big { 10.0 } else { 6.0 };
    let start = PI * 0.75;
    let sweep = PI * 1.5;
    let arc = |to: f32, r: f32| -> Vec<egui::Pos2> {
        (0..=64)
            .map(|k| {
                let a = start + sweep * to * k as f32 / 64.0;
                egui::pos2(c.x + r * a.cos(), c.y + r * a.sin())
            })
            .collect()
    };
    painter.circle_filled(c, radius - thick - 4.0, PANEL);
    painter.add(egui::Shape::line(arc(1.0, radius), egui::Stroke::new(thick, PANEL_ON)));
    let v = param.value();
    if v > 0.001 {
        painter.add(egui::Shape::line(arc(v, radius), egui::Stroke::new(thick, SAND)));
    }
    if let Some(s) = inner {
        if s > 0.001 {
            painter.add(egui::Shape::line(arc(s.min(1.0), radius - 16.0), egui::Stroke::new(3.0_f32, AMBER)));
        }
    }
    let (vs, ls) = if big { (38.0, 13.0) } else { (17.0, 11.0) };
    painter.text(c - Vec2::new(0.0, vs * 0.2), egui::Align2::CENTER_CENTER, format!("{:.0} %", v * 100.0), egui::FontId::proportional(vs), TEXT);
    painter.text(c + Vec2::new(0.0, vs * 0.75), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(ls), DIM);
}

pub fn create(params: Arc<RiseParams>, shared: Arc<RiseShared>) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        (),
        |ctx, _| {
            let mut style = (*ctx.style()).clone();
            style.visuals = egui::Visuals::dark();
            style.visuals.panel_fill = BG;
            ctx.set_style(style);
        },
        move |ctx, setter, _| {
            let shaped = f32::from_bits(shared.shaped.load(Ordering::Relaxed));
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label(RichText::new("HAPPY ACCIDENT RISE").size(26.0).strong().color(SAND));
                ui.label(RichText::new(concat!("BUILD-UP TENSION  —  by BRNB  ·  v", env!("CARGO_PKG_VERSION"))).size(12.0).color(DIM));
                ui.add_space(8.0);
                ui.vertical_centered(|ui| {
                    knob(ui, &params.tension, setter, 210.0, "TENSION", TENSION_TIP, Some(shaped));
                });
                ui.add_space(10.0);
                let small = 110.0;
                let gap = 30.0;
                let total = 3.0 * small + 2.0 * gap;
                ui.horizontal(|ui| {
                    ui.add_space(((ui.available_width() - total) / 2.0).max(0.0));
                    ui.spacing_mut().item_spacing.x = gap;
                    knob(ui, &params.cut, setter, small, "CUT", CUT_TIP, None);
                    knob(ui, &params.space, setter, small, "SPACE", SPACE_TIP, None);
                    knob(ui, &params.roll, setter, small, "ROLL", ROLL_TIP, None);
                });
            });
            ctx.request_repaint_after(std::time::Duration::from_millis(40));
        },
    )
}
