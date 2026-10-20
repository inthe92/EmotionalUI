//! Offscreen software platform for Linux containers. No DRM, evdev or display claims.
use crate::{state::State, Home};
use emotional_core::controls::{Controller, Mapping, RawEvent};
use serde::Deserialize;
use slint::{
    platform::{
        software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
        EventLoopProxy, Platform, WindowAdapter,
    },
    ComponentHandle, Rgb8Pixel,
};
use std::{
    cell::RefCell,
    error::Error,
    path::Path,
    rc::Rc,
    sync::{mpsc, Mutex},
    time::{Duration, Instant},
};

enum Message {
    Quit,
    Invoke(Box<dyn FnOnce() + Send>),
}
struct Proxy(mpsc::Sender<Message>);
impl EventLoopProxy for Proxy {
    fn quit_event_loop(&self) -> Result<(), slint::EventLoopError> {
        self.0
            .send(Message::Quit)
            .map_err(|_| slint::EventLoopError::EventLoopTerminated)
    }
    fn invoke_from_event_loop(
        &self,
        event: Box<dyn FnOnce() + Send>,
    ) -> Result<(), slint::EventLoopError> {
        self.0
            .send(Message::Invoke(event))
            .map_err(|_| slint::EventLoopError::EventLoopTerminated)
    }
}
struct Offscreen {
    window: Rc<MinimalSoftwareWindow>,
    sender: mpsc::Sender<Message>,
    receiver: Mutex<mpsc::Receiver<Message>>,
}
impl Platform for Offscreen {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.window.clone())
    }
    fn new_event_loop_proxy(&self) -> Option<Box<dyn EventLoopProxy>> {
        Some(Box::new(Proxy(self.sender.clone())))
    }
    fn run_event_loop(&self) -> Result<(), slint::PlatformError> {
        let mut pixels = vec![Rgb8Pixel::default(); 640 * 480];
        let start = Instant::now();
        let mut frames = 0u64;
        'event_loop: loop {
            slint::platform::update_timers_and_animations();
            while let Ok(event) = self.receiver.lock().unwrap().try_recv() {
                match event {
                    Message::Quit => break 'event_loop,
                    Message::Invoke(callback) => callback(),
                }
            }
            if self.window.has_active_animations() {
                self.window.request_redraw();
            }
            if self.window.draw_if_needed(|renderer| {
                renderer.render(&mut pixels, 640);
            }) {
                frames += 1;
            }
            // A bound for unattended CI, separate from the application's smoke timer.
            if start.elapsed() > Duration::from_secs(35) {
                return Err("Simulator event-loop deadline exceeded".into());
            }
            let limit = if self.window.has_active_animations() {
                Duration::from_millis(16)
            } else {
                Duration::from_millis(50)
            };
            let wait = slint::platform::duration_until_next_timer_update()
                .unwrap_or(limit)
                .min(limit);
            match self.receiver.lock().unwrap().recv_timeout(wait) {
                Ok(Message::Quit) => break,
                Ok(Message::Invoke(callback)) => callback(),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err("Simulator event channel closed".into())
                }
            }
        }
        println!("SIMULATOR: rasterized_frames={frames}; physical_frames_presented=UNMEASURED");
        if frames == 0 {
            return Err("Simulator did not rasterize a frame".into());
        }
        Ok(())
    }
}
pub fn install() -> Result<(), Box<dyn Error>> {
    let (sender, receiver) = mpsc::channel();
    let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
    window.set_size(slint::PhysicalSize::new(640, 480));
    slint::platform::set_platform(Box::new(Offscreen {
        window,
        sender,
        receiver: Mutex::new(receiver),
    }))?;
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    mapping: Mapping,
    #[serde(default)]
    keyboard: bool,
    steps: Vec<Step>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Step {
    at_ms: u64,
    kind: Option<u16>,
    code: Option<u16>,
    value: Option<i32>,
    #[serde(default)]
    disconnect: bool,
    selected: Option<usize>,
    dock_cursor: Option<i32>,
    system_open: Option<bool>,
    system_focused: Option<bool>,
    active_section: Option<i32>,
    menu: Option<bool>,
    library: Option<bool>,
    menu_index: Option<usize>,
    reduced: Option<bool>,
    held: Option<bool>,
    text: Option<String>,
    pressed: Option<bool>,
    mood: Option<i32>,
    atmosphere_contains: Option<String>,
    library_index: Option<usize>,
    hero_title: Option<String>,
    resume: Option<bool>,
    scene_phase: Option<i32>,
    statistics: Option<bool>,
    personalized: Option<bool>,
    audio_open: Option<bool>,
    audio_mode: Option<String>,
    audio_master: Option<u8>,
    has_recent: Option<bool>,
    resume_count: Option<i32>,
    catalog_count: Option<usize>,
    local_art: Option<bool>,
    message_contains: Option<String>,
    game_options: Option<bool>,
    game_option_index: Option<usize>,
    game_reason_contains: Option<String>,
    launch_pending: Option<bool>,
    pan_min: Option<f32>,
    pan_max: Option<f32>,
    section_title: Option<String>,
    section_min: Option<f32>,
    section_max: Option<f32>,
    capture: Option<std::path::PathBuf>,
}
impl Scenario {
    fn validate(&self) -> Result<(), Box<dyn Error>> {
        self.mapping.validate()?;
        if self.steps.is_empty() || self.steps.len() > 5000 {
            return Err("Expected 1..5000 replay steps".into());
        }
        let mut previous = 0;
        for step in &self.steps {
            let fields = [
                step.kind.is_some(),
                step.code.is_some(),
                step.value.is_some(),
            ];
            if step.at_ms < previous
                || step.at_ms > 25_000
                || (fields.iter().any(|v| *v) && !fields.iter().all(|v| *v))
                || (step.disconnect && step.kind.is_some())
                || (step.text.is_some() != step.pressed.is_some())
                || (step.text.is_some() && (!self.keyboard || step.kind.is_some()))
            {
                return Err("Invalid replay order, duration or event fields".into());
            }
            previous = step.at_ms;
            if step
                .capture
                .as_ref()
                .is_some_and(|p| !p.is_absolute() || p.extension().is_none_or(|e| e != "png"))
            {
                return Err("Replay capture requires an absolute PNG path".into());
            }
        }
        Ok(())
    }
}
pub struct ReplayRun {
    _timer: slint::Timer,
    result: Rc<RefCell<Option<Result<(), String>>>>,
}
impl ReplayRun {
    pub fn finish(&self) -> Result<(), Box<dyn Error>> {
        self.result
            .borrow()
            .clone()
            .ok_or("Simulation did not finish")?
            .map_err(Into::into)
    }
}
pub fn replay(
    path: &Path,
    ui: &Home,
    state: Rc<RefCell<State>>,
) -> Result<ReplayRun, Box<dyn Error>> {
    let scenario: Scenario = toml::from_str(&std::fs::read_to_string(path)?)?;
    scenario.validate()?;
    let mut controller = Controller::new(scenario.mapping)?;
    let result = Rc::new(RefCell::new(None));
    let stored = result.clone();
    let timer = slint::Timer::default();
    let weak = ui.as_weak();
    let start = Instant::now();
    let mut index = 0;
    ui.set_native_controls(!scenario.keyboard);
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(8), move || {
        if stored.borrow().is_some() { return; }
        let Some(ui) = weak.upgrade() else { return; };
        while index < scenario.steps.len() && start.elapsed().as_millis() >= u128::from(scenario.steps[index].at_ms) {
            let step = &scenario.steps[index];
            if let (Some(text),Some(pressed))=(&step.text,step.pressed) {
                let text=match text.as_str() {
                    "up"=>char::from(slint::platform::Key::UpArrow).to_string(),
                    "down"=>char::from(slint::platform::Key::DownArrow).to_string(),
                    "left"=>char::from(slint::platform::Key::LeftArrow).to_string(),
                    "right"=>char::from(slint::platform::Key::RightArrow).to_string(),
                    "enter"=>char::from(slint::platform::Key::Return).to_string(),
                    "escape"=>char::from(slint::platform::Key::Escape).to_string(),
                    _=>text.clone(),
                }.into();
                ui.window().dispatch_event(if pressed {slint::platform::WindowEvent::KeyPressed{text}}else{slint::platform::WindowEvent::KeyReleased{text}});
            }
            let mut state = state.borrow_mut();
            let now = Duration::from_millis(step.at_ms);
            // Replay times are deterministic input timestamps, not measured input latency.
            let mut actions = controller.tick(now);
            if step.disconnect { controller.disconnect(); actions.clear(); }
            if let (Some(kind), Some(code), Some(value)) = (step.kind, step.code, step.value) {
                actions.extend(controller.event(RawEvent { kind, code, value }, now));
            }
            for action in actions {
                if let Err(error) = state.action(&ui, action) {
                    *stored.borrow_mut() = Some(Err(error.to_string()));
                    let _ = slint::quit_event_loop();
                    return;
                }
            }
            state.native_held = controller.held();
            if step.pan_min.is_some() || step.pan_max.is_some() || step.resume.is_some() {
                println!("PAN_SAMPLE scheduled_ms={} elapsed_ms={} position={:.4}", step.at_ms, start.elapsed().as_millis(), ui.get_pan_position());
            }
            if step.section_title.is_some() || step.section_min.is_some() || step.section_max.is_some() || step.capture.is_some() {
                println!("SECTION_SAMPLE scheduled_ms={} elapsed_ms={} position={:.4} primary={:.4} secondary={:.4} target={:.1}", step.at_ms, start.elapsed().as_millis(), ui.get_section_position(), ui.get_discovery_main(), ui.get_discovery_secondary(), ui.get_section_target());
            }
            let checks = [
                step.system_open.is_none_or(|v|ui.get_system_open()==v),
                step.system_focused.is_none_or(|v|ui.get_system_focused()==v),
                step.dock_cursor.is_none_or(|v|ui.get_dock_cursor()==v),
                step.active_section.is_none_or(|v|ui.get_active_section()==v),
                step.catalog_count.is_none_or(|v|state.games.len()==v),
                step.local_art.is_none_or(|v|ui.get_local_art_available()==v),
                step.message_contains.as_ref().is_none_or(|v|ui.get_message().contains(v)),
                step.resume_count.is_none_or(|v|ui.get_resume_count()==v),
                step.game_options.is_none_or(|v|state.game_options.is_some()==v),
                step.game_reason_contains.as_ref().is_none_or(|v|ui.get_game_option_reason().contains(v)),
                step.game_option_index.is_none_or(|v|state.game_option_index==v),
                step.has_recent.is_none_or(|v|ui.get_has_recent()==v),
                step.launch_pending.is_none_or(|v|state.launch.as_ref().is_some_and(|c|c.pending.is_some())==v),
                step.selected.is_none_or(|v| state.selected == v),
                step.menu.is_none_or(|v| state.menu == v),
                step.library.is_none_or(|v| state.library == v),
                step.menu_index.is_none_or(|v| state.menu_index == v),
                step.reduced.is_none_or(|v| state.config.reduced_motion == v),
                step.held.is_none_or(|v| controller.held() == v),
                step.mood.is_none_or(|v|ui.get_mood()==v),
                step.atmosphere_contains.as_ref().is_none_or(|v|ui.get_atmosphere().contains(v)),
                step.library_index.is_none_or(|v|state.library_index==v),
                step.hero_title.as_ref().is_none_or(|v|ui.get_hero_title().as_str()==v),
                step.resume.is_none_or(|v|state.resume==v),
                step.scene_phase.is_none_or(|v|ui.get_scene_phase()==v),
                step.statistics.is_none_or(|v|state.privacy.statistics==v),
                step.personalized.is_none_or(|v|state.privacy.personalized==v),
                step.audio_open.is_none_or(|v|state.audio_panel==v),
                step.audio_mode.as_ref().is_none_or(|v|ui.get_audio_mode().as_str()==v),
                step.audio_master.is_none_or(|v|state.audio_preferences.master==v),
                step.pan_min.is_none_or(|v|ui.get_pan_position()>=v),
                step.pan_max.is_none_or(|v|ui.get_pan_position()<=v),
                step.section_title.as_ref().is_none_or(|v|ui.get_section_title().as_str()==v),
                step.section_min.is_none_or(|v|ui.get_section_position()>=v),
                step.section_max.is_none_or(|v|ui.get_section_position()<=v),
            ];
            if checks.contains(&false) {
                *stored.borrow_mut() = Some(Err(format!("Replay check failed at step {index} ({} ms): {checks:?}", step.at_ms)));
                let _ = slint::quit_event_loop();
                return;
            }
            // Development-only evidence; capture work affects timing, never a FPS probe.
            if let Some(path) = &step.capture {
                let capture = (|| -> Result<(), Box<dyn Error>> {
                    if path.exists() { return Err("Replay capture already exists".into()); }
                    crate::capture::save_ui(path,&ui.window().take_snapshot()?)
                })();
                if let Err(error) = capture {
                    *stored.borrow_mut() = Some(Err(error.to_string()));
                    let _ = slint::quit_event_loop();
                    return;
                }
            }
            index += 1;
            // Snapshot encoding is synchronous evidence work. Yield so other
            // timers/animations run before processing another overdue capture.
            if step.capture.is_some() { break; }
        }
        if index == scenario.steps.len() {
            *stored.borrow_mut() = Some(Ok(()));
            println!("SIMULATOR: {index} raw-input replay steps passed; physical_input_verified=false");
        }
    });
    Ok(ReplayRun {
        _timer: timer,
        result,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_rejects_partial_events_and_unbounded_times() {
        for text in [
            "at_ms=0\nkind=1",
            "at_ms=25001",
            "at_ms=2\n[[steps]]\nat_ms=1",
        ] {
            let s: Scenario = toml::from_str(&format!("[mapping]\n[[steps]]\n{text}")).unwrap();
            assert!(s.validate().is_err());
        }
    }
}
