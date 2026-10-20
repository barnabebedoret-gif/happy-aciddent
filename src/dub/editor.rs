//! Interface du Happy Accident DUB.

use super::engine::DUB_GROUPS;
use super::{DubParams, DubShared};
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, Color32, RichText, Vec2};
use nih_plug_egui::{create_egui_editor, widgets};
use std::sync::atomic::Ordering;
use std::sync::Arc;

const BG: Color32 = Color32::from_rgb(18, 18, 20);
const PANEL: Color32 = Color32::from_rgb(30, 30, 34);
const PANEL_ON: Color32 = Color32::from_rgb(42, 42, 47);
const TEAL: Color32 = Color32::from_rgb(222, 218, 208);
const ORANGE: Color32 = Color32::from_rgb(208, 168, 110);
const DIM: Color32 = Color32::from_rgb(128, 128, 134);
const GROUP_TIPS: [&str; 4] = [
    "TIME: delay times (left / right) and ping-pong. LOCK keeps them on the next ACCIDENT.",
    "SEND: which steps go into the delay. Everything, or only some hits (dub throws). LOCK keeps it.",
    "FILTER: filter inside the feedback loop, so each repeat gets filtered again. Fixed, different on each echo, or stepping over 2 bars. LOCK keeps it.",
    "COLOR: saturation and tape wobble inside the loop. LOCK keeps it.",
];
const TEXT: Color32 = Color32::from_rgb(226, 226, 228);

