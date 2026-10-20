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
                let label = |ui: &mut egui::Ui, t: &str| {
                    ui.label(RichText::new(t).size(12.0).color(DIM));
                };
                egui::Grid::new("macros").num_columns(4).min_col_width(60.0).spacing(Vec2::new(12.0, 8.0)).show(ui, |ui| {
                    label(ui, "CHAOS");
                    ui.add(widgets::ParamSlider::for_param(&params.chaos, setter).with_width(160.0));
                    label(ui, "TIMBRE / PATTERN");
                    ui.add(widgets::ParamSlider::for_param(&params.balance, setter).with_width(160.0));
                    ui.end_row();
                    label(ui, "MOVEMENT");
                    ui.add(widgets::ParamSlider::for_param(&params.movement, setter).with_width(160.0));
                    label(ui, "SPEED");
                    ui.add(widgets::ParamSlider::for_param(&params.speed, setter).with_width(160.0));
                    ui.end_row();
                    label(ui, "TONALITÉ");
                    ui.horizontal(|ui| {
                        ui.add(widgets::ParamSlider::for_param(&params.key, setter).with_width(50.0));
                        ui.add(widgets::ParamSlider::for_param(&params.scale, setter).with_width(60.0));
                    });
                    label(ui, "MIX");
                    ui.add(widgets::ParamSlider::for_param(&params.mix, setter).with_width(160.0));
                    ui.end_row();
                    ui.label("");
                    ui.label("");
                    label(ui, "OUTPUT");
                    ui.add(widgets::ParamSlider::for_param(&params.output, setter).with_width(160.0));
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
                                            ui.label(RichText::new(kind.name()).size(15.0).strong().color(if on { ACID } else { DIM }));
                                            if on {
                                                ui.label(RichText::new(format!("#{}", positions[i])).size(11.0).color(DIM));
                                            }
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                let mut v = locked;
                                                let txt = RichText::new("LOCK").size(11.0).color(if locked { PINK } else { DIM });
                                                if ui.toggle_value(&mut v, txt).on_hover_text("Garder ce module tel quel").changed() {
                                                    setter.begin_set_parameter(locks[i]);
                                                    setter.set_parameter(locks[i], v);
                                                    setter.end_set_parameter(locks[i]);
                                                }
                                            });
                                        });
                                        ui.add_space(2.0);
                                        let desc = if on { patch.describe(kind) } else { "off".to_string() };
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
                ui.label(RichText::new(format!("CHAÎNE :  IN  >  {}  >  OUT", chain.join("  >  "))).size(12.0).color(DIM));
            });
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        },
    )
}
