//! Finite spatial motion. Logical navigation belongs to State; these trajectories
//! retain the current rendered position and never queue completed destinations.
use crate::{Home, MotionDesign};
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::time::{Duration, Instant};

const PAGES: usize = 4;

fn interpolate(from: f32, to: f32, t: f32) -> f32 {
    if t >= 1.0 {
        return to;
    }
    let t = t.clamp(0.0, 1.0);
    from + (to - from) * (1.0 - (1.0 - t).powi(3))
}

// Spatial navigation has a quieter onset than the short modal/press feedback.
// Zero endpoint speed avoids launching a full-width page at maximum velocity.
fn spatial_interpolate(from: f32, to: f32, t: f32) -> f32 {
    if t >= 1.0 {
        return to;
    }
    let t = t.clamp(0.0, 1.0);
    from + (to - from) * (t * t * (3.0 - 2.0 * t))
}

fn values(model: ModelRc<f32>, fallback: [f32; PAGES]) -> [f32; PAGES] {
    std::array::from_fn(|i| model.row_data(i).unwrap_or(fallback[i]))
}

#[derive(Clone, Copy, Debug)]
struct SpatialState {
    pages: [f32; PAGES],
    secondary: [f32; PAGES],
    nav: f32,
    pan: f32,
    section: f32,
}

#[derive(Clone, Debug)]
struct SpatialPlan {
    from: SpatialState,
    to: SpatialState,
    destination: usize,
    duration_ms: f32,
    delay_ms: f32,
}

// Simulator-only deterministic captures of the SAME production trajectory.
// Cancels timer callbacks; never compiled into the physical KMS application.
#[cfg(feature = "simulator")]
thread_local! { static CAPTURE_PLAN: std::cell::RefCell<Option<SpatialPlan>> = const { std::cell::RefCell::new(None) }; }
#[cfg(feature = "simulator")]
pub fn sample_for_capture(ui: &Home, elapsed_ms: u32) -> bool {
    CAPTURE_PLAN.with(|plan| {
        let plan = plan.borrow();
        let Some(plan) = plan.as_ref() else {
            return false;
        };
        ui.set_motion_generation(ui.get_motion_generation().wrapping_add(1));
        apply(ui, plan.sample(elapsed_ms as f32));
        true
    })
}
impl SpatialPlan {
    fn new(
        mut from: SpatialState,
        destination: usize,
        direction: i32,
        span: f32,
        pan: f32,
        duration_ms: f32,
        delay_ms: f32,
    ) -> Self {
        let sign = if direction < 0 { -1.0 } else { 1.0 };
        // A page completely outside the viewport can be prepared on its entry
        // side. Partially visible pages retain their exact rendered position.
        if from.pages[destination].abs() >= span {
            from.pages[destination] = sign * span;
        }
        if from.secondary[destination].abs() >= span {
            from.secondary[destination] = sign * span;
        }
        let pages = std::array::from_fn(|i| {
            if i == destination {
                0.0
            } else if from.pages[i].abs() < span || from.secondary[i].abs() < span {
                -sign * span
            } else {
                from.pages[i]
            }
        });
        let secondary = std::array::from_fn(|i| if i == destination { 0.0 } else { pages[i] });
        Self {
            from,
            to: SpatialState {
                pages,
                secondary,
                nav: destination as f32,
                pan,
                section: if destination == 0 { 0.0 } else { 1.0 },
            },
            destination,
            duration_ms: duration_ms.max(1.0),
            delay_ms: delay_ms.max(0.0).min(duration_ms * 0.4),
        }
    }
    fn sample(&self, elapsed_ms: f32) -> SpatialState {
        let t = elapsed_ms / self.duration_ms;
        let secondary_t =
            (elapsed_ms - self.delay_ms).max(0.0) / (self.duration_ms - self.delay_ms).max(1.0);
        SpatialState {
            pages: std::array::from_fn(|i| {
                spatial_interpolate(self.from.pages[i], self.to.pages[i], t)
            }),
            secondary: std::array::from_fn(|i| {
                // Only entering content is staggered. Outgoing content responds
                // on the first tick, including an immediate direction reversal.
                spatial_interpolate(
                    self.from.secondary[i],
                    self.to.secondary[i],
                    if i == self.destination {
                        secondary_t
                    } else {
                        t
                    },
                )
            }),
            nav: spatial_interpolate(self.from.nav, self.to.nav, t),
            pan: spatial_interpolate(self.from.pan, self.to.pan, t),
            section: spatial_interpolate(self.from.section, self.to.section, t),
        }
    }
    fn active(&self, elapsed_ms: f32) -> bool {
        elapsed_ms < self.duration_ms
    }
    fn changed(&self) -> bool {
        self.from.pages != self.to.pages
            || self.from.secondary != self.to.secondary
            || self.from.nav != self.to.nav
            || self.from.pan != self.to.pan
            || self.from.section != self.to.section
    }
}

