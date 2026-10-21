//! Minute-sampled artistic progression; only finite change smoothing uses a timer.
use crate::{DaycycleAssets, Home};
use emotional_core::ambient_art::Blend;
use slint::{ComponentHandle, Model};
use std::time::{Duration, Instant};

pub fn update(ui: &Home, blend: Blend, season: i32, weather: i32, celebration: i32, snap: bool) {
    let style = ui.global::<DaycycleAssets>();
    let from = style.get_resolved_colors().iter().collect::<Vec<_>>();
    let old_target = style.get_target_colors().iter().collect::<Vec<_>>();
    style.set_palette_index(blend.from as i32);
    style.set_palette_next_index(blend.to as i32);
    style.set_day_mix(blend.mix);
    style.set_season_index(season);
    style.set_weather_index(weather);
    style.set_celebration_index(celebration);
    let target = style.get_target_colors().iter().collect::<Vec<_>>();
    let snap = snap
        || ui.get_reduced()
        || ui.global::<crate::AtmosphereDesign>().get_diagnostic_mode() > 0;
    if target == old_target && (!snap || style.get_palette_progress() >= 1.) {
        return;
    }
    let generation = ui.get_ambient_generation().wrapping_add(1);
    ui.set_ambient_generation(generation);
    style.set_from_colors(slint::ModelRc::new(slint::VecModel::from(from)));
    style.set_palette_progress(if snap { 1. } else { 0. });
    if !snap {
        step(ui.as_weak(), generation, Instant::now());
    }
}
// Existing native regression helper: exercises interrupted palette changes.
pub fn transition(ui: &Home, index: i32, snap: bool) {
    let style = ui.global::<DaycycleAssets>();
    update(
        ui,
        Blend::still(index as usize),
        style.get_season_index(),
        style.get_weather_index(),
        style.get_celebration_index(),
        snap,
    );
}
fn step(weak: slint::Weak<Home>, generation: i32, start: Instant) {
    slint::Timer::single_shot(Duration::from_millis(16), move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        if ui.get_ambient_generation() != generation {
            return;
        }
        let t = if ui.get_reduced() {
            1.
        } else {
            (start.elapsed().as_secs_f32() / 0.24).min(1.)
        };
        ui.global::<DaycycleAssets>().set_palette_progress(t);
        if t < 1. {
            step(weak, generation, start);
        }
    });
}