pub fn create(params: Arc<DubParams>, shared: Arc<DubShared>) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        (),
        |ctx, _| {
            let mut style = (*ctx.style()).clone();
            style.visuals = egui::Visuals::dark();
            style.visuals.panel_fill = BG;
            style.visuals.selection.bg_fill = TEAL.gamma_multiply(0.55);
            style.visuals.widgets.inactive.bg_fill = PANEL;
            style.spacing.item_spacing = Vec2::new(8.0, 6.0);
            ctx.set_style(style);
        },
        move |ctx, setter, _| {
            let hist = shared.display.lock().map(|h| *h).unwrap_or_default();
            let patch = hist.current();

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("HAPPY ACCIDENT DUB").size(26.0).strong().color(TEAL));
                        ui.label(RichText::new(concat!("FILTER DELAY  —  by BRNB  ·  v", env!("CARGO_PKG_VERSION"))).size(12.0).color(DIM));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_enabled(hist.pos + 1 < hist.len, egui::Button::new("▶")).clicked() {
                            shared.next.fetch_add(1, Ordering::Relaxed);
                        }
                        ui.label(RichText::new(format!("{}/{}", hist.pos + 1, hist.len)).color(TEXT));
                        if ui.add_enabled(hist.pos > 0, egui::Button::new("◀")).clicked() {
                            shared.prev.fetch_add(1, Ordering::Relaxed);
                        }
                        ui.label(RichText::new(format!("#{:08X}", patch.seed)).monospace().color(DIM));
                    });
                });
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    let acc = egui::Button::new(RichText::new("ACCIDENT").size(28.0).strong().color(BG)).fill(TEAL).min_size(Vec2::new(330.0, 62.0));
                    if ui.add(acc).on_hover_text("Roll a new delay: time, sends, filter and color. CHAOS sets how wild it gets. Locked groups are kept.").clicked() {
                        shared.accident.fetch_add(1, Ordering::Relaxed);
                    }
                    let mt = egui::Button::new(RichText::new("MUTATE").size(20.0).strong().color(TEXT)).fill(PANEL_ON).min_size(Vec2::new(170.0, 62.0));
                    if ui.add(mt).on_hover_text("Small variation of the current delay.").clicked() {
                        shared.mutate.fetch_add(1, Ordering::Relaxed);
                    }
                    // THROW : maintenir enfoncé
                    let throwing = params.throw.value();
                    let th = egui::Button::new(RichText::new("THROW").size(20.0).strong().color(if throwing { BG } else { ORANGE }))
                        .fill(if throwing { ORANGE } else { PANEL })
                        .stroke(egui::Stroke::new(1.5_f32, ORANGE))
                        .min_size(Vec2::new(170.0, 62.0))
                        .sense(egui::Sense::click_and_drag());
                    let r = ui.add(th).on_hover_text("Hold to push the feedback past 100% for a dub spiral. The loop saturation keeps it under control. Map it to a pad!");
                    let down = r.is_pointer_button_down_on();
                    if down != throwing {
                        setter.begin_set_parameter(&params.throw);
                        setter.set_parameter(&params.throw, down);
                        setter.end_set_parameter(&params.throw);
                    }
                });
                ui.add_space(10.0);

                let label = |ui: &mut egui::Ui, t: &str, tip: &str| {
                    ui.label(RichText::new(t).size(12.0).color(DIM)).on_hover_text(tip);
                };
                egui::Grid::new("dub_macros").num_columns(4).min_col_width(60.0).spacing(Vec2::new(12.0, 8.0)).show(ui, |ui| {
                    label(ui, "CHAOS", "How radical ACCIDENT and MUTATE are. Low = small changes. High = anything goes.");
                    ui.add(widgets::ParamSlider::for_param(&params.chaos, setter).with_width(160.0)).on_hover_text("How radical ACCIDENT and MUTATE are. Low = small changes. High = anything goes.");
                    label(ui, "FEEDBACK", "How long the echoes last. Above ~85% they ring for a long time.");
                    ui.add(widgets::ParamSlider::for_param(&params.feedback, setter).with_width(160.0)).on_hover_text("How long the echoes last. Above ~85% they ring for a long time.");
                    ui.end_row();
                    label(ui, "MOVEMENT", "How much the filter jumps from echo to echo, and how much the tape wobbles.");
                    ui.add(widgets::ParamSlider::for_param(&params.movement, setter).with_width(160.0)).on_hover_text("How much the filter jumps from echo to echo, and how much the tape wobbles.");
                    label(ui, "ECHOES", "Level of the echoes. The dry sound always passes untouched.");
                    ui.add(widgets::ParamSlider::for_param(&params.mix, setter).with_width(160.0)).on_hover_text("Level of the echoes. The dry sound always passes untouched.");
                    ui.end_row();
                    ui.label("");
                    ui.label("");
                    label(ui, "OUTPUT", "Output level, after the safety limiter.");
                    ui.add(widgets::ParamSlider::for_param(&params.output, setter).with_width(160.0)).on_hover_text("Output level, after the safety limiter.");
                    ui.end_row();
                });
                ui.add_space(12.0);

                let locks = params.lock_params();
                let gap = 10.0;
                let tile_w = ((ui.available_width() - gap) / 2.0).floor();
                for row in 0..2 {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for col in 0..2 {
                            let i = row * 2 + col;
                            let locked = locks[i].value();
                            ui.allocate_ui_with_layout(Vec2::new(tile_w, 68.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                                egui::Frame::new()
                                    .fill(PANEL_ON)
                                    .stroke(egui::Stroke::new(1.0_f32, if locked { ORANGE } else { TEAL.gamma_multiply(0.4) }))
                                    .corner_radius(8)
                                    .inner_margin(egui::Margin::same(10))
                                    .show(ui, |ui| {
                                        let inner = tile_w - 22.0;
                                        ui.set_min_size(Vec2::new(inner, 44.0));
                                        ui.set_max_width(inner);
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(DUB_GROUPS[i]).size(15.0).strong().color(TEAL)).on_hover_text(GROUP_TIPS[i]);
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                let mut v = locked;
                                                let txt = RichText::new("LOCK").size(11.0).color(if locked { ORANGE } else { DIM });
                                                if ui.toggle_value(&mut v, txt).on_hover_text(GROUP_TIPS[i]).changed() {
                                                    setter.begin_set_parameter(locks[i]);
                                                    setter.set_parameter(locks[i], v);
                                                    setter.end_set_parameter(locks[i]);
                                                }
                                            });
                                        });
                                        ui.add_space(2.0);
                                        ui.label(RichText::new(patch.describe(i)).size(12.0).color(TEXT));
                                    });
                            });
                        }
                    });
                    ui.add_space(gap - ui.spacing().item_spacing.y);
                }
            });
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        },
    )
}
