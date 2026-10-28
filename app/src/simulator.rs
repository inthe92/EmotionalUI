//! Offscreen software platform for Linux containers. No DRM, evdev or display claims.
use crate::{state::State, Home};
use emotional_core::controls::{Controller, Mapping, RawEvent};
use serde::Deserialize;
use slint::{
    platform::{
        software_renderer::{
            MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType, TargetPixel,
        },
        EventLoopProxy, Platform, WindowAdapter,
    },
    ComponentHandle, Rgb8Pixel,
};
use std::{
    cell::RefCell,
    error::Error,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{mpsc, Mutex},
    time::{Duration, Instant},
};

// Diagnostic target matches Slint 1.13.1 linuxkms renderer/sw.rs XRGB packing.
// Copyright SixtyFPS GmbH; GPL-3.0-only (same license as this project).
#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct KmsXrgb(u32);
impl TargetPixel for KmsXrgb {
    #[inline]
    fn blend(&mut self, color: PremultipliedRgbaColor) {
        if color.alpha == 0 {
            return;
        }
        if color.alpha == 255 {
            self.0 = 0xff000000
                | ((color.red as u32) << 16)
                | ((color.green as u32) << 8)
                | color.blue as u32;
            return;
        }
        let mut pixel = PremultipliedRgbaColor {
            red: (self.0 >> 16) as u8,
            green: (self.0 >> 8) as u8,
            blue: self.0 as u8,
            alpha: (self.0 >> 24) as u8,
        };
        pixel.blend(color);
        self.0 = ((pixel.alpha as u32) << 24)
            | ((pixel.red as u32) << 16)
            | ((pixel.green as u32) << 8)
            | pixel.blue as u32;
    }
    fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self(0xff000000 | ((r as u32) << 16) | ((g as u32) << 8) | b as u32)
    }
    fn background() -> Self {
        Self(0)
    }
}