fn apply(ui: &Home, frame: SpatialState) {
    ui.set_page_offsets(ModelRc::new(VecModel::from(frame.pages.to_vec())));
    ui.set_page_secondary_offsets(ModelRc::new(VecModel::from(frame.secondary.to_vec())));
    ui.set_nav_position(frame.nav);
    ui.set_pan_position(frame.pan);
    ui.set_section_position(frame.section);
}

/// Retarget all four persistent sections. Direction is the accepted L/R action,
/// including wraparound, rather than a delayed inference from rendered position.
pub fn sync(ui: &Home, destination: i32, direction: i32, snap: bool) {
    let destination = destination.clamp(0, 3) as usize;
    let snap = snap || ui.get_reduced();
    let camera = if ui.get_pan_enabled()
        && ui.get_ambient_motion()
        && !ui.get_neutral()
        && !ui.get_reduced()
    {
        [-1.0, 1.0, 0.0, 0.0][destination]
    } else {
        0.0
    };
    if ui.get_motion_section() == destination as i32 && ui.get_pan_target() == camera && !snap {
        return;
    }
    let style = ui.global::<MotionDesign>();
    let span = style.get_page_span();
    let fallback = [0.0, span, span, span];
    let from = SpatialState {
        pages: values(ui.get_page_offsets(), fallback),
        secondary: values(ui.get_page_secondary_offsets(), fallback),
        nav: ui.get_nav_position(),
        pan: ui.get_pan_position(),
        section: ui.get_section_position(),
    };
    let plan = SpatialPlan::new(
        from,
        destination,
        direction,
        span,
        camera,
        if destination < 2 && ui.get_motion_section() < 2 {
            style.get_section_ms()
        } else {
            style.get_functional_ms()
        } as f32,
        style.get_secondary_delay_ms() as f32,
    );
    let generation = ui.get_motion_generation().wrapping_add(1);
    #[cfg(feature = "simulator")]
    CAPTURE_PLAN.with(|last| *last.borrow_mut() = Some(plan.clone()));
    ui.set_motion_generation(generation);
    ui.set_motion_section(destination as i32);
    ui.set_section_target(plan.to.section);
    ui.set_pan_target(camera);
    if snap || !plan.changed() {
        apply(ui, plan.to);
    } else {
        // Offscreen preparation is invisible; visible pages stay at their
        // current positions until the next actual event-loop tick.
        apply(ui, plan.from);
        spatial_step(ui.as_weak(), generation, Instant::now(), plan);
    }
}
fn spatial_step(weak: slint::Weak<Home>, generation: i32, start: Instant, plan: SpatialPlan) {
    let frame_ms = weak
        .upgrade()
        .map(|ui| ui.global::<MotionDesign>().get_frame_ms())
        .unwrap_or(16);
    slint::Timer::single_shot(Duration::from_millis(frame_ms.max(1) as u64), move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        if ui.get_motion_generation() != generation {
            return;
        }
        let elapsed = if ui.get_reduced() {
            plan.duration_ms
        } else {
            start.elapsed().as_secs_f32() * 1000.0
        };
        apply(&ui, plan.sample(elapsed));
        if plan.active(elapsed) {
            spatial_step(weak, generation, start, plan);
        } else if crate::performance::enabled() {
            crate::trial_replay::motion_completed(generation, plan.duration_ms, elapsed);
        }
    });
}

