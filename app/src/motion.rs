//! Finite, interruptible section motion. No timer survives a settled transition.
use crate::Home;
use slint::ComponentHandle;
use std::time::{Duration, Instant};
fn interpolate(from: f32, to: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    from + (to - from) * (1.0 - (1.0 - t).powi(3))
}
pub fn transition(ui: &Home, section: f32, pan: f32, snap: bool) {
    let generation = ui.get_motion_generation().wrapping_add(1);
    ui.set_motion_generation(generation);
    ui.set_section_target(section);
    ui.set_pan_target(pan);
    if snap {
        ui.set_section_position(section);
        ui.set_pan_position(pan);
        return;
    }
    step(
        ui.as_weak(),
        generation,
        Instant::now(),
        ui.get_section_position(),
        ui.get_pan_position(),
        section,
        pan,
    );
}
fn step(
    weak: slint::Weak<Home>,
    generation: i32,
    start: Instant,
    from: f32,
    from_pan: f32,
    target: f32,
    target_pan: f32,
) {
    slint::Timer::single_shot(Duration::from_millis(16), move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        if ui.get_motion_generation() != generation {
            return;
        }
        let duration = ui.get_section_duration().max(1) as f32 / 1000.0;
        let t = (start.elapsed().as_secs_f32() / duration).min(1.0);
        ui.set_section_position(interpolate(from, target, t));
        #[cfg(feature = "simulator")]
        println!(
            "MOTION_SAMPLE elapsed_ms={} section={:.4} target={target:.1}",
            start.elapsed().as_millis(),
            ui.get_section_position()
        );
        ui.set_pan_position(interpolate(from_pan, target_pan, t));
        if t < 1.0 {
            step(weak, generation, start, from, from_pan, target, target_pan);
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interruption_starts_from_current_position_and_ends_exactly() {
        let midway = interpolate(0.0, 1.0, 0.3);
        assert!(midway > 0.0 && midway < 1.0);
        assert_eq!(interpolate(midway, 0.0, 0.0), midway);
        assert_eq!(interpolate(midway, 0.0, 1.0), 0.0);
        assert_eq!(interpolate(0.0, 1.0, 1.0), 1.0);
    }
}
