//! Opt-in, bounded KMS comparison: real actions and motion, never synthetic rendering.
use crate::{state::State, Home};
use emotional_core::controls::Action;
use slint::{ComponentHandle, Model};
use std::{
    cell::{Cell, RefCell},
    error::Error,
    path::PathBuf,
    rc::{Rc, Weak},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    Sample,
    Action(Action),
    Capture(usize),
    Finish,
}

// Fixed input times, not promised presentation times. No Confirm, data editing,
// renderer sampling, or permanent polling is permitted in this comparison.
const PLAN: &[(u64, Event)] = &[
    (0, Event::Sample),
    (10_000, Event::Action(Action::Next)),
    (11_000, Event::Action(Action::Next)),
    (12_000, Event::Action(Action::Next)),
    (13_000, Event::Action(Action::Next)),
    (15_000, Event::Action(Action::Previous)),
    (16_000, Event::Action(Action::Previous)),
    (17_000, Event::Action(Action::Previous)),
    (18_000, Event::Action(Action::Previous)),
    (20_000, Event::Action(Action::Next)),
    (20_080, Event::Action(Action::Previous)),
    (20_160, Event::Action(Action::Next)),
    (22_000, Event::Action(Action::Details)),
    (23_000, Event::Action(Action::Back)),
    (26_000, Event::Sample),
    (55_000, Event::Sample),
    (60_000, Event::Finish),
];
const CAPTURE_PLAN: &[(u64, Event)] = &[
    (5_000, Event::Capture(0)),
    (6_000, Event::Action(Action::Next)),
    (7_000, Event::Capture(1)),
    (8_000, Event::Action(Action::Next)),
    (9_000, Event::Capture(2)),
    (10_000, Event::Action(Action::Next)),
    (11_000, Event::Capture(3)),
    (14_000, Event::Finish),
];

fn validate(mode: Option<&str>, trial: bool, perf: Option<&str>) -> Result<bool, &'static str> {
    match mode {
        None => Ok(false),
        Some("fixed-v1" | "capture-v1") if trial && perf == Some("full") => Ok(true),
        Some("fixed-v1" | "capture-v1") => {
            Err("EMOTIONAL_GPU_REPLAY requires --trial and EMOTIONAL_PERF=full")
        }
        Some(_) => Err("EMOTIONAL_GPU_REPLAY only accepts fixed-v1 or capture-v1"),
    }
}

/// Call after CLI parsing, before constructing a window or touching the library.
pub fn configured(trial: bool) -> Result<bool, Box<dyn Error>> {
    let mode = std::env::var("EMOTIONAL_GPU_REPLAY");
    let mode = match &mode {
        Ok(value) => Some(value.as_str()),
        Err(std::env::VarError::NotPresent) => None,
        Err(_) => return Err("EMOTIONAL_GPU_REPLAY must be valid Unicode".into()),
    };
    let enabled = validate(mode, trial, std::env::var("EMOTIONAL_PERF").ok().as_deref())?;
    if mode == Some("capture-v1") {
        capture_backend(std::env::var("SLINT_BACKEND").ok().as_deref())?;
    }
    Ok(enabled)
}

fn capture_backend(backend: Option<&str>) -> Result<&'static str, &'static str> {
    match backend {
        Some("linuxkms-software") => Ok("software"),
        Some("linuxkms-femtovg") => Ok("femtovg"),
        _ => Err("capture-v1 requires SLINT_BACKEND=linuxkms-software or linuxkms-femtovg"),
    }
}

struct Captures {
    directory: PathBuf,
    pending: RefCell<Option<usize>>,
    saved: Cell<usize>,
    notifier: bool,
}
impl Captures {
    fn save(&self, ui: &Home, index: usize) -> Result<(), Box<dyn Error>> {
        if ui.get_active_section() != index as i32 {
            return Err("Capture section differs from the fixed capture plan".into());
        }
        let path = self.directory.join(format!("section-{index}.png"));
        if path.exists() {
            return Err("Refusing to replace an existing diagnostic capture".into());
        }
        let pixels = ui.window().take_snapshot()?;
        crate::capture::save_ui(&path, &pixels)?;
        self.saved.set(self.saved.get() + 1);
        println!("GPU_CAPTURE saved={} section={index} source=renderer_snapshot physical_presentation=UNMEASURED performance_sample=false", path.display());
        Ok(())
    }
}

pub struct Run {
    generation: Rc<Cell<u64>>,
    _captures: Option<Rc<Captures>>,
}
impl Drop for Run {
    fn drop(&mut self) {
        self.generation.set(self.generation.get().wrapping_add(1));
    }
}