/// One drawn modal is retained during close. State has already released its
/// logical modal/focus; rendering this closing surface must never capture input.
pub fn modal(ui: &Home, kind: i32, snap: bool) {
    let kind = kind.clamp(0, 3);
    let snap = snap || ui.get_reduced();
    if ui.get_modal_target() == kind && !snap {
        return;
    }
    let generation = ui.get_modal_generation().wrapping_add(1);
    ui.set_modal_generation(generation);
    ui.set_modal_target(kind);
    if kind > 0 {
        ui.set_modal_kind(kind);
    }
    let target = if kind > 0 { 1.0 } else { 0.0 };
    let from = ui.get_modal_progress();
    if snap || from == target {
        ui.set_modal_progress(target);
        if kind == 0 {
            ui.set_modal_kind(0);
        }
        return;
    }
    let style = ui.global::<MotionDesign>();
    let duration = if kind == 0 {
        style.get_close_ms()
    } else {
        style.get_panel_ms()
    };
    modal_step(
        ui.as_weak(),
        generation,
        Instant::now(),
        from,
        target,
        duration.max(1) as f32,
    );
}
fn modal_step(
    weak: slint::Weak<Home>,
    generation: i32,
    start: Instant,
    from: f32,
    target: f32,
    duration_ms: f32,
) {
    let frame_ms = weak
        .upgrade()
        .map(|ui| ui.global::<MotionDesign>().get_frame_ms())
        .unwrap_or(16);
    slint::Timer::single_shot(Duration::from_millis(frame_ms.max(1) as u64), move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        if ui.get_modal_generation() != generation {
            return;
        }
        let t = if ui.get_reduced() {
            1.0
        } else {
            start.elapsed().as_secs_f32() * 1000.0 / duration_ms
        };
        ui.set_modal_progress(interpolate(from, target, t));
        if t < 1.0 {
            modal_step(weak, generation, start, from, target, duration_ms);
        } else if target == 0.0 {
            ui.set_modal_kind(0);
        }
    });
}