enum Message {
    Quit,
    Invoke(Box<dyn FnOnce() + Send>),
}
struct FrameCapture {
    path: std::path::PathBuf,
    ui: slint::Weak<Home>,
    replay_start: Instant,
    scheduled_ms: u64,
}
thread_local! {
    static FRAME_CAPTURE: RefCell<Option<FrameCapture>> = const { RefCell::new(None) };
    static MOVIE: RefCell<Option<Movie>> = const { RefCell::new(None) };
}
struct Movie { directory: PathBuf, ui: slint::Weak<Home>, frames: usize }
impl Movie {
    fn record(&mut self, rgb: &[u8]) -> std::io::Result<()> {
        if self.frames >= 160 { return Ok(()); }
        let Some(ui)=self.ui.upgrade() else { return Ok(()) };
        let name=format!("{:04}",self.frames);
        std::fs::write(self.directory.join(format!("{name}.rgb")),rgb)?;
        let timestamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
        let metadata=serde_json::json!({"width":640,"height":480,"unix_ms":timestamp,
            "launch_open":ui.get_launch_open(),"progress":ui.get_launch_progress(),
            "interface_opacity":ui.get_launch_ui_opacity(),"card_progress":ui.get_launch_card_progress(),
            "source":"actual Docker/software raster; no physical presentation"});
        std::fs::write(self.directory.join(format!("{name}.json")),metadata.to_string())?;
        self.frames+=1;
        Ok(())
    }
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
        // Opt-in RAM-only control: not a measurement of mmap/DRM or LCD timing.
        let kms_target = crate::performance::enabled()
            && std::env::var("EMOTIONAL_PERF_TARGET").as_deref() == Ok("xrgb");
        let mut pixels = if kms_target {
            Vec::new()
        } else {
            vec![Rgb8Pixel::default(); 640 * 480]
        };
        let mut kms_pixels = if kms_target {
            vec![KmsXrgb::default(); 640 * 480]
        } else {
            Vec::new()
        };
        let verify_shadow =
            kms_target && std::env::var("EMOTIONAL_PERF_VERIFY_SHADOW").as_deref() == Ok("1");
        // Only correctness verification: a full reference repaint and three
        // ordinary RAM copies. Never used in timing runs or in production KMS.
        let mut reference = if verify_shadow {
            vec![KmsXrgb::default(); 640 * 480]
        } else {
            Vec::new()
        };
        let mut ring = if verify_shadow {
            vec![reference.clone(); 3]
        } else {
            Vec::new()
        };
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
            let animated = self.window.has_active_animations();
            if animated {
                self.window.request_redraw();
            }
            let mut verify_failed = false;
            if self.window.draw_if_needed(|renderer| {
                crate::performance::frame(Instant::now(), start.elapsed(), animated);
                let _timing = crate::performance::span("software_raster_ms");
                if kms_target {
                    renderer.render(&mut kms_pixels, 640);
                } else {
                    renderer.render(&mut pixels, 640);
                }
                crate::performance::raster_finished();
                if verify_shadow {
                    renderer.set_repaint_buffer_type(RepaintBufferType::NewBuffer);
                    renderer.render(&mut reference, 640);
                    renderer.set_repaint_buffer_type(RepaintBufferType::ReusedBuffer);
                    let target = &mut ring[frames as usize % 3];
                    target.copy_from_slice(&kms_pixels);
                    verify_failed = *target != reference;
                }
            }) {
                frames += 1;
                let recording=MOVIE.with(|movie|movie.borrow().is_some());
                if recording {
                    let rgb:Vec<u8>=if kms_target {
                        kms_pixels.iter().flat_map(|p|[(p.0>>16) as u8,(p.0>>8) as u8,p.0 as u8]).collect()
                    } else {pixels.iter().flat_map(|p|[p.r,p.g,p.b]).collect()};
                    MOVIE.with(|movie|movie.borrow_mut().as_mut().unwrap().record(&rgb))
                        .map_err(|e|slint::PlatformError::from(e.to_string()))?;
                }
                if let Some(request) = FRAME_CAPTURE.with(|capture| capture.borrow_mut().take()) {
                    // This is the buffer actually rasterized above, not a second
                    // take_snapshot render of a potentially different state.
                    let rgb: Vec<u8> = if kms_target {
                        kms_pixels
                            .iter()
                            .flat_map(|p| [(p.0 >> 16) as u8, (p.0 >> 8) as u8, p.0 as u8])
                            .collect()
                    } else {
                        pixels.iter().flat_map(|p| [p.r, p.g, p.b]).collect()
                    };
                    image::save_buffer(&request.path, &rgb, 640, 480, image::ColorType::Rgb8)
                        .map_err(|e| slint::PlatformError::from(e.to_string()))?;
                    if let Some(ui) = request.ui.upgrade() {
                        use slint::Model;
                        let metadata = format!("scheduled_ms={}\nobserved_ms={}\nsection={}\npage_offsets={:?}\nmodal_kind={}\nmodal_progress={}\nsource=actual_offscreen_raster_buffer\nphysical_presented=false\n",request.scheduled_ms,request.replay_start.elapsed().as_millis(),ui.get_active_section(),ui.get_page_offsets().iter().collect::<Vec<_>>(),ui.get_modal_kind(),ui.get_modal_progress());
                        std::fs::write(request.path.with_extension("frame.txt"), metadata)
                            .map_err(|e| slint::PlatformError::from(e.to_string()))?;
                    }
                }
            }
            if verify_failed {
                return Err(format!("Shadow/reference pixel mismatch at frame {frames}").into());
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
        if verify_shadow {
            println!("SIMULATOR: shadow_full_reference_equal_frames={frames}; triple_ram_copies=true; verification_not_timing=true");
        }
        if frames == 0 {
            return Err("Simulator did not rasterize a frame".into());
        }
        Ok(())
    }
}
pub fn install() -> Result<(), Box<dyn Error>> {
    let (sender, receiver) = mpsc::channel();
    let buffer_type = if crate::performance::enabled()
        && std::env::var("EMOTIONAL_PERF_BUFFER").as_deref() == Ok("new")
    {
        RepaintBufferType::NewBuffer
    } else {
        RepaintBufferType::ReusedBuffer
    };
    let window = MinimalSoftwareWindow::new(buffer_type);
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
    drop_open: Option<bool>,
    active_section: Option<i32>,
    page_at_rest: Option<usize>,
    motion_full_viewport: Option<bool>,
    modal_kind: Option<i32>,
    modal_progress_min: Option<f32>,
    modal_progress_max: Option<f32>,
    distinct_protagonists: Option<bool>,
    menu: Option<bool>,
    library: Option<bool>,
    menu_index: Option<usize>,
    menu_choose: Option<i32>,
    system_choose: Option<i32>,
    settings_title: Option<String>,
    reduced: Option<bool>,
    held: Option<bool>,
    text: Option<String>,
    pressed: Option<bool>,
    mood: Option<i32>,
    atmosphere_contains: Option<String>,
    library_index: Option<usize>,
    library_cursor: Option<usize>,
    library_choose: Option<i32>,
    library_key_choose: Option<i32>,
    library_back: Option<bool>,
    library_search_open: Option<bool>,
    library_heading_contains: Option<String>,
    library_first_title: Option<String>,
    hero_title: Option<String>,
    resume: Option<bool>,
    scene_phase: Option<i32>,
    dark_mode: Option<bool>,
    scene_weather: Option<i32>,
    palette_index: Option<i32>,
    options_open: Option<bool>,
    statistics: Option<bool>,
    personalized: Option<bool>,
    audio_open: Option<bool>,
    audio_mode: Option<String>,
    audio_master: Option<u8>,
    has_recent: Option<bool>,
    resume_count: Option<i32>,
    catalog_count: Option<usize>,
    local_art: Option<bool>,
    artwork_refresh: Option<bool>,
    message_contains: Option<String>,
    game_options: Option<bool>,
    game_option_index: Option<usize>,
    game_reason_contains: Option<String>,
    launch_pending: Option<bool>,
    launch_open: Option<bool>,
    launch_progress_min: Option<f32>,
    launch_progress_max: Option<f32>,
    launch_title: Option<String>,
    artwork_open: Option<bool>,
    artwork_ready: Option<bool>,
    artwork_choose: Option<i32>,
    artwork_command: Option<i32>,
    game_option_choose: Option<i32>,
    pan_min: Option<f32>,
    pan_max: Option<f32>,
    section_title: Option<String>,
    section_min: Option<f32>,
    section_max: Option<f32>,
    capture: Option<std::path::PathBuf>,
    motion_sample_ms: Option<u32>,
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
                .motion_sample_ms
                .is_some_and(|ms| ms > 2000 || step.capture.is_none())
            {
                return Err("Motion sampling requires a capture and bounded elapsed time".into());
            }
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
    if let Some(directory)=std::env::var_os("EMOTIONAL_SIM_RECORD") {
        let folder=PathBuf::from(directory).join(format!("process-{}",std::process::id()));
        std::fs::create_dir_all(&folder)?;
        MOVIE.with(|movie|*movie.borrow_mut()=Some(Movie {directory:folder,ui:ui.as_weak(),frames:0}));
    }
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
            // Invoke the same callbacks used by TouchArea, before borrowing
            // state; they synchronously route through the production controller.
            if let Some(index)=step.menu_choose { ui.invoke_menu_select(index); }
            if let Some(index)=step.system_choose {
                if ui.get_system_open() { ui.invoke_system_select(index); }
            }
            if let Some(index)=step.game_option_choose {ui.invoke_game_option_select(index);}
            if step.artwork_refresh == Some(true) { ui.set_artwork_generation(ui.get_artwork_generation().wrapping_add(1)); }
            if let Some(index)=step.artwork_choose {ui.invoke_artwork_picker_choose(index);}
            if let Some(index)=step.artwork_command {ui.invoke_artwork_picker_command(index);}
            if let Some(index)=step.library_choose { if ui.get_library_open() && index>=0 {ui.invoke_activate(10+index);} }
            if let Some(index)=step.library_key_choose {if ui.get_library_search_open(){ui.invoke_library_search_key(index);} }
            if step.library_back==Some(true) && ui.get_library_open() && !ui.get_library_search_open() && !ui.get_system_open() && !ui.get_audio_open() && !ui.get_game_options_open() {ui.invoke_dismiss();}
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
            if let Some(ms)=step.motion_sample_ms {
                if !crate::motion::sample_for_capture(&ui,ms) {
                    *stored.borrow_mut()=Some(Err("No motion trajectory to capture".into()));
                    let _=slint::quit_event_loop(); return;
                }
            }
            if step.pan_min.is_some() || step.pan_max.is_some() || step.resume.is_some() {
                println!("PAN_SAMPLE scheduled_ms={} elapsed_ms={} position={:.4}", step.at_ms, start.elapsed().as_millis(), ui.get_pan_position());
            }
            if step.section_title.is_some() || step.section_min.is_some() || step.section_max.is_some() || step.capture.is_some() {
                println!("SECTION_SAMPLE scheduled_ms={} elapsed_ms={} position={:.4} primary={:.4} secondary={:.4} target={:.1}", step.at_ms, start.elapsed().as_millis(), ui.get_section_position(), ui.get_discovery_main(), ui.get_discovery_secondary(), ui.get_section_target());
            }
            let checks = [
                step.motion_full_viewport.is_none_or(|expected| {
                    let style=ui.global::<crate::MotionDesign>();
                    let width=ui.window().size().width as f32;
                    ((style.get_content_span()-width).abs()<0.01
                        && (style.get_page_span()-width).abs()<0.01)==expected
                }),
                step.page_at_rest.is_none_or(|page| {
                    use slint::Model;
                    ui.get_motion_section()==page as i32
                        && ui.get_page_offsets().row_data(page)==Some(0.)
                        && ui.get_page_secondary_offsets().row_data(page)==Some(0.)
                        && ui.get_page_offsets().iter().enumerate().all(|(i,x)| i==page || x.abs()>=640.)
                }),
                step.modal_kind.is_none_or(|v|ui.get_modal_kind()==v),
                step.modal_progress_min.is_none_or(|v|ui.get_modal_progress()>=v),
                step.modal_progress_max.is_none_or(|v|ui.get_modal_progress()<=v),
                step.distinct_protagonists.is_none_or(|v|(ui.get_recent_title()!=ui.get_hero_title())==v),
                step.system_open.is_none_or(|v|ui.get_system_open()==v),
                step.system_focused.is_none_or(|v|ui.get_system_focused()==v),
                step.drop_open.is_none_or(|v|ui.get_drop_open()==v),
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
                step.launch_open.is_none_or(|v|ui.get_launch_open()==v),
                step.launch_progress_min.is_none_or(|v|ui.get_launch_progress()>=v),
                step.launch_progress_max.is_none_or(|v|ui.get_launch_progress()<=v),
                step.launch_title.as_ref().is_none_or(|v|ui.get_launch_title().as_str()==v),
                step.artwork_open.is_none_or(|v|ui.get_artwork_picker_open()==v),
                step.artwork_ready.is_none_or(|v|ui.get_artwork_picker_ready()==v),
                step.selected.is_none_or(|v| state.selected == v),
                step.menu.is_none_or(|v| state.menu == v),
                step.library.is_none_or(|v| state.library == v),
                step.menu_index.is_none_or(|v| state.menu_index == v),
                step.settings_title.as_ref().is_none_or(|v|ui.get_settings_title().as_str()==v),
                step.reduced.is_none_or(|v| state.config.reduced_motion == v),
                step.held.is_none_or(|v| controller.held() == v),
                step.mood.is_none_or(|v|ui.get_mood()==v),
                step.atmosphere_contains.as_ref().is_none_or(|v|ui.get_atmosphere().contains(v)),
                step.library_index.is_none_or(|v|state.library_index==v),
                step.library_cursor.is_none_or(|v|state.library_cursor()==v),
                step.library_search_open.is_none_or(|v|ui.get_library_search_open()==v),
                step.library_heading_contains.as_ref().is_none_or(|v|ui.get_library_heading().contains(v)),
                step.library_first_title.as_ref().is_none_or(|v|{
                    use slint::Model;
                    ui.get_library_titles().row_data(0).is_some_and(|s|s.as_str()==v)
                }),
                step.hero_title.as_ref().is_none_or(|v|ui.get_hero_title().as_str()==v),
                step.resume.is_none_or(|v|state.resume==v),
                step.scene_phase.is_none_or(|v|ui.get_scene_phase()==v),
                step.dark_mode.is_none_or(|v|state.privacy.dark_mode==v),
                step.scene_weather.is_none_or(|v|ui.get_scene_weather()==v),
                step.palette_index.is_none_or(|v|ui.global::<crate::DaycycleAssets>().get_palette_index()==v),
                step.options_open.is_none_or(|v|state.options_open==v),
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
                    FRAME_CAPTURE.with(|capture| {
                        if capture.borrow().is_some() { return Err("Unconsumed frame capture".into()); }
                        *capture.borrow_mut() = Some(FrameCapture {path:path.clone(),ui:ui.as_weak(),replay_start:start,scheduled_ms:step.at_ms});
                        ui.window().request_redraw();
                        Ok(())
                    })
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
    fn xrgb_fast_paths_match_reference_for_every_source_and_destination_alpha() {
        for alpha in 0..=255u8 {
            for destination_alpha in 0..=255u8 {
                let source = PremultipliedRgbaColor {
                    red: alpha / 3,
                    green: alpha / 2,
                    blue: alpha,
                    alpha,
                };
                let mut reference = PremultipliedRgbaColor {
                    red: destination_alpha,
                    green: destination_alpha / 2,
                    blue: destination_alpha / 5,
                    alpha: destination_alpha,
                };
                let mut actual = KmsXrgb(
                    ((reference.alpha as u32) << 24)
                        | ((reference.red as u32) << 16)
                        | ((reference.green as u32) << 8)
                        | reference.blue as u32,
                );
                reference.blend(source);
                actual.blend(source);
                assert_eq!(
                    actual.0,
                    ((reference.alpha as u32) << 24)
                        | ((reference.red as u32) << 16)
                        | ((reference.green as u32) << 8)
                        | reference.blue as u32
                );
            }
        }
    }
    #[test]
    fn diagnostic_xrgb_matches_renderer_channel_packing() {
        let mut target = KmsXrgb::from_rgb(71, 122, 190);
        let mut reference = PremultipliedRgbaColor::from_rgb(71, 122, 190);
        for color in [
            PremultipliedRgbaColor {
                red: 25,
                green: 15,
                blue: 10,
                alpha: 80,
            },
            PremultipliedRgbaColor {
                red: 8,
                green: 32,
                blue: 63,
                alpha: 128,
            },
        ] {
            target.blend(color);
            reference.blend(color);
            assert_eq!(
                target.0,
                ((reference.alpha as u32) << 24)
                    | ((reference.red as u32) << 16)
                    | ((reference.green as u32) << 8)
                    | reference.blue as u32
            );
        }
    }
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