pub fn start(ui: &Home, state: &Rc<RefCell<State>>) -> Result<Run, Box<dyn Error>> {
    if !state.borrow().trial_mode || !crate::performance::enabled() {
        return Err("Diagnostic replay requires an active trial performance session".into());
    }
    if ui.get_active_section() != 0 || ui.get_modal_kind() != 0 || state.borrow().layout != 4 {
        return Err(
            "Fixed replay must start in Continuar with no modal, using the four-section UI".into(),
        );
    }
    let generation = Rc::new(Cell::new(1));
    let captures = if std::env::var("EMOTIONAL_GPU_REPLAY").as_deref() == Ok("capture-v1") {
        let backend = capture_backend(std::env::var("SLINT_BACKEND").ok().as_deref())?;
        let directory = std::env::current_dir()?.join(format!("gpu-captures-{backend}"));
        std::fs::create_dir(&directory)?; // Must be new: never overwrite an earlier run.
        let captures = Rc::new(Captures {
            directory,
            pending: RefCell::new(None),
            saved: Cell::new(0),
            notifier: backend == "femtovg",
        });
        if captures.notifier {
            let weak_capture = Rc::downgrade(&captures);
            let weak_ui = ui.as_weak();
            let weak_token = Rc::downgrade(&generation);
            ui.window().set_rendering_notifier(move |phase, _| {
                if !matches!(phase, slint::RenderingState::AfterRendering) {
                    return;
                }
                let Some(token) = weak_token.upgrade() else {
                    return;
                };
                if token.get() != 1 {
                    return;
                }
                let (Some(captures), Some(ui)) = (weak_capture.upgrade(), weak_ui.upgrade()) else {
                    return;
                };
                let pending = captures.pending.borrow_mut().take();
                if let Some(index) = pending {
                    if let Err(error) = captures.save(&ui, index) {
                        eprintln!("GPU_CAPTURE failed: {error}");
                        let _ = slint::quit_event_loop();
                    }
                }
            })?;
        }
        println!("GPU_REPLAY plan=capture-v1 duration_ms=14000 actions=3; no_manual_input=true; performance_sample=false");
        Some(captures)
    } else {
        println!("GPU_REPLAY plan=fixed-v1 duration_ms=60000 actions=13 epoch_kernel_us={}; no_manual_input=true; presentation_times=BACKEND_ONLY", monotonic_us());
        None
    };
    schedule(
        ui.as_weak(),
        Rc::downgrade(state),
        Rc::downgrade(&generation),
        1,
        Instant::now(),
        0,
        captures.as_ref().map(Rc::downgrade),
    );
    Ok(Run {
        generation,
        _captures: captures,
    })
}

// Linux CLOCK_MONOTONIC shares the DRM event timestamp domain. Diagnostic only.
// Do not use wall time, /proc/uptime (BOOTTIME), or timer ticks as presentation.
fn monotonic_us() -> u64 {
    #[cfg(target_os = "linux")]
    {
        #[repr(C)]
        struct Timespec {
            seconds: i64,
            nanos: i64,
        }
        unsafe extern "C" {
            fn clock_gettime(clock: i32, value: *mut Timespec) -> i32;
        }
        let mut value = Timespec {
            seconds: 0,
            nanos: 0,
        };
        if unsafe { clock_gettime(1, &mut value) } == 0 {
            return (value.seconds as u64).saturating_mul(1_000_000) + value.nanos as u64 / 1000;
        }
    }
    0 // Host simulation has no DRM timestamp domain.
}

pub fn motion_completed(generation: i32, requested_ms: f32, observed_ms: f32) {
    if std::env::var("EMOTIONAL_GPU_REPLAY").as_deref() == Ok("fixed-v1") {
        println!("GPU_MOTION_COMPLETE generation={generation} kernel_us={} requested_ms={requested_ms:.3} observed_ms={observed_ms:.3} physical_presentation=BACKEND_ONLY",monotonic_us());
    }
}

