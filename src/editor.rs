//! Interface du plugin (egui).

use crate::engine::{History, KINDS, NUM_MODULES};
use crate::{HaParams, Shared};
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, Color32, RichText, Vec2};
use nih_plug_egui::{create_egui_editor, widgets};
use std::sync::atomic::Ordering;
use std::sync::Arc;

const BG: Color32 = Color32::from_rgb(14, 14, 16);
const PANEL: Color32 = Color32::from_rgb(26, 26, 30);
const PANEL_ON: Color32 = Color32::from_rgb(38, 44, 22);
const ACID: Color32 = Color32::from_rgb(198, 255, 61);
const PINK: Color32 = Color32::from_rgb(255, 61, 139);
const DIM: Color32 = Color32::from_rgb(120, 120, 128);
const TEXT: Color32 = Color32::from_rgb(232, 232, 236);

pub fn create(params: Arc<HaParams>, shared: Arc<Shared>) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        (),
        |ctx, _| {
            let mut style = (*ctx.style()).clone();
            style.visuals = egui::Visuals::dark();
            style.visuals.panel_fill = BG;
            style.visuals.selection.bg_fill = ACID.gamma_multiply(0.6);
            style.visuals.widgets.inactive.bg_fill = PANEL;
            style.spacing.item_spacing = Vec2::new(8.0, 6.0);
            ctx.set_style(style);
        },
        move |ctx, setter, _| {
            let hist: History = shared.display.lock().map(|h| *h).unwrap_or_default();
            let patch = hist.current();

            egui::CentralPanel::default().show(ctx, |ui| {
                // ---------------- En-tête
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("HAPPY ACCIDENT").size(26.0).strong().color(ACID));
                        ui.label(RichText::new("GENERATOR  —  by BRNB").size(12.0).color(DIM));
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

                // ---------------- Boutons principaux
                ui.horizontal(|ui| {
                    let acc = egui::Button::new(RichText::new("ACCIDENT").size(30.0).strong().color(BG))
                        .fill(ACID)
                        .min_size(Vec2::new(430.0, 64.0));
                    if ui.add(acc).on_hover_text("Nouveau tirage (radicalité = Chaos)").clicked() {
                        shared.accident.fetch_add(1, Ordering::Relaxed);
                    }
                    let mt = egui::Button::new(RichText::new("MUTATE").size(22.0).strong().color(TEXT))
                        .fill(PINK.gamma_multiply(0.85))
                        .min_size(Vec2::new(250.0, 64.0));
                    if ui.add(mt).on_hover_text("Petite variation de l'accident actuel").clicked() {
                        shared.mutate.fetch_add(1, Ordering::Relaxed);
                    }
                });
                ui.add_space(10.0);

                // ---------------- Macros
                egui::Grid::new("macros").num_columns(4).spacing(Vec2::new(14.0, 6.0)).show(ui, |ui| {
                    for (label, p) in [("CHAOS", &params.chaos), ("TIMBRE ↔ PATTERN", &params.balance)] {
                        ui.label(RichText::new(label).size(12.0).color(DIM));
                        ui.add(widgets::ParamSlider::for_param(p, setter).with_width(200.0));
                    }
                    ui.end_row();
                    for (label, p) in [("MOVEMENT", &params.movement), ("MIX", &params.mix)] {
                        ui.label(RichText::new(label).size(12.0).color(DIM));
                        ui.add(widgets::ParamSlider::for_param(p, setter).with_width(200.0));
                    }
                    ui.end_row();
                    ui.label(RichText::new("TONALITÉ").size(12.0).color(DIM));
                    ui.horizontal(|ui| {
                        ui.add(widgets::ParamSlider::for_param(&params.key, setter).with_width(70.0));
                        ui.add(widgets::ParamSlider::for_param(&params.scale, setter).with_width(122.0));
                    });
                    ui.label(RichText::new("OUTPUT").size(12.0).color(DIM));
                    ui.add(widgets::ParamSlider::for_param(&params.output, setter).with_width(200.0));
                    ui.end_row();
                    ui.label(RichText::new("SPEED").size(12.0).color(DIM));
                    ui.add(widgets::ParamSlider::for_param(&params.speed, setter).with_width(200.0));
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
                egui::Grid::new("modules").num_columns(3).spacing(Vec2::new(10.0, 10.0)).show(ui, |ui| {
                    for (i, kind) in KINDS.iter().enumerate() {
                        let on = patch.enabled[i];
                        let locked = locks[i].value();
                        egui::Frame::new()
                            .fill(if on { PANEL_ON } else { PANEL })
                            .stroke(egui::Stroke::new(1.0_f32, if locked { PINK } else if on { ACID.gamma_multiply(0.5) } else { PANEL }))
                            .corner_radius(8)
                            .inner_margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                                ui.set_width(204.0);
                                ui.set_height(58.0);
                                ui.horizontal(|ui| {
                                    let name = RichText::new(kind.name()).size(15.0).strong().color(if on { ACID } else { DIM });
                                    ui.label(name);
                                    if on {
                                        ui.label(RichText::new(format!("#{}", positions[i])).size(11.0).color(DIM));
                                    }
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let mut v = locked;
                                        let txt = if locked { "🔒 LOCK" } else { "🔓" };
                                        if ui.toggle_value(&mut v, txt).on_hover_text("Garder ce module tel quel").changed() {
                                            setter.begin_set_parameter(locks[i]);
                                            setter.set_parameter(locks[i], v);
                                            setter.end_set_parameter(locks[i]);
                                        }
                                    });
                                });
                                let desc = if on { patch.describe(*kind) } else { "—".to_string() };
                                ui.label(RichText::new(desc).size(12.0).color(if on { TEXT } else { DIM }));
                            });
                        if i % 3 == 2 {
                            ui.end_row();
                        }
                    }
                });
                ui.add_space(8.0);

                // ---------------- Chaîne
                let chain: Vec<&str> = patch
                    .order
                    .iter()
                    .map(|&m| m as usize % NUM_MODULES)
                    .filter(|&m| patch.enabled[m])
                    .map(|m| KINDS[m].name())
                    .collect();
                ui.label(RichText::new(format!("CHAÎNE :  IN → {} → OUT", chain.join(" → "))).size(12.0).color(DIM));
            });
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        },
    )
}