// Compatibility entry point for the legacy two-section checks. Product State
// uses sync, so the compatibility timer never runs alongside a spatial plan.
#[allow(dead_code)]
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
    legacy_step(
        ui.as_weak(),
        generation,
        Instant::now(),
        ui.get_section_position(),
        ui.get_pan_position(),
        section,
        pan,
    );
}
fn legacy_step(
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
        let t = (start.elapsed().as_secs_f32() * 1000.0 / ui.get_section_duration().max(1) as f32)
            .min(1.0);
        ui.set_section_position(interpolate(from, target, t));
        ui.set_pan_position(interpolate(from_pan, target_pan, t));
        if t < 1.0 {
            legacy_step(weak, generation, start, from, from_pan, target, target_pan);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn initial() -> SpatialState {
        SpatialState {
            pages: [0., 640., 640., 640.],
            secondary: [0., 640., 640., 640.],
            nav: 0.,
            pan: -1.,
            section: 0.,
        }
    }
    fn plan(from: SpatialState, to: usize, direction: i32) -> SpatialPlan {
        SpatialPlan::new(from, to, direction, 640., [-1., 1., 0., 0.][to], 260., 45.)
    }
    #[test]
    fn interruption_starts_from_current_position_and_ends_exactly() {
        let midway = interpolate(0.0, 1.0, 0.3);
        assert!(midway > 0.0 && midway < 1.0);
        assert_eq!(interpolate(midway, 0.0, 0.0), midway);
        assert_eq!(interpolate(midway, 0.0, 1.0), 0.0);
        assert_eq!(interpolate(0.0, 1.0, 1.0), 1.0);
    }
    #[test]
    fn spatial_navigation_starts_and_stops_gently_without_changing_endpoints() {
        let sample = |t| spatial_interpolate(0., 1., t);
        assert_eq!(sample(0.), 0.);
        assert_eq!(sample(1.), 1.);
        assert!(sample(0.25) < 0.2);
        assert!(1. - sample(0.75) < 0.2);
        assert!(sample(0.6) - sample(0.4) > sample(0.2) - sample(0.));
        for i in 1..=100 {
            assert!(sample(i as f32 / 100.) >= sample((i - 1) as f32 / 100.));
        }
    }
    #[test]
    fn two_sections_coexist_and_finish_exactly_without_fading() {
        let p = plan(initial(), 1, 1);
        let halfway = p.sample(130.);
        assert!(halfway.pages[0] < 0. && halfway.pages[0] > -640.);
        assert!(halfway.pages[1] > 0. && halfway.pages[1] < 640.);
        assert_eq!(halfway.pages[1] - halfway.pages[0], 640.);
        assert_eq!(p.sample(260.).pages, [-640., 0., 640., 640.]);
        assert!(!p.active(260.));
        assert!(!p.active(10_000.));
    }
    #[test]
    fn retarget_and_reverse_keep_every_visible_page_at_current_position() {
        let first = plan(initial(), 1, 1);
        let now = first.sample(70.);
        let reverse = plan(now, 0, -1);
        assert_eq!(reverse.sample(0.).pages, now.pages);
        assert_eq!(reverse.sample(0.).secondary, now.secondary);
        assert_eq!(reverse.sample(0.).pan, now.pan);
        let next = plan(now, 2, 1);
        assert_eq!(next.sample(0.).pages[0], now.pages[0]);
        assert_eq!(next.sample(0.).pages[1], now.pages[1]);
        assert_eq!(next.sample(260.).pages[2], 0.);
    }
    #[test]
    fn wrap_enters_from_requested_side_only_while_offscreen() {
        let at_settings = plan(initial(), 3, 1).sample(260.);
        let wrap = plan(at_settings, 0, 1);
        assert_eq!(wrap.sample(0.).pages[0], 640.);
        assert_eq!(wrap.sample(0.).pages[3], 0.);
        assert_eq!(wrap.sample(260.).pages[0], 0.);
    }
    #[test]
    fn complete_cards_start_and_finish_outside_both_physical_edges() {
        // Actual card extents, including the wider functional panels. This
        // checks full rectangles, not only a page's left-hand origin.
        let rectangles = [
            (24., 448.),
            (24., 216.),
            (256., 216.),
            (24., 400.),
            (176., 248.),
            (24., 592.),
        ];
        for direction in [-1, 1] {
            let at_moment = plan(initial(), 1, 1).sample(260.);
            let (from, destination, outgoing) = if direction > 0 {
                (initial(), 1, 0)
            } else {
                (at_moment, 0, 1)
            };
            let trajectory = plan(from, destination, direction);
            for (left, width) in rectangles {
                let incoming = trajectory.sample(0.).pages[destination] + left;
                let departed = trajectory.sample(260.).pages[outgoing] + left;
                assert!(incoming >= 640. || incoming + width <= 0.);
                assert!(departed >= 640. || departed + width <= 0.);
            }
            assert_eq!(trajectory.sample(260.).pages[destination], 0.);
            // A larger path must not extend the shared deadline.
            assert!(!trajectory.active(260.));
        }
    }
    #[test]
    fn reduced_motion_endpoint_is_exact_and_no_idle_work_is_required() {
        let p = plan(initial(), 1, 1);
        assert_eq!(p.sample(p.duration_ms).pages, p.to.pages);
        assert_eq!(p.sample(p.duration_ms).secondary, p.to.secondary);
        assert_eq!(p.sample(p.duration_ms).pan, 1.);
        let settled = plan(p.to, 1, 1);
        assert!(!settled.changed());
    }
    #[test]
    fn secondary_delay_is_short_and_does_not_extend_global_duration() {
        let p = plan(initial(), 1, 1);
        assert_eq!(p.sample(30.).secondary[1], 640.);
        assert!(p.sample(30.).pages[1] < 640.);
        assert_eq!(p.sample(260.).secondary[1], 0.);
    }
}