fn schedule(
    ui: slint::Weak<Home>,
    state: Weak<RefCell<State>>,
    generation: Weak<Cell<u64>>,
    expected: u64,
    start: Instant,
    index: usize,
    captures: Option<Weak<Captures>>,
) {
    let plan = if captures.is_some() {
        CAPTURE_PLAN
    } else {
        PLAN
    };
    let Some(&(scheduled_ms, event)) = plan.get(index) else {
        return;
    };
    let delay = Duration::from_millis(scheduled_ms).saturating_sub(start.elapsed());
    slint::Timer::single_shot(delay, move || {
        let Some(token) = generation.upgrade() else {
            return;
        };
        if token.get() != expected {
            return;
        }
        let (Some(ui), Some(state)) = (ui.upgrade(), state.upgrade()) else {
            return;
        };
        let observed_ms = start.elapsed().as_millis();
        if let Event::Action(action) = event {
            if let Err(error) = state.borrow_mut().action(&ui, action) {
                eprintln!(
                    "GPU_REPLAY failed action={} scheduled_ms={scheduled_ms}: {error}",
                    action.id()
                );
                token.set(token.get().wrapping_add(1));
                let _ = slint::quit_event_loop();
                return;
            }
        }
        if let Event::Capture(index) = event {
            let Some(captures) = captures.as_ref().and_then(Weak::upgrade) else {
                return;
            };
            if captures.notifier {
                if captures.pending.borrow_mut().replace(index).is_some() {
                    eprintln!("GPU_CAPTURE failed: preceding capture has not completed");
                    let _ = slint::quit_event_loop();
                    return;
                }
                ui.window().request_redraw();
            } else if let Err(error) = captures.save(&ui, index) {
                eprintln!("GPU_CAPTURE failed: {error}");
                let _ = slint::quit_event_loop();
                return;
            }
        }
        println!("GPU_REPLAY event={event:?} scheduled_ms={scheduled_ms} observed_ms={observed_ms} section={} modal={} pan={:.4} motion_generation={} page_offsets={:?}",
            ui.get_active_section(), ui.get_modal_kind(), ui.get_pan_position(), ui.get_motion_generation(),
            ui.get_page_offsets().iter().collect::<Vec<_>>());
        if event != Event::Finish {
            schedule(
                ui.as_weak(),
                Rc::downgrade(&state),
                generation,
                expected,
                start,
                index + 1,
                captures,
            );
        } else if let Some(captures) = captures.as_ref().and_then(Weak::upgrade) {
            println!(
                "GPU_CAPTURE finished saved={} expected=4 performance_sample=false",
                captures.saved.get()
            );
            if captures.saved.get() != 4 {
                eprintln!("GPU_CAPTURE failed: incomplete capture set");
            }
            let _ = ui.hide();
            let _ = slint::quit_event_loop();
        }
        // The externally supervised trial deadline owns shutdown and recovery.
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_requires_explicit_trial_and_full_diagnostics() {
        for trial in [false, true] {
            for perf in [None, Some("full"), Some("static"), Some("simple")] {
                assert_eq!(validate(None, trial, perf), Ok(false));
                assert_eq!(
                    validate(Some("fixed-v1"), trial, perf).is_ok(),
                    trial && perf == Some("full")
                );
                assert_eq!(
                    validate(Some("capture-v1"), trial, perf).is_ok(),
                    trial && perf == Some("full")
                );
                for invalid in ["", "fixed-v2", "path.toml"] {
                    assert!(validate(Some(invalid), trial, perf).is_err());
                }
            }
        }
    }
    #[test]
    fn capture_plan_is_separate_settled_and_visits_four_sections() {
        assert_eq!(CAPTURE_PLAN.last(), Some(&(14_000, Event::Finish)));
        assert!(CAPTURE_PLAN.windows(2).all(|pair| pair[0].0 < pair[1].0));
        let mut section = 0;
        let mut last_action = 0;
        let mut captured = Vec::new();
        for &(at, event) in CAPTURE_PLAN {
            match event {
                Event::Action(Action::Next) => {
                    section += 1;
                    last_action = at;
                }
                Event::Capture(expected) => {
                    assert_eq!(section, expected);
                    assert!(at - last_action >= 1_000);
                    captured.push((at, expected));
                }
                Event::Finish => {}
                _ => panic!("Unexpected action or sample in capture-only plan"),
            }
        }
        assert_eq!(captured, [(5_000, 0), (7_000, 1), (9_000, 2), (11_000, 3)]);
        assert_eq!(capture_backend(Some("linuxkms-software")), Ok("software"));
        assert_eq!(capture_backend(Some("linuxkms-femtovg")), Ok("femtovg"));
        for invalid in [
            None,
            Some(""),
            Some("linuxkms"),
            Some("winit"),
            Some("../private"),
        ] {
            assert!(capture_backend(invalid).is_err());
        }
    }
    #[test]
    fn fixed_plan_is_bounded_read_only_and_visits_both_directions() {
        assert_eq!(PLAN.first(), Some(&(0, Event::Sample)));
        assert_eq!(PLAN.last(), Some(&(60_000, Event::Finish)));
        assert!(PLAN.windows(2).all(|pair| pair[0].0 < pair[1].0));
        let mut section = 0;
        let mut actions = 0;
        let mut sections = Vec::new();
        for &(at, event) in PLAN {
            if let Event::Action(action) = event {
                actions += 1;
                assert!((10_000..=23_000).contains(&at));
                match action {
                    Action::Next => {
                        section = (section + 1) % 4;
                        sections.push(section);
                    }
                    Action::Previous => {
                        section = (section + 3) % 4;
                        sections.push(section);
                    }
                    Action::Details | Action::Back => {}
                    _ => panic!("Mutating or unexpected action in comparison"),
                }
            }
        }
        assert_eq!(actions, 13);
        assert_eq!(sections, [1, 2, 3, 0, 3, 2, 1, 0, 1, 0, 1]);
        assert_eq!(
            PLAN.iter()
                .filter(|(_, event)| *event == Event::Sample)
                .map(|(at, _)| *at)
                .collect::<Vec<_>>(),
            [0, 26_000, 55_000]
        );
    }
}
